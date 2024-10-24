use crate::{error, AppState, RoutingPolicy, SharedClient, UrlType};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, Response, StatusCode, Uri},
};

use base64::{engine::general_purpose, Engine as _};
use endpoints::{
    files::FileObject,
    images::{ImageCreateRequest, ImageEditRequest, ImageObject, ResponseFormat},
};
use hyper::{body::to_bytes, Method};
use multipart::server::{Multipart, ReadEntry, ReadEntryResult};
use multipart_2021 as multipart;
use serde_json::json;
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    time::SystemTime,
};

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

    let content_type = req
        .headers()
        .get("CONTENT_TYPE")
        .and_then(|ct| ct.to_str().ok());

    let (data, downstream_endpoint) = if &endpoint == "/v1/images/generations" {
        let image_request = match content_type {
            Some(content_type) if content_type.starts_with("multipart/") => {
                let boundary = "boundary=";

                let boundary = req.headers().get("content-type").and_then(|ct| {
                    let ct = ct.to_str().ok()?;
                    let idx = ct.find(boundary)?;
                    Some(ct[idx + boundary.len()..].to_string())
                });

                let body_bytes = match to_bytes(req.body_mut()).await {
                    Ok(body_bytes) => body_bytes,
                    Err(e) => {
                        let err_msg = format!("Fail to read buffer from request body. {}", e);

                        // log
                        error!(target: "stdout", "{}", &err_msg);

                        return Ok(error::internal_server_error(err_msg));
                    }
                };

                let cursor = Cursor::new(body_bytes.to_vec());

                let mut multipart = Multipart::with_body(cursor, boundary.unwrap());

                let mut image_request = ImageCreateRequest::default();
                while let ReadEntryResult::Entry(mut field) = multipart.read_entry_mut() {
                    match &*field.headers.name {
                        "prompt" => match field.is_text() {
                            true => {
                                let mut prompt = String::new();

                                if let Err(e) = field.data.read_to_string(&mut prompt) {
                                    let err_msg = format!("Failed to read the prompt. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                image_request.prompt = prompt;
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the prompt. The prompt field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "negative_prompt" => match field.is_text() {
                            true => {
                                let mut negative_prompt = String::new();

                                if let Err(e) = field.data.read_to_string(&mut negative_prompt) {
                                    let err_msg = format!("Failed to read the prompt. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                image_request.prompt = negative_prompt;
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the negative prompt. The negative prompt field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "model" => match field.is_text() {
                            true => {
                                let mut model = String::new();

                                if let Err(e) = field.data.read_to_string(&mut model) {
                                    let err_msg = format!("Failed to read the model. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                image_request.model = model;
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the model name. The model field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "n" => match field.is_text() {
                            true => {
                                let mut n = String::new();

                                if let Err(e) = field.data.read_to_string(&mut n) {
                                    let err_msg =
                                        format!("Failed to read the number of images. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match n.parse::<u64>() {
                                    Ok(n) => image_request.n = Some(n),
                                    Err(e) => {
                                        let err_msg = format!(
                                            "Failed to parse the number of images. Reason: {}",
                                            e
                                        );

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                "Failed to get the number of images. The n field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "size" => {
                            match field.is_text() {
                                true => {
                                    let mut size = String::new();

                                    if let Err(e) = field.data.read_to_string(&mut size) {
                                        let err_msg = format!("Failed to read the size. {}", e);

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::internal_server_error(err_msg));
                                    }

                                    // image_request.size = Some(size);

                                    let parts: Vec<&str> = size.split('x').collect();
                                    if parts.len() != 2 {
                                        let err_msg = "Invalid size format. The correct format is `HeightxWidth`. Example: 256x256";

                                        // log
                                        error!(target: "stdout", "{}", err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                    image_request.height = Some(parts[0].parse().unwrap());
                                    image_request.width = Some(parts[1].parse().unwrap());
                                }
                                false => {
                                    let err_msg =
                                    "Failed to get the size. The size field in the request should be a text field.";

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            }
                        }
                        "response_format" => match field.is_text() {
                            true => {
                                let mut response_format = String::new();

                                if let Err(e) = field.data.read_to_string(&mut response_format) {
                                    let err_msg =
                                        format!("Failed to read the response format. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match response_format.parse::<ResponseFormat>() {
                                    Ok(format) => image_request.response_format = Some(format),
                                    Err(e) => {
                                        let err_msg = format!(
                                            "Failed to parse the response format. Reason: {}",
                                            e
                                        );

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the response format. The response format field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "user" => match field.is_text() {
                            true => {
                                let mut user = String::new();

                                if let Err(e) = field.data.read_to_string(&mut user) {
                                    let err_msg = format!("Failed to read the user. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                image_request.user = Some(user);
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the user. The user field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "cfg_scale" => match field.is_text() {
                            true => {
                                let mut cfg_scale = String::new();

                                if let Err(e) = field.data.read_to_string(&mut cfg_scale) {
                                    let err_msg = format!("Failed to read the cfg_config. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match cfg_scale.parse::<f32>() {
                                    Ok(scale) => image_request.cfg_scale = Some(scale),
                                    Err(e) => {
                                        let err_msg = format!(
                                            "Failed to parse the number of images. Reason: {}",
                                            e
                                        );

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the cfg_config. The cfg_config field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "sample_method" => match field.is_text() {
                            true => {
                                let mut sample_method = String::new();

                                if let Err(e) = field.data.read_to_string(&mut sample_method) {
                                    let err_msg =
                                        format!("Failed to read the sample_method. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                image_request.sample_method = Some(sample_method.as_str().into());
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the sample_method. The sample_method field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "steps" => match field.is_text() {
                            true => {
                                let mut steps = String::new();

                                if let Err(e) = field.data.read_to_string(&mut steps) {
                                    let err_msg = format!("Failed to read the steps. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match steps.parse::<usize>() {
                                    Ok(steps) => image_request.steps = Some(steps),
                                    Err(e) => {
                                        let err_msg =
                                            format!("Failed to parse the steps. Reason: {}", e);

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the steps. The steps field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "height" => match field.is_text() {
                            true => {
                                let mut height = String::new();

                                if let Err(e) = field.data.read_to_string(&mut height) {
                                    let err_msg = format!("Failed to read the height. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match height.parse::<usize>() {
                                    Ok(height) => image_request.height = Some(height),
                                    Err(e) => {
                                        let err_msg =
                                            format!("Failed to parse the height. Reason: {}", e);

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the height. The height field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "width" => match field.is_text() {
                            true => {
                                let mut width = String::new();

                                if let Err(e) = field.data.read_to_string(&mut width) {
                                    let err_msg = format!("Failed to read the width. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match width.parse::<usize>() {
                                    Ok(width) => image_request.width = Some(width),
                                    Err(e) => {
                                        let err_msg =
                                            format!("Failed to parse the width. Reason: {}", e);

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the width. The width field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "control_strength" => match field.is_text() {
                            true => {
                                let mut control_strength = String::new();

                                if let Err(e) = field.data.read_to_string(&mut control_strength) {
                                    let err_msg =
                                        format!("Failed to read the control_strength. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match control_strength.parse::<f32>() {
                                    Ok(control_strength) => {
                                        image_request.control_strength = Some(control_strength)
                                    }
                                    Err(e) => {
                                        let err_msg = format!(
                                            "Failed to parse the control_strength. Reason: {}",
                                            e
                                        );

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the control_strength. The control_strength field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "control_image" => {
                            let filename = match field.headers.filename {
                                Some(filename) => filename,
                                None => {
                                    let err_msg =
                                        "Failed to upload the image file. The filename is not provided.";

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            };

                            // get the image data
                            let mut buffer = Vec::new();
                            let size_in_bytes = match field.data.read_to_end(&mut buffer) {
                                Ok(size_in_bytes) => size_in_bytes,
                                Err(e) => {
                                    let err_msg = format!("Failed to read the image file. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            };

                            // create a file id for the image file
                            let id = format!("file_{}", uuid::Uuid::new_v4());

                            // save the file
                            let path = std::path::Path::new("archives");
                            if !path.exists() {
                                fs::create_dir(path).unwrap();
                            }
                            let file_path = path.join(&id);
                            if !file_path.exists() {
                                fs::create_dir(&file_path).unwrap();
                            }
                            let mut file = match File::create(file_path.join(&filename)) {
                                Ok(file) => file,
                                Err(e) => {
                                    let err_msg = format!(
                                        "Failed to create archive document {}. {}",
                                        &filename, e
                                    );

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            };
                            file.write_all(&buffer[..]).unwrap();

                            // log
                            info!(target: "stdout", "file_id: {}, file_name: {}, size in bytes: {}", &id, &filename, size_in_bytes);

                            let created_at =
                                match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                                    Ok(n) => n.as_secs(),
                                    Err(_) => {
                                        let err_msg = "Failed to get the current time.";

                                        // log
                                        error!(target: "stdout", "{}", err_msg);

                                        return Ok(error::internal_server_error(err_msg));
                                    }
                                };

                            // create a file object
                            image_request.control_image = Some(FileObject {
                                id,
                                bytes: size_in_bytes as u64,
                                created_at,
                                filename,
                                object: "file".to_string(),
                                purpose: "assistants".to_string(),
                            });
                        }
                        "seed" => match field.is_text() {
                            true => {
                                let mut seed = String::new();

                                if let Err(e) = field.data.read_to_string(&mut seed) {
                                    let err_msg = format!("Failed to read the seed. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                match seed.parse::<i32>() {
                                    Ok(seed) => image_request.seed = Some(seed),
                                    Err(e) => {
                                        let err_msg =
                                            format!("Failed to parse the seed. Reason: {}", e);

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the seed. The seed field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "strength" => match field.is_text() {
                            true => {
                                let mut strength = String::new();

                                if let Err(e) = field.data.read_to_string(&mut strength) {
                                    let err_msg = format!("Failed to read the strength. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);
                                }

                                match strength.parse::<f32>() {
                                    Ok(strength) => image_request.strength = Some(strength),
                                    Err(e) => {
                                        let err_msg =
                                            format!("Failed to parse the strength. Reason: {}", e);

                                        // log
                                        error!(target: "stdout", "{}", &err_msg);

                                        return Ok(error::bad_request(err_msg));
                                    }
                                }
                            }
                            false => {
                                let err_msg = "Failed to get the strength. The strength field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        "scheduler" => match field.is_text() {
                            true => {
                                let mut scheduler = String::new();

                                if let Err(e) = field.data.read_to_string(&mut scheduler) {
                                    let err_msg = format!("Failed to read the scheduler. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                image_request.scheduler = Some(scheduler.as_str().into());
                            }
                            false => {
                                let err_msg =
                                    "Failed to get the scheduler. The scheduler field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        },
                        unsupported_field => {
                            let err_msg = format!("Unsupported field: {}", unsupported_field);

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::bad_request(err_msg));
                        }
                    }
                }

                image_request
            }
            _ => {
                if req.method() == Method::POST {
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
                    let image_request: ImageCreateRequest =
                        match serde_json::from_slice(&body_bytes) {
                            Ok(image_request) => image_request,
                            Err(e) => {
                                let err_msg = format!(
                                    "Fail to deserialize image create request: {msg}",
                                    msg = e
                                );

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
                }
            }
        };

        let data = json!({
            "batch_size": 1,
            "cfg_scale": image_request.cfg_scale.unwrap(),
            "denoising_strength": image_request.strength.unwrap(),
            "height": image_request.height.unwrap(),
            "n_iter": 1,
            "negative_prompt": image_request.negative_prompt.unwrap(),
            "prompt": image_request.prompt,
            "sampler_index": image_request.sample_method.unwrap().to_string(),
            "scheduler": image_request.scheduler.unwrap(),
            "seed": image_request.seed.unwrap(),
            "steps": image_request.steps.unwrap(),
            "width": image_request.width.unwrap(),
        });

        (data, "/sdapi/v1/txt2img")
    } else if &endpoint == "/v1/images/edits" {
        let image_request = if req.method() == Method::POST {
            let boundary = "boundary=";

            let boundary = req.headers().get("content-type").and_then(|ct| {
                let ct = ct.to_str().ok()?;
                let idx = ct.find(boundary)?;
                Some(ct[idx + boundary.len()..].to_string())
            });

            let body_bytes = match to_bytes(req.body_mut()).await {
                Ok(body_bytes) => body_bytes,
                Err(e) => {
                    let err_msg = format!("Fail to read buffer from request body. {}", e);

                    // log
                    error!(target: "stdout", "{}", &err_msg);

                    return Ok(error::internal_server_error(err_msg));
                }
            };

            let cursor = Cursor::new(body_bytes.to_vec());

            let mut multipart = Multipart::with_body(cursor, boundary.unwrap());

            let mut image_request = ImageEditRequest::default();
            while let ReadEntryResult::Entry(mut field) = multipart.read_entry_mut() {
                match &*field.headers.name {
                    "image" => {
                        let filename = match field.headers.filename {
                            Some(filename) => filename,
                            None => {
                                let err_msg =
                                    "Failed to upload the image file. The filename is not provided.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };

                        // get the image data
                        let mut buffer = Vec::new();
                        let size_in_bytes = match field.data.read_to_end(&mut buffer) {
                            Ok(size_in_bytes) => size_in_bytes,
                            Err(e) => {
                                let err_msg = format!("Failed to read the image file. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };

                        // create a file id for the image file
                        let id = format!("file_{}", uuid::Uuid::new_v4());

                        // save the file
                        let path = std::path::Path::new("archives");
                        if !path.exists() {
                            fs::create_dir(path).unwrap();
                        }
                        let file_path = path.join(&id);
                        if !file_path.exists() {
                            fs::create_dir(&file_path).unwrap();
                        }
                        let image_file = file_path.join(&filename);
                        let mut file = match File::create(image_file) {
                            Ok(file) => file,
                            Err(e) => {
                                let err_msg = format!(
                                    "Failed to create archive document {}. {}",
                                    &filename, e
                                );

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };
                        file.write_all(&buffer[..]).unwrap();

                        // log
                        info!(target: "stdout", "file_id: {}, file_name: {}, size in bytes: {}", &id, &filename, size_in_bytes);

                        let created_at =
                            match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                                Ok(n) => n.as_secs(),
                                Err(_) => {
                                    let err_msg = "Failed to get the current time.";

                                    // log
                                    error!(target: "stdout", "{}", err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            };

                        // create a file object
                        image_request.image = FileObject {
                            id,
                            bytes: size_in_bytes as u64,
                            created_at,
                            filename,
                            object: "file".to_string(),
                            purpose: "assistants".to_string(),
                        };
                    }
                    "prompt" => match field.is_text() {
                        true => {
                            let mut prompt = String::new();

                            if let Err(e) = field.data.read_to_string(&mut prompt) {
                                let err_msg = format!("Failed to read the prompt. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            image_request.prompt = prompt;
                        }
                        false => {
                            let err_msg =
                                "Failed to get the prompt. The prompt field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "negative_prompt" => match field.is_text() {
                        true => {
                            let mut negative_prompt = String::new();

                            if let Err(e) = field.data.read_to_string(&mut negative_prompt) {
                                let err_msg = format!("Failed to read the prompt. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            image_request.prompt = negative_prompt;
                        }
                        false => {
                            let err_msg =
                                "Failed to get the negative prompt. The negative prompt field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "mask" => {
                        let filename = match field.headers.filename {
                            Some(filename) => filename,
                            None => {
                                let err_msg =
                                    "Failed to upload the image mask file. The filename is not provided.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };

                        // get the image data
                        let mut buffer = Vec::new();
                        let size_in_bytes = match field.data.read_to_end(&mut buffer) {
                            Ok(size_in_bytes) => size_in_bytes,
                            Err(e) => {
                                let err_msg = format!("Failed to read the image mask file. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };

                        // create a file id for the image file
                        let id = format!("file_{}", uuid::Uuid::new_v4());

                        // save the file
                        let path = std::path::Path::new("archives");
                        if !path.exists() {
                            fs::create_dir(path).unwrap();
                        }
                        let file_path = path.join(&id);
                        if !file_path.exists() {
                            fs::create_dir(&file_path).unwrap();
                        }
                        let mut file = match File::create(file_path.join(&filename)) {
                            Ok(file) => file,
                            Err(e) => {
                                let err_msg = format!(
                                    "Failed to create archive document {}. {}",
                                    &filename, e
                                );

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };
                        file.write_all(&buffer[..]).unwrap();

                        // log
                        info!(target: "stdout", "file_id: {}, file_name: {}, size in bytes: {}", &id, &filename, size_in_bytes);

                        let created_at =
                            match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                                Ok(n) => n.as_secs(),
                                Err(_) => {
                                    let err_msg = "Failed to get the current time.";

                                    // log
                                    error!(target: "stdout", "{}", err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            };

                        // create a file object
                        image_request.mask = Some(FileObject {
                            id,
                            bytes: size_in_bytes as u64,
                            created_at,
                            filename,
                            object: "file".to_string(),
                            purpose: "assistants".to_string(),
                        });
                    }
                    "model" => match field.is_text() {
                        true => {
                            let mut model = String::new();

                            if let Err(e) = field.data.read_to_string(&mut model) {
                                let err_msg = format!("Failed to read the model. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            image_request.model = model;
                        }
                        false => {
                            let err_msg =
                                "Failed to get the model name. The model field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "n" => match field.is_text() {
                        true => {
                            let mut n = String::new();

                            if let Err(e) = field.data.read_to_string(&mut n) {
                                let err_msg = format!("Failed to read the number of images. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match n.parse::<u64>() {
                                Ok(n) => image_request.n = Some(n),
                                Err(e) => {
                                    let err_msg = format!(
                                        "Failed to parse the number of images. Reason: {}",
                                        e
                                    );

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                            "Failed to get the number of images. The n field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "size" => {
                        match field.is_text() {
                            true => {
                                let mut size = String::new();

                                if let Err(e) = field.data.read_to_string(&mut size) {
                                    let err_msg = format!("Failed to read the size. {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }

                                // image_request.size = Some(size);

                                let parts: Vec<&str> = size.split('x').collect();
                                if parts.len() != 2 {
                                    let err_msg = "Invalid size format. The correct format is `HeightxWidth`. Example: 256x256";

                                    // log
                                    error!(target: "stdout", "{}", err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                                image_request.height = Some(parts[0].parse().unwrap());
                                image_request.width = Some(parts[1].parse().unwrap());
                            }
                            false => {
                                let err_msg =
                                "Failed to get the size. The size field in the request should be a text field.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        }
                    }
                    "response_format" => match field.is_text() {
                        true => {
                            let mut response_format = String::new();

                            if let Err(e) = field.data.read_to_string(&mut response_format) {
                                let err_msg = format!("Failed to read the response format. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match response_format.parse::<ResponseFormat>() {
                                Ok(format) => image_request.response_format = Some(format),
                                Err(e) => {
                                    let err_msg = format!(
                                        "Failed to parse the response format. Reason: {}",
                                        e
                                    );

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the response format. The response format field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "user" => match field.is_text() {
                        true => {
                            let mut user = String::new();

                            if let Err(e) = field.data.read_to_string(&mut user) {
                                let err_msg = format!("Failed to read the user. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            image_request.user = Some(user);
                        }
                        false => {
                            let err_msg =
                                "Failed to get the user. The user field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "cfg_scale" => match field.is_text() {
                        true => {
                            let mut cfg_scale = String::new();

                            if let Err(e) = field.data.read_to_string(&mut cfg_scale) {
                                let err_msg = format!("Failed to read the cfg_config. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match cfg_scale.parse::<f32>() {
                                Ok(scale) => image_request.cfg_scale = Some(scale),
                                Err(e) => {
                                    let err_msg = format!(
                                        "Failed to parse the number of images. Reason: {}",
                                        e
                                    );

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the cfg_config. The cfg_config field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "sample_method" => match field.is_text() {
                        true => {
                            let mut sample_method = String::new();

                            if let Err(e) = field.data.read_to_string(&mut sample_method) {
                                let err_msg = format!("Failed to read the sample_method. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            image_request.sample_method = Some(sample_method.as_str().into());
                        }
                        false => {
                            let err_msg =
                                "Failed to get the sample_method. The sample_method field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "steps" => match field.is_text() {
                        true => {
                            let mut steps = String::new();

                            if let Err(e) = field.data.read_to_string(&mut steps) {
                                let err_msg = format!("Failed to read the steps. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match steps.parse::<usize>() {
                                Ok(steps) => image_request.steps = Some(steps),
                                Err(e) => {
                                    let err_msg =
                                        format!("Failed to parse the steps. Reason: {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the steps. The steps field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "height" => match field.is_text() {
                        true => {
                            let mut height = String::new();

                            if let Err(e) = field.data.read_to_string(&mut height) {
                                let err_msg = format!("Failed to read the height. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match height.parse::<usize>() {
                                Ok(height) => image_request.height = Some(height),
                                Err(e) => {
                                    let err_msg =
                                        format!("Failed to parse the height. Reason: {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the height. The height field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "width" => match field.is_text() {
                        true => {
                            let mut width = String::new();

                            if let Err(e) = field.data.read_to_string(&mut width) {
                                let err_msg = format!("Failed to read the width. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match width.parse::<usize>() {
                                Ok(width) => image_request.width = Some(width),
                                Err(e) => {
                                    let err_msg =
                                        format!("Failed to parse the width. Reason: {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the width. The width field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "control_strength" => match field.is_text() {
                        true => {
                            let mut control_strength = String::new();

                            if let Err(e) = field.data.read_to_string(&mut control_strength) {
                                let err_msg = format!("Failed to read the control_strength. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match control_strength.parse::<f32>() {
                                Ok(control_strength) => {
                                    image_request.control_strength = Some(control_strength)
                                }
                                Err(e) => {
                                    let err_msg = format!(
                                        "Failed to parse the control_strength. Reason: {}",
                                        e
                                    );

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the control_strength. The control_strength field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "control_image" => {
                        let filename = match field.headers.filename {
                            Some(filename) => filename,
                            None => {
                                let err_msg =
                                    "Failed to upload the image file. The filename is not provided.";

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };

                        // get the image data
                        let mut buffer = Vec::new();
                        let size_in_bytes = match field.data.read_to_end(&mut buffer) {
                            Ok(size_in_bytes) => size_in_bytes,
                            Err(e) => {
                                let err_msg = format!("Failed to read the image file. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };

                        // create a file id for the image file
                        let id = format!("file_{}", uuid::Uuid::new_v4());

                        // save the file
                        let path = std::path::Path::new("archives");
                        if !path.exists() {
                            fs::create_dir(path).unwrap();
                        }
                        let file_path = path.join(&id);
                        if !file_path.exists() {
                            fs::create_dir(&file_path).unwrap();
                        }
                        let mut file = match File::create(file_path.join(&filename)) {
                            Ok(file) => file,
                            Err(e) => {
                                let err_msg = format!(
                                    "Failed to create archive document {}. {}",
                                    &filename, e
                                );

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }
                        };
                        file.write_all(&buffer[..]).unwrap();

                        // log
                        info!(target: "stdout", "file_id: {}, file_name: {}, size in bytes: {}", &id, &filename, size_in_bytes);

                        let created_at =
                            match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                                Ok(n) => n.as_secs(),
                                Err(_) => {
                                    let err_msg = "Failed to get the current time.";

                                    // log
                                    error!(target: "stdout", "{}", err_msg);

                                    return Ok(error::internal_server_error(err_msg));
                                }
                            };

                        // create a file object
                        image_request.control_image = Some(FileObject {
                            id,
                            bytes: size_in_bytes as u64,
                            created_at,
                            filename,
                            object: "file".to_string(),
                            purpose: "assistants".to_string(),
                        });
                    }
                    "seed" => match field.is_text() {
                        true => {
                            let mut seed = String::new();

                            if let Err(e) = field.data.read_to_string(&mut seed) {
                                let err_msg = format!("Failed to read the seed. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match seed.parse::<i32>() {
                                Ok(seed) => image_request.seed = Some(seed),
                                Err(e) => {
                                    let err_msg =
                                        format!("Failed to parse the seed. Reason: {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the seed. The seed field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    "strength" => match field.is_text() {
                        true => {
                            let mut strength = String::new();

                            if let Err(e) = field.data.read_to_string(&mut strength) {
                                let err_msg = format!("Failed to read the strength. {}", e);

                                // log
                                error!(target: "stdout", "{}", &err_msg);

                                return Ok(error::internal_server_error(err_msg));
                            }

                            match strength.parse::<f32>() {
                                Ok(strength) => {
                                    image_request.strength = Some(strength);
                                    info!(target: "stdout", "strength: {}", strength);
                                }
                                Err(e) => {
                                    let err_msg =
                                        format!("Failed to parse the strength. Reason: {}", e);

                                    // log
                                    error!(target: "stdout", "{}", &err_msg);

                                    return Ok(error::bad_request(err_msg));
                                }
                            }
                        }
                        false => {
                            let err_msg =
                                "Failed to get the strength. The strength field in the request should be a text field.";

                            // log
                            error!(target: "stdout", "{}", &err_msg);

                            return Ok(error::internal_server_error(err_msg));
                        }
                    },
                    unsupported_field => {
                        let err_msg = format!("Unsupported field: {}", unsupported_field);

                        // log
                        error!(target: "stdout", "{}", &err_msg);

                        return Ok(error::bad_request(err_msg));
                    }
                }
            }

            image_request
        } else {
            let err_msg = "Invalid HTTP Method.";

            // log
            error!(target: "stdout", "{}", &err_msg);

            return Ok(error::internal_server_error(err_msg));
        };

        let image_file = std::path::Path::new("archives")
            .join(&image_request.image.id)
            .join(&image_request.image.filename);
        if !image_file.exists() {
            let err_msg = format!(
                "Not found the init image: {}",
                &image_file.to_string_lossy()
            );

            error!(target: "stdout", "{}", &err_msg);

            return Ok(error::bad_request(err_msg));
        }

        // convert the image to base64 string
        let base64_string = match image_to_base64(image_file) {
            Ok(base64_string) => base64_string,
            Err(e) => {
                let err_msg = format!("Fail to convert the image to base64 string. {}", e);

                error!(target: "stdout", "{}", &err_msg);

                return Ok(error::internal_server_error(&err_msg));
            }
        };

        let data = json!({
                "prompt": image_request.prompt,
                "negative_prompt": image_request.negative_prompt.unwrap(),
                "sampler_index": image_request.sample_method.unwrap().to_string(),
                "seed": image_request.seed.unwrap(),
                "scheduler": image_request.scheduler.unwrap(),
                "batch_size": 1,
                "n_iter": 1,
                "steps": image_request.steps.unwrap(),
                "cfg_scale": image_request.cfg_scale.unwrap(),
                "width": image_request.width.unwrap(),
                "height": image_request.height.unwrap(),
                "denoising_strength": image_request.strength.unwrap(),
                "init_images": [
                    base64_string,
                ]
        });

        (data, "/sdapi/v1/img2img")
    } else {
        let err_msg = format!(
            "404 The requested service endpoint is not found: {}",
            endpoint
        );

        error!(target: "stdout", "{}", &err_msg);

        return Err(StatusCode::NOT_FOUND);
    };

    let body = serde_json::to_string(&data).unwrap();

    let mut server_socket_addr = downstream_url.to_string();
    server_socket_addr = server_socket_addr.trim_end_matches('/').to_string();

    let downstream_uri: Uri = format!("{}{}", server_socket_addr, downstream_endpoint)
        .parse()
        .unwrap();
    info!(target: "stdout", "dispatch the request to {}", downstream_uri);

    // create a request to the downstream server
    let downstream_request = Request::builder()
        .method(req.method())
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
                        image_objects.push(ImageObject {
                            b64_json: Some(image.to_string()),
                            url: None,
                            prompt: data.get("prompt").map(|v| v.to_string()),
                        })
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

// convert an image file to a base64 string
fn image_to_base64(image_path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
    // Open the file
    let mut image_file = File::open(image_path)?;

    // Read the file into a byte array
    let mut buffer = Vec::new();
    image_file.read_to_end(&mut buffer)?;

    Ok(general_purpose::STANDARD.encode(&buffer))
}
