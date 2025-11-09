use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    error::AppError,
    health::HealthChecker,
    metrics::MetricsCollector,
    rate_limit_config::{RateLimitConfig, RateLimitRule},
    response::ApiResponse,
    throttler::Throttler,
    validation::ValidationError,
};

#[derive(Debug, Deserialize)]
pub struct CheckRateLimitQuery {
    pub client_id: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub allowed: bool,
    pub remaining: u64,
    pub reset_time: u64,
    pub retry_after: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub throttler: Throttler,
    pub health_checker: HealthChecker,
    pub metrics: MetricsCollector,
}

pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::health::HealthStatus>>, AppError> {
    let health_status = state.health_checker.check_health().await?;
    
    let response = ApiResponse {
        success: health_status.status == "healthy",
        data: Some(health_status),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn check_rate_limit(
    Query(query): Query<CheckRateLimitQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<RateLimitStatus>>, AppError> {
    let endpoint = query.endpoint.unwrap_or_else(|| "default".to_string());
    
    let result = state.throttler.check_rate_limit(
        &query.client_id,
        &endpoint,
    ).await?;
    
    state.metrics.record_rate_limit_check(&query.client_id, &endpoint, result.allowed).await;
    
    let status = RateLimitStatus {
        allowed: result.allowed,
        remaining: result.remaining,
        reset_time: result.reset_time,
        retry_after: if result.allowed { None } else { Some(result.retry_after) },
    };
    
    let response = ApiResponse {
        success: true,
        data: Some(status),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn create_rate_limit_config(
    State(state): State<AppState>,
    Json(config): Json<RateLimitConfig>,
) -> Result<Json<ApiResponse<RateLimitConfig>>, AppError> {
    // Validate the configuration
    config.validate().map_err(|e| AppError::Validation(e))?;
    
    state.throttler.create_config(config.clone()).await?;
    
    let response = ApiResponse {
        success: true,
        data: Some(config),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn get_rate_limit_config(
    Path(config_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<RateLimitConfig>>, AppError> {
    let config = state.throttler.get_config(config_id).await?;
    
    let response = ApiResponse {
        success: true,
        data: Some(config),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn update_rate_limit_config(
    Path(config_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(mut config): Json<RateLimitConfig>,
) -> Result<Json<ApiResponse<RateLimitConfig>>, AppError> {
    config.id = config_id;
    config.validate().map_err(|e| AppError::Validation(e))?;
    
    state.throttler.update_config(config.clone()).await?;
    
    let response = ApiResponse {
        success: true,
        data: Some(config),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn delete_rate_limit_config(
    Path(config_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.throttler.delete_config(config_id).await?;
    
    let response = ApiResponse {
        success: true,
        data: None,
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn list_rate_limit_configs(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<RateLimitConfig>>>, AppError> {
    let configs = state.throttler.list_configs().await?;
    
    let response = ApiResponse {
        success: true,
        data: Some(configs),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}

pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::metrics::MetricsSummary>>, AppError> {
    let metrics = state.metrics.get_summary().await;
    
    let response = ApiResponse {
        success: true,
        data: Some(metrics),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    
    Ok(Json(response))
}