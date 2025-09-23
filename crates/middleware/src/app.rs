//! Main application structure and lifecycle management

use crate::{api::ApiServer, database::Database, scheduler::Scheduler};
use anyhow::{Context, Result};
use config::Config;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Main application state
#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub database: Database,
    pub killswitch: Arc<RwLock<bool>>,
}

/// Main application that coordinates all components
pub struct Application {
    state: Arc<AppState>,
    api_server: ApiServer,
    scheduler: Scheduler,
}

impl Application {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing application components...");

        // Initialize database
        let database = Database::new(&config.database)
            .await
            .context("Failed to initialize database")?;

        // Run database migrations
        database
            .migrate()
            .await
            .context("Failed to run database migrations")?;

        // Create shared application state
        let state = Arc::new(AppState {
            config: config.clone(),
            database,
            killswitch: Arc::new(RwLock::new(false)),
        });

        // Initialize API server
        let api_server = ApiServer::new(state.clone())
            .context("Failed to create API server")?;

        // Initialize scheduler
        let scheduler = Scheduler::new(state.clone())
            .await
            .context("Failed to create scheduler")?;

        info!("Application components initialized successfully");

        Ok(Self {
            state,
            api_server,
            scheduler,
        })
    }

    /// Run the application
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting application services...");

        // Start metrics server if enabled
        // metrics removed

        // Start scheduler
        let scheduler_handle = {
            let mut scheduler = self.scheduler.clone();
            tokio::spawn(async move {
                if let Err(e) = scheduler.run().await {
                    tracing::error!("Scheduler error: {}", e);
                }
            })
        };

        info!("Background scheduler started");

        // Start API server (this will block until shutdown)
        info!("Starting API server on {}:{}", 
            self.state.config.server.host, 
            self.state.config.server.port
        );
        
        tokio::select! {
            result = self.api_server.run() => {
                result.context("API server error")?;
            }
            result = scheduler_handle => {
                result.context("Scheduler task error")?;
            }
        }

        Ok(())
    }

    /// Shutdown the application gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down application...");

        // Set killswitch to stop processing new requests
        {
            let mut killswitch = self.state.killswitch.write().await;
            *killswitch = true;
        }
        info!("Killswitch activated - no new bundles will be processed");

        // Shutdown API server
        self.api_server.shutdown().await
            .context("Failed to shutdown API server")?;
        info!("API server shutdown complete");

        // Shutdown scheduler
        self.scheduler.shutdown().await
            .context("Failed to shutdown scheduler")?;
        info!("Scheduler shutdown complete");

        // Shutdown metrics server
        // metrics removed

        // Close database connections
        self.state.database.close().await
            .context("Failed to close database")?;
        info!("Database connections closed");

        info!("Application shutdown complete");
        Ok(())
    }

    /// Check if the killswitch is activated
    pub async fn is_killswitch_active(&self) -> bool {
        *self.state.killswitch.read().await
    }

    /// Get application state
    pub fn state(&self) -> Arc<AppState> {
        self.state.clone()
    }
}

impl AppState {
    /// Check if the killswitch is activated
    pub async fn is_killswitch_active(&self) -> bool {
        *self.killswitch.read().await
    }

    /// Activate the killswitch
    pub async fn activate_killswitch(&self) {
        let mut killswitch = self.killswitch.write().await;
        *killswitch = true;
        warn!("Killswitch activated - system will stop processing new requests");
    }

    /// Deactivate the killswitch
    pub async fn deactivate_killswitch(&self) {
        let mut killswitch = self.killswitch.write().await;
        *killswitch = false;
        info!("Killswitch deactivated - system will resume processing requests");
    }
}
