//! HTTP middleware implementations

use crate::app::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Middleware to check killswitch status
pub async fn killswitch_check(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip killswitch check for health endpoints and admin endpoints
    let path = request.uri().path();
    if path.starts_with("/healthz") || path.starts_with("/admin/") || path.starts_with("/status") {
        return Ok(next.run(request).await);
    }

    // Check if killswitch is active
    if state.is_killswitch_active().await {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    Ok(next.run(request).await)
}

/// Create killswitch middleware layer
pub fn killswitch_middleware(state: Arc<AppState>) -> axum::middleware::FromFnLayer<fn(State<Arc<AppState>>, Request<Body>, Next) -> Result<Response, StatusCode>, State<Arc<AppState>>, Arc<AppState>> {
    axum::middleware::from_fn_with_state(state, killswitch_check)
}

// metrics middleware removed for now
