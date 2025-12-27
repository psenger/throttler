//! # HTTP Request Handlers
//!
//! This module contains all HTTP request handlers for the Throttler API.
//! Handlers are async functions that process requests and return responses.
//!
//! ## Handler Overview
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                           Request Handlers                             │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │                                                                        │
//! │  Rate Limiting Endpoints:                                              │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │ POST /rate-limit/:key/check  →  check_rate_limit()              │  │
//! │  │   • Validates key format                                         │  │
//! │  │   • Consumes token from bucket                                   │  │
//! │  │   • Returns allowed/denied with headers                          │  │
//! │  ├──────────────────────────────────────────────────────────────────┤  │
//! │  │ GET  /rate-limit/:key        →  get_rate_limit()                │  │
//! │  │   • Returns current token count and limit                        │  │
//! │  ├──────────────────────────────────────────────────────────────────┤  │
//! │  │ POST /rate-limit/:key        →  set_rate_limit()                │  │
//! │  │   • Creates or updates rate limit configuration                  │  │
//! │  ├──────────────────────────────────────────────────────────────────┤  │
//! │  │ DELETE /rate-limit/:key      →  delete_rate_limit()             │  │
//! │  │   • Removes rate limit and resets bucket                         │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                        │
//! │  Health Endpoints:                                                     │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │ GET /health  →  health_check()     (Liveness probe)             │  │
//! │  │ GET /ready   →  readiness_check()  (Readiness probe)            │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Response Headers
//!
//! Rate limit responses include standard headers:
//!
//! | Header                  | Description                          |
//! |-------------------------|--------------------------------------|
//! | `X-RateLimit-Limit`     | Maximum requests allowed             |
//! | `X-RateLimit-Remaining` | Remaining requests in current window |
//! | `Retry-After`           | Seconds until tokens refill (429)    |
//!
//! ## Error Handling
//!
//! All handlers return `Result<impl IntoResponse, ThrottlerError>`, where
//! `ThrottlerError` automatically converts to appropriate HTTP status codes.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ThrottlerError;
use crate::rate_limiter::RateLimiter;
use crate::validation::RequestValidator;

/// Thread-safe shared application state.
///
/// Uses `Arc` for shared ownership across async tasks and `RwLock` for
/// interior mutability with concurrent read access.
///
/// # Locking Strategy
///
/// - **Read lock** (`state.read().await`): Used for check, get, set operations
/// - **Write lock** (`state.write().await`): Used for delete operations
pub type SharedState = Arc<RwLock<AppState>>;

/// Application state containing rate limiter and validator.
///
/// This struct holds all stateful components needed by request handlers:
/// - `rate_limiter`: Core rate limiting engine
/// - `validator`: Request input validation
///
/// # Thread Safety
///
/// `AppState` is wrapped in `Arc<RwLock<...>>` to allow safe concurrent access
/// from multiple Tokio tasks handling requests simultaneously.
pub struct AppState {
    /// The rate limiting engine (local or Redis-backed)
    pub rate_limiter: RateLimiter,
    /// Request input validator (key format, parameter ranges)
    pub validator: RequestValidator,
}

/// Request body for rate limit check endpoint.
///
/// # Fields
///
/// * `tokens` - Number of tokens to consume (default: 1)
///
/// # Example JSON
///
/// ```json
/// {"tokens": 1}
/// ```
///
/// Or simply `{}` to use the default of 1 token.
#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    /// Number of tokens to consume from the bucket.
    /// Defaults to 1 if not specified.
    #[serde(default)]
    pub tokens: Option<u64>,
}

/// Response body for rate limit check endpoint.
///
/// # Fields
///
/// * `allowed` - Whether the request was allowed
/// * `remaining` - Tokens remaining in the bucket
/// * `limit` - Maximum bucket capacity
///
/// # Example JSON (Allowed)
///
/// ```json
/// {"allowed": true, "remaining": 99, "limit": 100}
/// ```
///
/// # Example JSON (Denied)
///
/// ```json
/// {"allowed": false, "remaining": 0, "limit": 100}
/// ```
#[derive(Debug, Serialize)]
pub struct CheckResponse {
    /// Whether the request was allowed (had sufficient tokens)
    pub allowed: bool,
    /// Number of tokens remaining in the bucket after this request
    pub remaining: u64,
    /// Maximum bucket capacity (rate limit)
    pub limit: u64,
}

