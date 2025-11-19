use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::net::SocketAddr;
use tracing::{error, warn};

use crate::error::ThrottlerError;
use crate::rate_limiter::RateLimiter;
use crate::response::ThrottleResponse;

pub async fn throttling_middleware(
    rate_limiter: axum::extract::State<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = get_client_ip(&request);
    let path = request.uri().path();
    let method = request.method();
    
    match rate_limiter.check_rate_limit(&client_ip, path, method.as_str()).await {
        Ok(allowed) => {
            if allowed {
                Ok(next.run(request).await)
            } else {
                warn!("Rate limit exceeded for client: {} on path: {}", client_ip, path);
                let response = ThrottleResponse::rate_limited(
                    "Rate limit exceeded".to_string(),
                    None,
                );
                Ok(response.into_response())
            }
        }
        Err(ThrottlerError::RedisConnection(ref err)) => {
            error!("Redis connection failure in throttling middleware: {}", err);
            // Fail open - allow request when Redis is unavailable
            warn!("Failing open due to Redis unavailability for client: {}", client_ip);
            Ok(next.run(request).await)
        }
        Err(ThrottlerError::RedisOperation(ref err)) => {
            error!("Redis operation failed in throttling middleware: {}", err);
            // Fail open for Redis operation errors
            warn!("Failing open due to Redis operation error for client: {}", client_ip);
            Ok(next.run(request).await)
        }
        Err(err) => {
            error!("Unexpected error in throttling middleware: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn get_client_ip(request: &Request) -> String {
    // Try to get real IP from headers first
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }
    
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }
    
    // Fallback to connection info
    if let Some(addr) = request.extensions().get::<SocketAddr>() {
        addr.ip().to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, Method, Uri};
    use std::str::FromStr;
    
    #[test]
    fn test_get_client_ip_with_forwarded_header() {
        let mut request = Request::new(());
        request.headers_mut().insert(
            "x-forwarded-for", 
            HeaderValue::from_static("192.168.1.1, 10.0.0.1")
        );
        
        let ip = get_client_ip(&request);
        assert_eq!(ip, "192.168.1.1");
    }
    
    #[test]
    fn test_get_client_ip_with_real_ip_header() {
        let mut request = Request::new(());
        request.headers_mut().insert(
            "x-real-ip", 
            HeaderValue::from_static("203.0.113.1")
        );
        
        let ip = get_client_ip(&request);
        assert_eq!(ip, "203.0.113.1");
    }
    
    #[test]
    fn test_get_client_ip_fallback() {
        let request = Request::new(());
        let ip = get_client_ip(&request);
        assert_eq!(ip, "unknown");
    }
}