use crate::{error, AppState, RoutingPolicy, SharedClient, UrlType};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, Response, StatusCode, Uri},
};
use base64::{engine::general_purpose, Engine as _};
use endpoints::images::{sd_webui::Txt2ImgRequest, ImageObject};
use hyper::{body::to_bytes, Method};
use std::{fs::File, io::Read};

pub(crate) async fn image_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    info!(target: "stdout", "handling image request");

    let image_url = match state.image_urls.read().await.next().await {
        Ok(url) => url,
        Err(e) => {
            let err_msg = e.to_string();
            info!(target: "stdout", "{}", &err_msg);
            return Ok(error::internal_server_error(&err_msg));
        }
    };

    proxy_request(state.client, req, image_url).await
}

pub(crate) async fn proxy_request(
    client: SharedClient,
    mut req: Request<Body>,
    downstream_url: Uri,
) -> Result<Response<Body>, StatusCode> {
    if req.method().eq(&hyper::http::Method::OPTIONS) {
        let result = Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .header("Access-Control-Allow-Headers", "*")
            .header("Content-Type", "application/json")
            .body(Body::empty());

        match result {
            Ok(response) => return Ok(response),
            Err(e) => {
                let err_msg = e.to_string();

                // log
                error!(target: "stdout", "{}", &err_msg);

                return Ok(error::internal_server_error(&err_msg));
            }
        }
    }

    // Change the request URL to the downstream server
    let endpoint = req
        .uri()
        .path_and_query()
        .map(|x| x.to_string())
        .unwrap_or_default();
    info!(target: "stdout", "endpoint: {}", endpoint);

    if &endpoint == "/v1/images/generations" {
        let image_request = if req.method() == Method::POST {
            info!(target: "stdout", "Prepare the image generation request.");

            // parse request
            let body_bytes = match to_bytes(req.body_mut()).await {
                Ok(body_bytes) => body_bytes,
                Err(e) => {
                    let err_msg = format!("Fail to read buffer from request body. {}", e);

                    // log
                    error!(target: "stdout", "{}", &err_msg);

                    return Ok(error::internal_server_error(err_msg));
                }
            };
            let image_request: Txt2ImgRequest = match serde_json::from_slice(&body_bytes) {
                Ok(image_request) => image_request,
                Err(e) => {
                    let err_msg =
                        format!("Fail to deserialize image create request: {msg}", msg = e);

                    // log
                    error!(target: "stdout", "{}", &err_msg);

                    return Ok(error::bad_request(err_msg));
                }
            };

            image_request
        } else {
            let err_msg = "Invalid HTTP Method.";

            // log
            error!(target: "stdout", "{}", &err_msg);

            return Ok(error::internal_server_error(err_msg));
        };

        let body = serde_json::to_string(&image_request).unwrap();

        let mut server_socket_addr = downstream_url.to_string();
        server_socket_addr = server_socket_addr.trim_end_matches('/').to_string();

        let downstream_uri: Uri = format!("{}/sdapi/v1/txt2img", server_socket_addr)
            .parse()
            .unwrap();
        info!(target: "stdout", "dispatch the request to {}", downstream_uri);

        // create a request to the downstream server
        let downstream_request = Request::builder()
            .method("POST")
            .uri(downstream_uri)
            .body(Body::from(body))
            .unwrap();

        // Forward the request to the downstream server
        match client.request(downstream_request).await {
            Ok(mut response) => match response.status() {
                StatusCode::OK => {
                    let response_body = hyper::body::to_bytes(response.body_mut()).await.unwrap();
                    let deserialized_response: serde_json::Value =
                        serde_json::from_slice(&response_body).unwrap();

                    let mut image_objects: Vec<ImageObject> = vec![];
                    if let Some(images) = deserialized_response.get("images") {
                        let image_vec = images.as_array().unwrap();
                        info!(target: "stdout", "number of images: {}", image_vec.len());

                        for image in image_vec {
                            if let serde_json::Value::String(b64) = image {
                                image_objects.push(ImageObject {
                                    b64_json: Some(b64.clone()),
                                    url: None,
                                    prompt: Some(image_request.prompt.clone()),
                                })
                            }
                        }
                    }

                    let response_body = serde_json::to_string(&image_objects).unwrap();
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(Body::from(response_body))
                        .unwrap();

                    Ok(response)
                }
                _ => {
                    warn!(target: "stdout", "status is not ok");

                    Ok(response)
                }
            },
            Err(e) => {
                let err_msg = format!(
                    "failed to forward the request to the downstream server: {}",
                    e
                );

                error!(target: "stdout", "{}", &err_msg);

                Ok(error::internal_server_error(&err_msg))
            }
        }
    } else {
        let err_msg = format!(
            "404 The requested service endpoint is not found: {}",
            endpoint
        );

        error!(target: "stdout", "{}", &err_msg);

        Ok(error::internal_server_error(&err_msg))
    }
}