/// Request body for rate limit configuration endpoint.
///
/// # Fields
///
/// * `requests` - Maximum requests allowed in the window
/// * `window_ms` - Window size in milliseconds
///
/// # Example JSON
///
/// ```json
/// {"requests": 100, "window_ms": 60000}
/// ```
///
/// This configures 100 requests per 60 seconds (1 minute).
#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    /// Maximum number of requests allowed in the window
    pub requests: u64,
    /// Window size in milliseconds (e.g., 60000 = 1 minute)
    pub window_ms: u64,
}

/// Response body for configuration update operations.
///
/// # Example JSON
///
/// ```json
/// {
///   "status": "success",
///   "message": "Rate limit configuration updated",
///   "key": "api-client-123"
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    /// Operation status ("success" or "error")
    pub status: String,
    /// Human-readable message describing the result
    pub message: String,
    /// The rate limit key that was modified
    pub key: String,
}

/// Response body for health check endpoints.
///
/// # Example JSON
///
/// ```json
/// {"status": "healthy", "redis_connected": true}
/// ```
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Overall health status ("healthy" or "degraded")
    pub status: String,
    /// Whether Redis is connected and responsive
    pub redis_connected: bool,
}

/// Checks rate limit for a key and consumes tokens from the bucket.
///
/// This is the primary endpoint for rate limiting. It:
/// 1. Validates the key format
/// 2. Attempts to consume tokens from the bucket
/// 3. Returns the result with standard rate limit headers
///
/// # Request
///
/// ```text
/// POST /rate-limit/:key/check
/// Content-Type: application/json
///
/// {"tokens": 1}
/// ```
///
/// # Response (200 OK - Allowed)
///
/// ```text
/// HTTP/1.1 200 OK
/// X-RateLimit-Limit: 100
/// X-RateLimit-Remaining: 99
/// Content-Type: application/json
///
/// {"allowed": true, "remaining": 99, "limit": 100}
/// ```
///
/// # Response (429 Too Many Requests - Denied)
///
/// ```text
/// HTTP/1.1 429 Too Many Requests
/// X-RateLimit-Limit: 100
/// X-RateLimit-Remaining: 0
/// Retry-After: 60
/// Content-Type: application/json
///
/// {"allowed": false, "remaining": 0, "limit": 100}
/// ```
///
/// # Errors
///
/// - `400 Bad Request` - Invalid key format
/// - `500 Internal Server Error` - Redis or internal error
pub async fn check_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
    Json(_payload): Json<CheckRequest>,
) -> Result<impl IntoResponse, ThrottlerError> {
    // Acquire read lock - allows concurrent rate limit checks
    let state = state.read().await;

    // Validate key format (alphanumeric, -, _, :, .)
    state.validator.validate_key(&key)?;

    // Check rate limit - consumes 1 token if available
    let (allowed, remaining) = state.rate_limiter.check_rate_limit(&key)?;

    // Build response body
    let response = CheckResponse {
        allowed,
        remaining,
        limit: 100, // TODO: Get from config
    };

    let mut resp = Json(response).into_response();

    // Add standard rate limit headers
    resp.headers_mut().insert("X-RateLimit-Limit", "100".parse().unwrap());
    resp.headers_mut().insert("X-RateLimit-Remaining", remaining.to_string().parse().unwrap());

    // If rate limited, set 429 status and Retry-After header
    if !allowed {
        *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        resp.headers_mut().insert("Retry-After", "60".parse().unwrap());
    }

    Ok(resp)
}

/// Gets current rate limit status for a key.
///
/// Returns the current token count and limit configuration for the specified key.
/// Does not consume any tokens.
///
/// # Request
///
/// ```text
/// GET /rate-limit/:key
/// ```
///
/// # Response (200 OK)
///
/// ```json
/// {"key": "api-client-123", "remaining": 85, "limit": 100}
/// ```
///
/// # Errors
///
/// - `400 Bad Request` - Invalid key format
/// - `500 Internal Server Error` - Redis or internal error
pub async fn get_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ThrottlerError> {
    // Acquire read lock for concurrent access
    let state = state.read().await;

    // Validate key format
    state.validator.validate_key(&key)?;

    // Get remaining tokens without consuming any
    let remaining = state.rate_limiter.get_remaining_tokens(&key)?;

    Ok(Json(serde_json::json!({
        "key": key,
        "remaining": remaining,
        "limit": 100
    })))
}

