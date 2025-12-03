use std::time::Duration;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use throttler::{
    config::Config,
    server::create_app,
    token_bucket::TokenBucket,
};

/// Helper function to convert response body to bytes
async fn body_to_bytes(body: Body) -> Vec<u8> {
    let collected = body.collect().await.unwrap();
    collected.to_bytes().to_vec()
}

#[tokio::test]
async fn test_token_bucket_edge_cases() {
    // Test zero refill rate
    let mut bucket = TokenBucket::new(10, 0.0);
    assert!(bucket.try_consume(5).unwrap());

    let wait_time = bucket.time_until_tokens(10).unwrap();
    assert_eq!(wait_time, Duration::from_secs(u64::MAX));

    // Test very small time differences
    let mut bucket = TokenBucket::new(100, 10.0);
    bucket.try_consume(50).unwrap();

    // Immediate refill attempt should not cause issues
    let available = bucket.available_tokens().unwrap();
    assert!(available <= 50);

    // Test floating point precision
    let mut bucket = TokenBucket::new(1000, 0.1);
    for _ in 0..10 {
        bucket.refill().unwrap();
    }

    assert!(bucket.tokens <= 1000.0);
}

#[tokio::test]
async fn test_health_endpoint() {
    let config = Config::default();
    let app = create_app(config).unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_bytes(response.into_body()).await;
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_readiness_endpoint() {
    let config = Config::default();
    let app = create_app(config).unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/ready")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_bytes(response.into_body()).await;
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn test_rate_limit_check() {
    let config = Config::default();
    let app = create_app(config).unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/rate-limit/test-key/check")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"tokens": 1}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_bytes(response.into_body()).await;
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["allowed"], true);
}

#[tokio::test]
async fn test_get_rate_limit() {
    let config = Config::default();
    let app = create_app(config).unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/rate-limit/test-key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_bytes(response.into_body()).await;
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["key"], "test-key");
    assert!(body["remaining"].is_number());
}

#[tokio::test]
async fn test_empty_key_rejected() {
    let config = Config::default();
    let app = create_app(config).unwrap();

    // Empty key should be rejected (results in 404 for missing path param)
    let request = Request::builder()
        .method("GET")
        .uri("/rate-limit/")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 Not Found for empty key path
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_token_bucket_basic_operations() {
    let mut bucket = TokenBucket::new(100, 10.0);

    // Test initial state
    assert_eq!(bucket.capacity, 100);
    assert_eq!(bucket.tokens, 100.0);

    // Test consumption
    assert!(bucket.try_consume(50).unwrap());
    assert_eq!(bucket.tokens, 50.0);

    // Test cannot consume more than available
    assert!(!bucket.try_consume(60).unwrap());
    assert_eq!(bucket.tokens, 50.0);

    // Test reset
    bucket.reset();
    assert_eq!(bucket.tokens, 100.0);
}

#[tokio::test]
async fn test_token_bucket_serialization() {
    let bucket = TokenBucket::new(100, 10.0);

    // Test serialization roundtrip
    let json = serde_json::to_string(&bucket).unwrap();
    let deserialized: TokenBucket = serde_json::from_str(&json).unwrap();

    assert_eq!(bucket.capacity, deserialized.capacity);
    assert_eq!(bucket.refill_rate, deserialized.refill_rate);
}
