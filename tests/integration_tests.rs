use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use throttler::config::Config;
use throttler::server::create_app;

#[tokio::test]
async fn test_basic_rate_limiting() {
    let config = Config::new();
    let app = create_app(config).await;
    
    // Start test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    let client = Client::new();
    let base_url = format!("http://{}", addr);
    
    // Create a rate limit configuration
    let config_payload = json!({
        "key": "test_api",
        "requests_per_second": 2,
        "burst_capacity": 5
    });
    
    let response = client
        .post(&format!("{}/config", base_url))
        .json(&config_payload)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 201);
    
    // Test rate limiting
    for i in 1..=5 {
        let response = client
            .post(&format!("{}/throttle", base_url))
            .header("X-API-Key", "test_api")
            .send()
            .await
            .unwrap();
        
        if i <= 5 {
            assert_eq!(response.status(), 200);
        }
    }
    
    // This request should be rate limited
    let response = client
        .post(&format!("{}/throttle", base_url))
        .header("X-API-Key", "test_api")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 429);
}

#[tokio::test]
async fn test_token_bucket_refill() {
    let config = Config::new();
    let app = create_app(config).await;
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    let client = Client::new();
    let base_url = format!("http://{}", addr);
    
    // Create a strict rate limit
    let config_payload = json!({
        "key": "refill_test",
        "requests_per_second": 1,
        "burst_capacity": 1
    });
    
    client
        .post(&format!("{}/config", base_url))
        .json(&config_payload)
        .send()
        .await
        .unwrap();
    
    // Use up the token
    let response = client
        .post(&format!("{}/throttle", base_url))
        .header("X-API-Key", "refill_test")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    
    // Should be rate limited
    let response = client
        .post(&format!("{}/throttle", base_url))
        .header("X-API-Key", "refill_test")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 429);
    
    // Wait for token refill
    sleep(Duration::from_millis(1100)).await;
    
    // Should work again
    let response = client
        .post(&format!("{}/throttle", base_url))
        .header("X-API-Key", "refill_test")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_config_management() {
    let config = Config::new();
    let app = create_app(config).await;
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    let client = Client::new();
    let base_url = format!("http://{}", addr);
    
    // Create config
    let config_payload = json!({
        "key": "config_test",
        "requests_per_second": 10,
        "burst_capacity": 20
    });
    
    let response = client
        .post(&format!("{}/config", base_url))
        .json(&config_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    
    // Get config
    let response = client
        .get(&format!("{}/config/config_test", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    
    let config_data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(config_data["key"], "config_test");
    assert_eq!(config_data["requests_per_second"], 10);
    
    // Update config
    let updated_config = json!({
        "key": "config_test",
        "requests_per_second": 15,
        "burst_capacity": 25
    });
    
    let response = client
        .put(&format!("{}/config/config_test", base_url))
        .json(&updated_config)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    
    // Delete config
    let response = client
        .delete(&format!("{}/config/config_test", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    
    // Should not exist anymore
    let response = client
        .get(&format!("{}/config/config_test", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let config = Config::new();
    let app = create_app(config).await;
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    let client = Client::new();
    let base_url = format!("http://{}", addr);
    
    // Create config and make some requests
    let config_payload = json!({
        "key": "metrics_test",
        "requests_per_second": 5,
        "burst_capacity": 10
    });
    
    client
        .post(&format!("{}/config", base_url))
        .json(&config_payload)
        .send()
        .await
        .unwrap();
    
    // Make some throttle requests
    for _ in 0..3 {
        client
            .post(&format!("{}/throttle", base_url))
            .header("X-API-Key", "metrics_test")
            .send()
            .await
            .unwrap();
    }
    
    // Check metrics
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let metrics: serde_json::Value = response.json().await.unwrap();
    assert!(metrics["total_requests"].as_u64().unwrap() >= 3);
}

#[tokio::test]
async fn test_health_check() {
    let config = Config::new();
    let app = create_app(config).await;
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    let client = Client::new();
    let base_url = format!("http://{}", addr);
    
    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let health: serde_json::Value = response.json().await.unwrap();
    assert_eq!(health["status"], "healthy");
}