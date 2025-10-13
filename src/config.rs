use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub rate_limiting: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub default_requests_per_second: u32,
    pub default_burst_capacity: u32,
    pub cleanup_interval: u64,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable {0} not found")]
    EnvVarNotFound(String),
    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
    #[error("Configuration parsing error: {0}")]
    ParseError(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv::dotenv().ok(); // Load .env file if present

        let server = ServerConfig {
            host: env::var("THROTTLER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("THROTTLER_PORT")
                .unwrap_or_else(|_| "3030".to_string())
                .parse()
                .map_err(|e| ConfigError::InvalidValue(format!("Invalid port: {}", e)))?,
            workers: env::var("THROTTLER_WORKERS")
                .ok()
                .map(|w| w.parse().map_err(|e| ConfigError::InvalidValue(format!("Invalid workers: {}", e))))
                .transpose()?,
        };

        let redis = RedisConfig {
            url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            pool_size: env::var("REDIS_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|e| ConfigError::InvalidValue(format!("Invalid pool size: {}", e)))?,
            connection_timeout: env::var("REDIS_CONNECTION_TIMEOUT")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .map_err(|e| ConfigError::InvalidValue(format!("Invalid timeout: {}", e)))?,
        };

        let rate_limiting = RateLimitConfig {
            default_requests_per_second: env::var("DEFAULT_REQUESTS_PER_SECOND")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|e| ConfigError::InvalidValue(format!("Invalid RPS: {}", e)))?,
            default_burst_capacity: env::var("DEFAULT_BURST_CAPACITY")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .map_err(|e| ConfigError::InvalidValue(format!("Invalid burst capacity: {}", e)))?,
            cleanup_interval: env::var("CLEANUP_INTERVAL")
                .unwrap_or_else(|_| "60000".to_string())
                .parse()
                .map_err(|e| ConfigError::InvalidValue(format!("Invalid cleanup interval: {}", e)))?,
        };

        Ok(Config {
            server,
            redis,
            rate_limiting,
        })
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::InvalidValue("Port cannot be 0".to_string()));
        }

        if self.redis.pool_size == 0 {
            return Err(ConfigError::InvalidValue("Redis pool size cannot be 0".to_string()));
        }

        if self.rate_limiting.default_requests_per_second == 0 {
            return Err(ConfigError::InvalidValue("RPS cannot be 0".to_string()));
        }

        if self.rate_limiting.default_burst_capacity == 0 {
            return Err(ConfigError::InvalidValue("Burst capacity cannot be 0".to_string()));
        }

        Ok(())
    }
}