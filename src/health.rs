use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use crate::rate_limiter::RateLimiter;

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
    rate_limiter: RateLimiter,
}

impl HealthChecker {
    pub fn new(rate_limiter: RateLimiter) -> Self {
        Self { rate_limiter }
    }

    pub fn check_health(&self) -> HealthStatus {
        let now = SystemTime::now();
        let uptime = now.duration_since(*START_TIME)
            .unwrap_or_default()
            .as_secs();

        let redis_status = self.check_redis();

        let overall_status = if redis_status.status == "healthy" {
            "healthy"
        } else {
            "degraded" // Not unhealthy, just running without Redis
        };

        HealthStatus {
            status: overall_status.to_string(),
            timestamp: now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            dependencies: DependencyStatus {
                redis: redis_status,
            },
        }
    }

    fn check_redis(&self) -> ServiceStatus {
        let start = SystemTime::now();

        if self.rate_limiter.is_redis_available() {
            let response_time = start.elapsed()
                .unwrap_or_default()
                .as_millis() as u64;

            ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: response_time,
                error: None,
            }
        } else {
            ServiceStatus {
                status: "unavailable".to_string(),
                response_time_ms: start.elapsed()
                    .unwrap_or_default()
                    .as_millis() as u64,
                error: Some("Redis not configured or not reachable".to_string()),
            }
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
