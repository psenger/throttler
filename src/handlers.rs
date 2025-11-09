use crate::error::ThrottlerError;
use crate::rate_limit_config::RateLimitConfig;
use crate::response::{ApiResponse, ErrorResponse};
use crate::throttler::Throttler;
use crate::validation::validate_rate_limit_config;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SharedState = Arc<RwLock<HashMap<String, RateLimitConfig>>>;

/// Get rate limit configuration for a specific key
pub async fn get_rate_limit(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<RateLimitConfig>>, (StatusCode, Json<ErrorResponse>)> {
    let configs = state.read().await;
    
    match configs.get(&key) {
        Some(config) => Ok(Json(ApiResponse {
            success: true,
            data: Some(config.clone()),
            message: "Rate limit configuration retrieved successfully".to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: "Rate limit configuration not found".to_string(),
                code: "NOT_FOUND".to_string(),
            }),
        )),
    }
}

/// Set rate limit configuration for a specific key
pub async fn set_rate_limit(
    Path(key): Path<String>,
    State(state): State<SharedState>,
    Json(config): Json<RateLimitConfig>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    // Validate the configuration
    if let Err(validation_error) = validate_rate_limit_config(&config) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: validation_error,
                code: "VALIDATION_ERROR".to_string(),
            }),
        ));
    }

    let mut configs = state.write().await;
    configs.insert(key, config);
    
    Ok(Json(ApiResponse {
        success: true,
        data: None,
        message: "Rate limit configuration set successfully".to_string(),
    }))
}

/// Delete rate limit configuration for a specific key
pub async fn delete_rate_limit(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    let mut configs = state.write().await;
    
    match configs.remove(&key) {
        Some(_) => Ok(Json(ApiResponse {
            success: true,
            data: None,
            message: "Rate limit configuration deleted successfully".to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: "Rate limit configuration not found".to_string(),
                code: "NOT_FOUND".to_string(),
            }),
        )),
    }
}

/// Check rate limit for a specific key
pub async fn check_rate_limit(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ErrorResponse>)> {
    let configs = state.read().await;
    
    match configs.get(&key) {
        Some(config) => {
            let mut throttler = Throttler::new(config.clone());
            match throttler.check_rate_limit(&key).await {
                Ok(allowed) => Ok(Json(ApiResponse {
                    success: true,
                    data: Some(allowed),
                    message: if allowed {
                        "Request allowed".to_string()
                    } else {
                        "Rate limit exceeded".to_string()
                    },
                })),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        success: false,
                        error: e.to_string(),
                        code: "THROTTLER_ERROR".to_string(),
                    }),
                )),
            }
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: "Rate limit configuration not found".to_string(),
                code: "NOT_FOUND".to_string(),
            }),
        )),
    }
}

/// Health check endpoint
pub async fn health_check() -> Json<ApiResponse<HashMap<String, String>>> {
    let mut health_info = HashMap::new();
    health_info.insert("status".to_string(), "healthy".to_string());
    health_info.insert("service".to_string(), "throttler".to_string());
    health_info.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    health_info.insert("uptime".to_string(), "running".to_string());
    
    Json(ApiResponse {
        success: true,
        data: Some(health_info),
        message: "Service is healthy".to_string(),
    })
}

/// Readiness check endpoint
pub async fn readiness_check() -> Result<Json<ApiResponse<HashMap<String, String>>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Add actual readiness checks (Redis connectivity, etc.)
    let mut readiness_info = HashMap::new();
    readiness_info.insert("status".to_string(), "ready".to_string());
    readiness_info.insert("redis".to_string(), "connected".to_string());
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(readiness_info),
        message: "Service is ready".to_string(),
    }))
}