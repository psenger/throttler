use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
pub struct RateLimitResponse {
    pub allowed: bool,
    pub remaining: u64,
    pub reset_time: u64,
    pub retry_after: Option<u64>,
}

impl RateLimitResponse {
    pub fn allowed(remaining: u64, reset_time: u64) -> Self {
        Self {
            allowed: true,
            remaining,
            reset_time,
            retry_after: None,
        }
    }

    pub fn denied(reset_time: u64, retry_after: u64) -> Self {
        Self {
            allowed: false,
            remaining: 0,
            reset_time,
            retry_after: Some(retry_after),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub redis_connected: bool,
}

impl HealthResponse {
    pub fn healthy(redis_connected: bool) -> Self {
        Self {
            status: "healthy".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            redis_connected,
        }
    }

    pub fn unhealthy(redis_connected: bool) -> Self {
        Self {
            status: "unhealthy".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            redis_connected,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub message: String,
    pub config: serde_json::Value,
}

impl ConfigResponse {
    pub fn updated(config: serde_json::Value) -> Self {
        Self {
            message: "Configuration updated successfully".to_string(),
            config,
        }
    }

    pub fn current(config: serde_json::Value) -> Self {
        Self {
            message: "Current configuration".to_string(),
            config,
        }
    }
}
