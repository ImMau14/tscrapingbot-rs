use axum::body::Body;
use axum::body::to_bytes;
use axum::http::Request;
use tower::util::ServiceExt;
use tscrapingbot_rs::server;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = server::build_router(None);

    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.expect("service call failed");
    assert_eq!(resp.status(), 200);

    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(body_str.contains("\"status\":\"ok\""));
}
