use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use redis::AsyncCommands;
use crate::redis::RedisPool;
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    pub dependencies: DependencyStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub redis: ServiceStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub status: String,
    pub response_time_ms: u64,
    pub error: Option<String>,
}

static START_TIME: std::sync::LazyLock<SystemTime> = std::sync::LazyLock::new(SystemTime::now);

pub struct HealthChecker {
    redis_pool: RedisPool,
}

impl HealthChecker {
    pub fn new(redis_pool: RedisPool) -> Self {
        Self { redis_pool }
    }

    pub async fn check_health(&self) -> Result<HealthStatus, AppError> {
        let now = SystemTime::now();
        let uptime = now.duration_since(*START_TIME)
            .unwrap_or_default()
            .as_secs();

        let redis_status = self.check_redis().await;

        let overall_status = if redis_status.status == "healthy" {
            "healthy"
        } else {
            "unhealthy"
        };

        Ok(HealthStatus {
            status: overall_status.to_string(),
            timestamp: now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            dependencies: DependencyStatus {
                redis: redis_status,
            },
        })
    }

    async fn check_redis(&self) -> ServiceStatus {
        let start = SystemTime::now();
        
        match self.redis_pool.get().await {
            Ok(mut conn) => {
                match conn.ping().await {
                    Ok(_) => {
                        let response_time = start.elapsed()
                            .unwrap_or_default()
                            .as_millis() as u64;
                        
                        ServiceStatus {
                            status: "healthy".to_string(),
                            response_time_ms: response_time,
                            error: None,
                        }
                    }
                    Err(e) => ServiceStatus {
                        status: "unhealthy".to_string(),
                        response_time_ms: start.elapsed()
                            .unwrap_or_default()
                            .as_millis() as u64,
                        error: Some(format!("Redis ping failed: {}", e)),
                    },
                }
            }
            Err(e) => ServiceStatus {
                status: "unhealthy".to_string(),
                response_time_ms: start.elapsed()
                    .unwrap_or_default()
                    .as_millis() as u64,
                error: Some(format!("Redis connection failed: {}", e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus {
            status: "healthy".to_string(),
            timestamp: 1234567890,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            dependencies: DependencyStatus {
                redis: ServiceStatus {
                    status: "healthy".to_string(),
                    response_time_ms: 5,
                    error: None,
                },
            },
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1234567890"));
    }
}