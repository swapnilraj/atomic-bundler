//! Atomic Bundler Middleware - Main Application Entry Point

use anyhow::{Context, Result};
use config::ConfigLoader;
use std::env;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod app;
mod database;
mod scheduler;
mod storage;

use app::Application;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    if let Err(e) = dotenv::dotenv() {
        // Only warn if the error is not "file not found"
        if !e.to_string().contains("No such file or directory") {
            warn!("Could not load .env file: {}", e);
        }
    } else {
        info!("Loaded environment variables from .env file");
    }

    // Initialize logging
    init_logging()?;

    info!("Starting Atomic Bundler v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    let config = ConfigLoader::load(&config_path)
        .context("Failed to load configuration")?;

    info!("Configuration loaded from: {}", config_path);
    info!("Network: {}", config.network.network);
    let enabled_builders: Vec<String> = config.builders.iter()
        .filter(|b| b.enabled)
        .map(|b| b.name.clone())
        .collect();
    info!("Enabled builders: {}", enabled_builders.join(", "));

    // Create and start the application
    let mut app = Application::new(config).await
        .context("Failed to create application")?;

    // Setup signal handling
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutdown signal received");
    };

    // Run the application
    info!("Application starting...");
    tokio::select! {
        result = app.run() => {
            if let Err(e) = result {
                tracing::error!("Application error: {}", e);
                return Err(e);
            }
        }
        _ = shutdown_signal => {
            info!("Initiating graceful shutdown...");
            app.shutdown().await?;
        }
    }

    info!("Atomic Bundler shutdown complete");
    Ok(())
}

/// Initialize logging based on environment variables
fn init_logging() -> Result<()> {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let log_format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));

    let registry = tracing_subscriber::registry().with(env_filter);

    match log_format.as_str() {
        "pretty" => {
            registry
                .with(tracing_subscriber::fmt::layer().pretty())
                .try_init()
                .context("Failed to initialize pretty logging")?;
        }
        "json" | _ => {
            registry
                .with(tracing_subscriber::fmt::layer().json())
                .try_init()
                .context("Failed to initialize JSON logging")?;
        }
    }

    // Log configuration
    info!("Logging initialized");
    info!("Log level: {}", log_level);
    info!("Log format: {}", log_format);

    if log_level == "trace" || log_level == "debug" {
        warn!("Debug/trace logging enabled - may impact performance in production");
    }

    Ok(())
}
