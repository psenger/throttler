use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};

/// Logging middleware that tracks request duration and basic metrics
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status = response.status();
    
    if status.is_success() {
        info!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = duration.as_millis(),
            "Request completed"
        );
    } else {
        warn!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = duration.as_millis(),
            "Request completed with error"
        );
    }
    
    response
}

/// Rate limiting middleware that applies throttling rules
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    // For now, pass through - will be implemented with actual rate limiting logic
    // This is a placeholder for future rate limiting integration
    next.run(request).await
}

/// CORS middleware for API access
pub async fn cors_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    headers.insert(
        "access-control-allow-origin",
        "*".parse().unwrap(),
    );
    headers.insert(
        "access-control-allow-methods",
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "access-control-allow-headers",
        "content-type, authorization".parse().unwrap(),
    );
    
    response
}