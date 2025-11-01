use crate::error::{ThrottlerError, ThrottlerResult};
use crate::redis::RedisClient;
use crate::response::ApiResponse;
use crate::throttler::Throttler;
use crate::validation::validate_rate_limit_request;
use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct CreateRateLimitRequest {
    pub key: String,
    pub limit: u64,
    pub window_seconds: u64,
    pub burst_capacity: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    pub key: String,
    pub limit: u64,
    pub window_seconds: u64,
    pub remaining: u64,
    pub reset_time: u64,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub redis_connected: bool,
    pub uptime_seconds: u64,
}

static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn init_start_time() {
    START_TIME.set(std::time::Instant::now()).ok();
}

pub async fn health_check(
    State(redis_client): State<Arc<RedisClient>>,
) -> ThrottlerResult<Json<ApiResponse<HealthStatus>>> {
    let start_time = START_TIME.get().copied().unwrap_or_else(std::time::Instant::now);
    let uptime = start_time.elapsed().as_secs();
    
    let redis_connected = match redis_client.ping().await {
        Ok(_) => true,
        Err(_) => false,
    };

    let health = HealthStatus {
        status: if redis_connected { "healthy".to_string() } else { "degraded".to_string() },
        version: env!("CARGO_PKG_VERSION").to_string(),
        redis_connected,
        uptime_seconds: uptime,
    };

    Ok(Json(ApiResponse::success(health)))
}

pub async fn create_rate_limit(
    State(throttler): State<Arc<Throttler>>,
    Json(request): Json<CreateRateLimitRequest>,
) -> ThrottlerResult<Json<ApiResponse<RateLimitInfo>>> {
    validate_rate_limit_request(&request)?;

    let bucket_capacity = request.burst_capacity.unwrap_or(request.limit);
    
    throttler
        .create_rate_limit(
            &request.key,
            request.limit,
            request.window_seconds,
            bucket_capacity,
        )
        .await?;

    let info = RateLimitInfo {
        key: request.key,
        limit: request.limit,
        window_seconds: request.window_seconds,
        remaining: bucket_capacity,
        reset_time: chrono::Utc::now().timestamp() as u64 + request.window_seconds,
    };

    Ok(Json(ApiResponse::success(info)))
}

pub async fn check_rate_limit(
    State(throttler): State<Arc<Throttler>>,
    Path(key): Path<String>,
) -> ThrottlerResult<Json<ApiResponse<RateLimitInfo>>> {
    if key.is_empty() {
        return Err(ThrottlerError::InvalidRequest(
            "Rate limit key cannot be empty".to_string(),
        ));
    }

    let (allowed, remaining, reset_time) = throttler.check_rate_limit(&key, 1).await?;

    if !allowed {
        return Err(ThrottlerError::RateLimitExceeded(format!(
            "Rate limit exceeded for key: {}",
            key
        )));
    }

    // Get rate limit configuration for this key
    let config = throttler.get_rate_limit_config(&key).await?;

    let info = RateLimitInfo {
        key,
        limit: config.limit,
        window_seconds: config.window_seconds,
        remaining,
        reset_time,
    };

    Ok(Json(ApiResponse::success(info)))
}

pub async fn delete_rate_limit(
    State(throttler): State<Arc<Throttler>>,
    Path(key): Path<String>,
) -> ThrottlerResult<Json<ApiResponse<HashMap<String, String>>>> {
    if key.is_empty() {
        return Err(ThrottlerError::InvalidRequest(
            "Rate limit key cannot be empty".to_string(),
        ));
    }

    throttler.delete_rate_limit(&key).await?;

    let mut response = HashMap::new();
    response.insert("message".to_string(), format!("Rate limit for key '{}' deleted successfully", key));

    Ok(Json(ApiResponse::success(response)))
}

pub async fn list_rate_limits(
    State(throttler): State<Arc<Throttler>>,
) -> ThrottlerResult<Json<ApiResponse<Vec<String>>>> {
    let keys = throttler.list_rate_limit_keys().await?;
    Ok(Json(ApiResponse::success(keys)))
}