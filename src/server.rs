use axum::{Router, routing::get};
use serde_json::json;

pub async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(json!({ "status": "ok" }))
}

/// Build the main router. webhook_router can be passed (via webhooks::axumtorouter)
/// or None to only use the /health route (useful in tests).
pub fn build_router(webhook_router: Option<Router>) -> Router {
    let base = Router::new().route("/health", get(health_handler));
    match webhook_router {
        Some(r) => base.merge(r),
        None => base,
    }
}
