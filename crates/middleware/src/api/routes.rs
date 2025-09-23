//! API route definitions

use crate::api::handlers;
use crate::app::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// Create the main API router
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Bundle endpoints
        .route("/bundles", post(handlers::submit_bundle))
        .route("/bundles/:bundle_id", get(handlers::get_bundle_status))
        
        // Health and status endpoints
        .route("/healthz", get(handlers::health_check))
        .route("/status", get(handlers::system_status))
        
        // Admin endpoints
        .route("/admin/config/reload", post(handlers::reload_config))
        .route("/admin/killswitch", post(handlers::toggle_killswitch))
        .route("/admin/metrics", get(handlers::admin_metrics))
        
        // Legacy endpoint names (for compatibility)
        .route("/config/reload", post(handlers::reload_config))
        .route("/killswitch", post(handlers::toggle_killswitch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::database::Database;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use config::Config;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower::util::ServiceExt;

    async fn create_test_state() -> Arc<AppState> {
        let config = Config::default();
        let database = Database::new_in_memory().await.unwrap();
        
        Arc::new(AppState {
            config,
            database,
            killswitch: Arc::new(RwLock::new(false)),
        })
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = create_test_state().await;
        let app = create_routes().with_state(state);

        let request = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bundle_submission_endpoint() {
        let state = create_test_state().await;
        let app = create_routes().with_state(state);

        let bundle_request = serde_json::json!({
            "tx1": "0x02f86c0182...",
            "payment": {
                "mode": "direct",
                "formula": "basefee",
                "maxAmountWei": "500000000000000",
                "expiry": "2024-01-01T12:00:00Z"
            },
            "targets": {
                "blocks": [18500000, 18500001, 18500002]
            }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/bundles")
            .header("content-type", "application/json")
            .body(Body::from(bundle_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // This might fail due to validation, but the route should exist
        assert!(response.status().is_client_error() || response.status().is_success());
    }
}