pub(crate) async fn add_url_handler(
    State(state): State<AppState>,
    Path(url_type): Path<String>,
    body: String,
) -> Result<Response<Body>, StatusCode> {
    info!(target: "stdout", "url_type: {}", url_type);
    info!(target: "stdout", "body: {}", &body);

    let url_type = match url_type.as_str() {
        "image" => UrlType::Image,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let url: Uri = match body.parse() {
        Ok(url) => url,
        Err(_) => {
            let err_msg = format!("invalid url: {}", &body);

            error!(target: "stdout", "{}", &err_msg);

            return Ok(error::internal_server_error(&err_msg));
        }
    };
    if let Err(e) = state.add_url(url_type, &url).await {
        let err_msg = e.to_string();

        info!(target: "stdout", "{}", &err_msg);

        return Ok(error::internal_server_error(&err_msg));
    }

    // create a response with status code 200. Content-Type is JSON
    let json_body = serde_json::json!({
        "message": "URL registered successfully",
        "url": url.to_string()
    });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json_body.to_string()))
        .unwrap();

    Ok(response)
}

pub(crate) async fn remove_url_handler(
    State(state): State<AppState>,
    Path(url_type): Path<String>,
    body: String,
) -> Result<Response<Body>, StatusCode> {
    info!(target: "stdout", "In remove_url_handler");

    let url_type = match url_type.as_str() {
        "image" => UrlType::Image,
        _ => {
            let err_msg = format!("invalid url type: {}", url_type);
            error!(target: "stdout", "{}", &err_msg);
            return Ok(error::internal_server_error(&err_msg));
        }
    };

    let url: Uri = match body.parse() {
        Ok(url) => url,
        Err(_) => {
            let err_msg = format!("invalid url: {}", &body);

            error!(target: "stdout", "{}", &err_msg);

            return Ok(error::internal_server_error(&err_msg));
        }
    };
    if let Err(e) = state.remove_url(url_type, &url).await {
        let err_msg = e.to_string();

        error!(target: "stdout", "{}", &err_msg);

        return Ok(error::internal_server_error(&err_msg));
    }

    info!(target: "stdout", "unregistered {}", url);

    // create a response with status code 200. Content-Type is JSON
    let json_body = serde_json::json!({
        "message": "URL unregistered successfully",
        "url": url.to_string()
    });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json_body.to_string()))
        .unwrap();

    Ok(response)
}

pub(crate) async fn list_downstream_servers_handler(
    State(state): State<AppState>,
) -> Result<Response<Body>, StatusCode> {
    let servers = state.list_downstream_servers().await;

    // create a response with status code 200. Content-Type is JSON
    let json_body = serde_json::json!({
        "image": servers.get("image").unwrap(),
    });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(json_body.to_string()))
        .unwrap();

    Ok(response)
}

// convert an image file to a base64 string
fn _image_to_base64(image_path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
    // Open the file
    let mut image_file = File::open(image_path)?;

    // Read the file into a byte array
    let mut buffer = Vec::new();
    image_file.read_to_end(&mut buffer)?;

    Ok(general_purpose::STANDARD.encode(&buffer))
}
