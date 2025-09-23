//! HTTP API server implementation

use crate::app::AppState;
use crate::api::routes;
use anyhow::{Context, Result};
use axum::{
    http::{HeaderValue, Method},
    Router,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level};

/// HTTP API server
pub struct ApiServer {
    app: Router,
    addr: SocketAddr,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(state: Arc<AppState>) -> Result<Self> {
        let config = &state.config;
        
        // Parse server address
        let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
            .parse()
            .context("Invalid server host/port configuration")?;

        // Build CORS layer
        let cors = if config.server.cors_enabled {
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(tower_http::cors::Any)
        } else {
            CorsLayer::permissive()
        };

        // Build the router
        let app = Router::new()
            .nest("/", routes::create_routes())
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO))
            )
            .layer(TimeoutLayer::new(Duration::from_secs(
                config.server.request_timeout_seconds,
            )))
            .layer(cors)
            .with_state(state);

        info!("API server configured for {}", addr);

        Ok(Self { app, addr })
    }

    /// Run the API server
    pub async fn run(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.addr)
            .await
            .context("Failed to bind to server address")?;

        info!("API server listening on {}", self.addr);

        axum::serve(listener, self.app.clone())
            .await
            .context("API server error")?;

        Ok(())
    }

    /// Shutdown the API server
    pub async fn shutdown(&mut self) -> Result<()> {
        // Axum doesn't have explicit shutdown in the current version
        // The server will shutdown when the task is cancelled
        info!("API server shutdown initiated");
        Ok(())
    }
}
