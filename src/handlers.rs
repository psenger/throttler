use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ThrottlerError;
use crate::response::ApiResponse;
use crate::throttler::Throttler;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLimiterRequest {
    pub key: String,
    pub capacity: u32,
    pub refill_rate: u32,
    pub window_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLimiterRequest {
    pub capacity: Option<u32>,
    pub refill_rate: Option<u32>,
    pub window_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LimiterInfo {
    pub key: String,
    pub capacity: u32,
    pub refill_rate: u32,
    pub window_seconds: u64,
    pub current_tokens: u32,
    pub last_refill: u64,
}

type ThrottlerState = Arc<RwLock<Throttler>>;

/// Create a new rate limiter
pub async fn create_limiter(
    throttler: web::Data<ThrottlerState>,
    req: web::Json<CreateLimiterRequest>,
) -> Result<HttpResponse, ThrottlerError> {
    let mut throttler_guard = throttler.write().await;
    
    throttler_guard.add_limiter(
        &req.key,
        req.capacity,
        req.refill_rate,
        req.window_seconds,
    ).await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(
        format!("Rate limiter '{}' created successfully", req.key),
        Some(serde_json::json!({
            "key": req.key,
            "capacity": req.capacity,
            "refill_rate": req.refill_rate,
            "window_seconds": req.window_seconds
        }))
    )))
}

/// Get rate limiter information
pub async fn get_limiter(
    throttler: web::Data<ThrottlerState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ThrottlerError> {
    let key = path.into_inner();
    let throttler_guard = throttler.read().await;
    
    let info = throttler_guard.get_limiter_info(&key).await?;
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        "Rate limiter information retrieved".to_string(),
        Some(info)
    )))
}

/// Update rate limiter configuration
pub async fn update_limiter(
    throttler: web::Data<ThrottlerState>,
    path: web::Path<String>,
    req: web::Json<UpdateLimiterRequest>,
) -> Result<HttpResponse, ThrottlerError> {
    let key = path.into_inner();
    let mut throttler_guard = throttler.write().await;
    
    throttler_guard.update_limiter(
        &key,
        req.capacity,
        req.refill_rate,
        req.window_seconds,
    ).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        format!("Rate limiter '{}' updated successfully", key),
        None
    )))
}

/// Delete a rate limiter
pub async fn delete_limiter(
    throttler: web::Data<ThrottlerState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ThrottlerError> {
    let key = path.into_inner();
    let mut throttler_guard = throttler.write().await;
    
    throttler_guard.remove_limiter(&key).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        format!("Rate limiter '{}' deleted successfully", key),
        None
    )))
}

/// List all rate limiters
pub async fn list_limiters(
    throttler: web::Data<ThrottlerState>,
) -> Result<HttpResponse, ThrottlerError> {
    let throttler_guard = throttler.read().await;
    let limiters = throttler_guard.list_limiters().await?;
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        "Rate limiters listed successfully".to_string(),
        Some(serde_json::json!({
            "count": limiters.len(),
            "limiters": limiters
        }))
    )))
}

/// Check if a request is allowed (throttling endpoint)
pub async fn check_request(
    throttler: web::Data<ThrottlerState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, ThrottlerError> {
    let key = path.into_inner();
    let tokens_requested = query.get("tokens")
        .and_then(|t| t.parse::<u32>().ok())
        .unwrap_or(1);
    
    let throttler_guard = throttler.read().await;
    let allowed = throttler_guard.check_rate_limit(&key, tokens_requested).await?;
    
    if allowed {
        Ok(HttpResponse::Ok().json(ApiResponse::success(
            "Request allowed".to_string(),
            Some(serde_json::json!({
                "allowed": true,
                "tokens_requested": tokens_requested
            }))
        )))
    } else {
        Ok(HttpResponse::TooManyRequests().json(ApiResponse::error(
            "Rate limit exceeded".to_string(),
            Some(serde_json::json!({
                "allowed": false,
                "tokens_requested": tokens_requested,
                "retry_after": "Check rate limiter configuration for refill rate"
            }))
        )))
    }
}

/// Health check endpoint
pub async fn health_check() -> Result<HttpResponse, ThrottlerError> {
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        "Throttler service is healthy".to_string(),
        Some(serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().timestamp()
        }))
    )))
}