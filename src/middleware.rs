use axum::{extract::Request, middleware::Next, response::Response};
use std::net::SocketAddr;
use tracing::info;

/// Logging middleware for request/response tracking
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let client_ip = get_client_ip(&request);

    info!(
        target: "throttler::middleware",
        method = %method,
        uri = %uri,
        client_ip = %client_ip,
        "Incoming request"
    );

    let response = next.run(request).await;

    let status = response.status();
    info!(
        target: "throttler::middleware",
        method = %method,
        uri = %uri,
        status = %status,
        "Request completed"
    );

    response
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
    use axum::http::HeaderValue;

    #[test]
    fn test_get_client_ip_with_forwarded_header() {
        let mut request = Request::new(axum::body::Body::empty());
        request.headers_mut().insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.168.1.1, 10.0.0.1")
        );

        let ip = get_client_ip(&request);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_get_client_ip_with_real_ip_header() {
        let mut request = Request::new(axum::body::Body::empty());
        request.headers_mut().insert(
            "x-real-ip",
            HeaderValue::from_static("203.0.113.1")
        );

        let ip = get_client_ip(&request);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_get_client_ip_fallback() {
        let request = Request::new(axum::body::Body::empty());
        let ip = get_client_ip(&request);
        assert_eq!(ip, "unknown");
    }
}