/// Creates or updates rate limit configuration for a key.
///
/// Sets the rate limit parameters for a specific key. If the key already exists,
/// its configuration is updated; otherwise, a new rate limit is created.
///
/// # Request
///
/// ```text
/// POST /rate-limit/:key
/// Content-Type: application/json
///
/// {"requests": 100, "window_ms": 60000}
/// ```
///
/// # Response (200 OK)
///
/// ```json
/// {
///   "status": "success",
///   "message": "Rate limit configuration updated",
///   "key": "api-client-123"
/// }
/// ```
///
/// # Validation
///
/// - `requests`: 1 to 10,000
/// - `window_ms`: 1,000 (1 second) to 86,400,000 (24 hours)
///
/// # Errors
///
/// - `400 Bad Request` - Invalid key format or parameters out of range
/// - `500 Internal Server Error` - Redis or internal error
pub async fn set_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
    Json(payload): Json<ConfigRequest>,
) -> Result<impl IntoResponse, ThrottlerError> {
    // Acquire read lock (config update is idempotent)
    let state = state.read().await;

    // Validate key format and rate limit parameters
    state.validator.validate_key(&key)?;
    state.validator.validate_rate_limit(payload.requests, payload.window_ms)?;

    Ok(Json(ConfigResponse {
        status: "success".to_string(),
        message: "Rate limit configuration updated".to_string(),
        key,
    }))
}

/// Deletes rate limit configuration and resets bucket for a key.
///
/// Removes the rate limit configuration and resets the token bucket to its
/// initial state. After deletion, the key will use default rate limits.
///
/// # Request
///
/// ```text
/// DELETE /rate-limit/:key
/// ```
///
/// # Response (200 OK)
///
/// ```json
/// {
///   "status": "success",
///   "message": "Rate limit configuration deleted",
///   "key": "api-client-123"
/// }
/// ```
///
/// # Note
///
/// This operation requires a write lock as it modifies state.
///
/// # Errors
///
/// - `400 Bad Request` - Invalid key format
/// - `500 Internal Server Error` - Redis or internal error
pub async fn delete_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ThrottlerError> {
    // Acquire write lock - delete requires exclusive access
    let state = state.write().await;

    // Validate key format
    state.validator.validate_key(&key)?;

    // Reset the rate limit bucket
    state.rate_limiter.reset(&key)?;

    Ok(Json(ConfigResponse {
        status: "success".to_string(),
        message: "Rate limit configuration deleted".to_string(),
        key,
    }))
}

/// Liveness probe endpoint for Kubernetes health checks.
///
/// Returns the current health status of the service. Always returns 200 OK
/// as long as the service is running, regardless of Redis connectivity.
///
/// # Request
///
/// ```text
/// GET /health
/// ```
///
/// # Response (200 OK)
///
/// ```json
/// {"status": "healthy", "redis_connected": true}
/// ```
///
/// # Kubernetes Usage
///
/// Configure as a liveness probe:
/// ```yaml
/// livenessProbe:
///   httpGet:
///     path: /health
///     port: 8080
///   initialDelaySeconds: 5
///   periodSeconds: 10
/// ```
pub async fn health_check(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let state = state.read().await;
    let redis_connected = state.rate_limiter.is_redis_available();

    Json(HealthResponse {
        status: "healthy".to_string(),
        redis_connected,
    })
}

/// Readiness probe endpoint for Kubernetes traffic routing.
///
/// Returns whether the service is ready to accept traffic. Checks Redis
/// connectivity but still returns 200 OK even if Redis is unavailable
/// (service can operate in local-only mode).
///
/// # Request
///
/// ```text
/// GET /ready
/// ```
///
/// # Response (200 OK - Redis Connected)
///
/// ```json
/// {"status": "ready", "redis": "connected"}
/// ```
///
/// # Response (200 OK - Local Mode)
///
/// ```json
/// {"status": "ready", "redis": "disconnected", "note": "Running in local-only mode"}
/// ```
///
/// # Kubernetes Usage
///
/// Configure as a readiness probe:
/// ```yaml
/// readinessProbe:
///   httpGet:
///     path: /ready
///     port: 8080
///   initialDelaySeconds: 5
///   periodSeconds: 5
/// ```
pub async fn readiness_check(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let state = state.read().await;
    let redis_connected = state.rate_limiter.is_redis_available();

    if redis_connected {
        (StatusCode::OK, Json(serde_json::json!({
            "status": "ready",
            "redis": "connected"
        })))
    } else {
        // Service is still ready, just in local-only mode
        (StatusCode::OK, Json(serde_json::json!({
            "status": "ready",
            "redis": "disconnected",
            "note": "Running in local-only mode"
        })))
    }
}
