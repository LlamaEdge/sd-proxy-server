#[macro_use]
extern crate log;

mod error;
mod handler;
mod utils;

use anyhow::Result;
use async_trait::async_trait;
use axum::{http::Uri, routing::post, Router};
use clap::{ArgGroup, Parser};
use error::ServerError;
use handler::*;
use hyper::{client::HttpConnector, Client};
use std::{
    collections::HashMap,
    fmt,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{net::TcpListener, sync::RwLock};
use utils::LogLevel;

type SharedClient = Arc<Client<HttpConnector>>;

// default port of SD-Proxy-Server
const DEFAULT_PORT: &str = "8080";

#[derive(Debug, Parser)]
#[command(name = "SD-Proxy-Server", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "SD-Proxy-Server")]
#[command(group = ArgGroup::new("socket_address_group").multiple(false).args(&["socket_addr", "port"]))]
struct Cli {
    /// Socket address of SD-Proxy-Server instance. For example, `0.0.0.0:8080`.
    #[arg(long, default_value = None, value_parser = clap::value_parser!(SocketAddr), group = "socket_address_group")]
    socket_addr: Option<SocketAddr>,
    /// Socket address of SD-Proxy-Server instance
    #[arg(long, default_value = DEFAULT_PORT, value_parser = clap::value_parser!(u16), group = "socket_address_group")]
    port: u16,
}

#[allow(clippy::needless_return)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), ServerError> {
    // get the environment variable `RUST_LOG`
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default().to_lowercase();
    let (_, log_level) = match rust_log.is_empty() {
        true => ("stdout", LogLevel::Info),
        false => match rust_log.split_once("=") {
            Some((target, level)) => (target, level.parse().unwrap_or(LogLevel::Info)),
            None => ("stdout", rust_log.parse().unwrap_or(LogLevel::Info)),
        },
    };

    // set global logger
    wasi_logger::Logger::install().expect("failed to install wasi_logger::Logger");
    log::set_max_level(log_level.into());

    // parse the command line arguments
    let cli = Cli::parse();

    // log the version of the server
    info!(target: "stdout", "version: {}", env!("CARGO_PKG_VERSION"));

    // Create a shared HTTP client
    let client = Arc::new(Client::new());

    let app_state = AppState::new(client);

    // Build our application with routes
    let app = Router::new()
        .route("/v1/images/generations", post(image_handler))
        .route("/v1/images/edits", post(image_handler))
        .route("/admin/register/:type", post(add_url_handler))
        .route("/admin/unregister/:type", post(remove_url_handler))
        .route("/admin/servers", post(list_downstream_servers_handler))
        .with_state(app_state);

    // socket address
    let addr = match cli.socket_addr {
        Some(addr) => addr,
        None => SocketAddr::from(([0, 0, 0, 0], cli.port)),
    };
    let tcp_listener = TcpListener::bind(addr).await.unwrap();
    info!(target: "stdout", "Listening on {}", addr);

    // run
    match axum::Server::from_tcp(tcp_listener.into_std().unwrap())
        .unwrap()
        .serve(app.into_make_service())
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerError::Operation(e.to_string())),
    }
}

#[async_trait]
trait RoutingPolicy {
    async fn next(&self) -> Result<Uri, ServerError>;
}

/// Represents a downstream SD server
#[derive(Debug)]
struct Server {
    url: Uri,
    connections: AtomicUsize,
}
impl Server {
    fn new(url: Uri) -> Self {
        Self {
            url,
            connections: AtomicUsize::new(0),
        }
    }
}

#[derive(Debug, Default)]
struct Services {
    servers: RwLock<Vec<Server>>,
}
impl Services {
    async fn push(&mut self, url: Uri) {
        let server = Server::new(url);
        self.servers.write().await.push(server)
    }
}
#[async_trait]
impl RoutingPolicy for Services {
    async fn next(&self) -> Result<Uri, ServerError> {
        if self.servers.read().await.is_empty() {
            return Err(ServerError::NotFoundServer);
        }

        let servers = self.servers.read().await;
        let server = if servers.len() == 1 {
            servers.first().unwrap()
        } else {
            servers
                .iter()
                .min_by(|s1, s2| {
                    s1.connections
                        .load(Ordering::Relaxed)
                        .cmp(&s2.connections.load(Ordering::Relaxed))
                })
                .unwrap()
        };

        server.connections.fetch_add(1, Ordering::Relaxed);
        Ok(server.url.clone())
    }
}

#[derive(Clone)]
struct AppState {
    client: SharedClient,
    image_urls: Arc<RwLock<Services>>,
}

impl AppState {
    fn new(client: SharedClient) -> Self {
        Self {
            client,
            image_urls: Arc::new(RwLock::new(Services::default())),
        }
    }

    async fn add_url(&self, url_type: UrlType, url: &Uri) -> Result<(), ServerError> {
        let mut services = match url_type {
            UrlType::Image => self.image_urls.write().await,
        };

        services.push(url.clone()).await;
        info!(target: "stdout", "registered server url: {}", url);

        Ok(())
    }

    async fn remove_url(&self, url_type: UrlType, url: &Uri) -> Result<(), ServerError> {
        let services = match &url_type {
            UrlType::Image => &self.image_urls,
        };

        let services = services.write().await;
        let before = services.servers.read().await.len();
        services
            .servers
            .write()
            .await
            .retain(|server| &server.url != url);
        let after = services.servers.read().await.len();

        if before == after {
            return Err(ServerError::NotFoundServer);
        }

        // Optionally, log the removal
        info!(target: "stdout", "Removed {} URL: {}", url_type, url);

        Ok(())
    }

    async fn list_downstream_servers(&self) -> HashMap<String, Vec<String>> {
        let image_servers = self
            .image_urls
            .read()
            .await
            .servers
            .read()
            .await
            .iter()
            .map(|s| s.url.to_string())
            .collect();

        let mut servers = HashMap::new();
        servers.insert("image".to_string(), image_servers);

        servers
    }
}

#[derive(Debug)]
enum UrlType {
    Image,
}
impl fmt::Display for UrlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlType::Image => write!(f, "Image"),
        }
    }
}
