use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    response::{ApiResponse, ThrottleResponse},
    throttler::Throttler,
    Result,
};

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub client_id: String,
    pub tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub rate: f64,
    pub capacity: u32,
    pub window_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub service: String,
    pub version: String,
    pub status: String,
    pub uptime_seconds: u64,
}

pub async fn health_check() -> Result<Json<ApiResponse<String>>> {
    Ok(Json(ApiResponse::success("healthy".to_string())))
}

pub async fn get_status(
    State(throttler): State<Throttler>,
) -> Result<Json<ApiResponse<StatusResponse>>> {
    let status = StatusResponse {
        service: "throttler".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: "running".to_string(),
        uptime_seconds: throttler.get_uptime().await,
    };
    
    Ok(Json(ApiResponse::success(status)))
}

pub async fn check_rate_limit(
    State(throttler): State<Throttler>,
    Query(params): Query<CheckRequest>,
) -> Result<Json<ApiResponse<ThrottleResponse>>> {
    let tokens = params.tokens.unwrap_or(1);
    let response = throttler.check_limit(&params.client_id, tokens).await?;
    
    let status = if response.allowed {
        StatusCode::OK
    } else {
        StatusCode::TOO_MANY_REQUESTS
    };
    
    Ok(Json(ApiResponse::success(response)))
}

pub async fn configure_client(
    State(throttler): State<Throttler>,
    Path(client_id): Path<String>,
    Json(config): Json<ConfigRequest>,
) -> Result<Json<ApiResponse<String>>> {
    throttler
        .configure_client(
            &client_id,
            config.rate,
            config.capacity,
            config.window_seconds,
        )
        .await?;
    
    Ok(Json(ApiResponse::success(format!(
        "Client {} configured successfully",
        client_id
    ))))
}

pub async fn get_client_info(
    State(throttler): State<Throttler>,
    Path(client_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    let info = throttler.get_client_info(&client_id).await?;
    Ok(Json(ApiResponse::success(info)))
}

pub async fn reset_client(
    State(throttler): State<Throttler>,
    Path(client_id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    throttler.reset_client(&client_id).await?;
    Ok(Json(ApiResponse::success(format!(
        "Client {} reset successfully",
        client_id
    ))))
}

pub async fn get_metrics(
    State(throttler): State<Throttler>,
) -> Result<Json<ApiResponse<HashMap<String, crate::metrics::ThrottleMetrics>>>> {
    let metrics = throttler.get_all_metrics().await;
    Ok(Json(ApiResponse::success(metrics)))
}

pub async fn get_client_metrics(
    State(throttler): State<Throttler>,
    Path(client_id): Path<String>,
) -> Result<Json<ApiResponse<crate::metrics::ThrottleMetrics>>> {
    let metrics = throttler.get_client_metrics(&client_id).await;
    match metrics {
        Some(metrics) => Ok(Json(ApiResponse::success(metrics))),
        None => Ok(Json(ApiResponse::error(
            "Client not found".to_string(),
            Some("NO_METRICS".to_string()),
        ))),
    }
}

pub async fn reset_client_metrics(
    State(throttler): State<Throttler>,
    Path(client_id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    throttler.reset_client_metrics(&client_id).await;
    Ok(Json(ApiResponse::success(format!(
        "Metrics for client {} reset successfully",
        client_id
    ))))
}