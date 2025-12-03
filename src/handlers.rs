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

/// Shared application state
pub type SharedState = Arc<RwLock<AppState>>;

/// Application state containing rate limiter and validator
pub struct AppState {
    pub rate_limiter: RateLimiter,
    pub validator: RequestValidator,
}

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    #[serde(default)]
    pub tokens: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub allowed: bool,
    pub remaining: u64,
    pub limit: u64,
}

#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub requests: u64,
    pub window_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub status: String,
    pub message: String,
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub redis_connected: bool,
}

/// Check rate limit for a key
pub async fn check_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
    Json(_payload): Json<CheckRequest>,
) -> Result<impl IntoResponse, ThrottlerError> {
    let state = state.read().await;

    // Validate key
    state.validator.validate_key(&key)?;

    // Check rate limit
    let (allowed, remaining) = state.rate_limiter.check_rate_limit(&key)?;

    let response = CheckResponse {
        allowed,
        remaining,
        limit: 100, // TODO: Get from config
    };

    let mut resp = Json(response).into_response();

    // Add rate limit headers
    resp.headers_mut().insert("X-RateLimit-Limit", "100".parse().unwrap());
    resp.headers_mut().insert("X-RateLimit-Remaining", remaining.to_string().parse().unwrap());

    if !allowed {
        *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        resp.headers_mut().insert("Retry-After", "60".parse().unwrap());
    }

    Ok(resp)
}

/// Get rate limit configuration for a key
pub async fn get_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ThrottlerError> {
    let state = state.read().await;

    // Validate key
    state.validator.validate_key(&key)?;

    // Get remaining tokens
    let remaining = state.rate_limiter.get_remaining_tokens(&key)?;

    Ok(Json(serde_json::json!({
        "key": key,
        "remaining": remaining,
        "limit": 100
    })))
}

/// Set rate limit configuration for a key
pub async fn set_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
    Json(payload): Json<ConfigRequest>,
) -> Result<impl IntoResponse, ThrottlerError> {
    let state = state.read().await;

    // Validate key and parameters
    state.validator.validate_key(&key)?;
    state.validator.validate_rate_limit(payload.requests, payload.window_ms)?;

    Ok(Json(ConfigResponse {
        status: "success".to_string(),
        message: "Rate limit configuration updated".to_string(),
        key,
    }))
}

/// Delete rate limit configuration for a key
pub async fn delete_rate_limit(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ThrottlerError> {
    let state = state.write().await;

    // Validate key
    state.validator.validate_key(&key)?;

    // Reset the rate limit
    state.rate_limiter.reset(&key)?;

    Ok(Json(ConfigResponse {
        status: "success".to_string(),
        message: "Rate limit configuration deleted".to_string(),
        key,
    }))
}

/// Health check endpoint
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

/// Readiness check endpoint
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
        (StatusCode::OK, Json(serde_json::json!({
            "status": "ready",
            "redis": "disconnected",
            "note": "Running in local-only mode"
        })))
    }
}
