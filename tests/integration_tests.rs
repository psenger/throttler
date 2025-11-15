use std::time::Duration;
use tokio::time::sleep;
use throttler::{
    config::Config,
    server::create_app,
    token_bucket::TokenBucket,
};

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
async fn test_rate_limiting_basic() {
    let config = Config::default();
    let app = create_app(config).await.unwrap();
    
    let response = warp::test::request()
        .method("POST")
        .path("/throttle")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "key": "test-key",
            "tokens": 1
        }))
        .reply(&app)
        .await;
        
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_health_endpoint() {
    let config = Config::default();
    let app = create_app(config).await.unwrap();
    
    let response = warp::test::request()
        .method("GET")
        .path("/health")
        .reply(&app)
        .await;
        
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let config = Config::default();
    let app = create_app(config).await.unwrap();
    
    let response = warp::test::request()
        .method("GET")
        .path("/metrics")
        .reply(&app)
        .await;
        
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_rate_limit_exceeded() {
    let config = Config::default();
    let app = create_app(config).await.unwrap();
    
    // Make multiple requests to exceed rate limit
    for i in 0..15 {
        let response = warp::test::request()
            .method("POST")
            .path("/throttle")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "key": "burst-test-key",
                "tokens": 10
            }))
            .reply(&app)
            .await;
            
        if i < 10 {
            assert_eq!(response.status(), 200);
        } else {
            assert_eq!(response.status(), 429);
        }
    }
}