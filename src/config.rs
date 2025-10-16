use serde::{Deserialize, Serialize};
use std::env;
use crate::error::ThrottlerError;

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
    pub max_connections: usize,
    pub request_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
    pub key_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub default_capacity: u32,
    pub default_refill_rate: u32,
    pub default_window_seconds: u64,
    pub max_capacity: u32,
    pub cleanup_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            redis: RedisConfig::default(),
            rate_limiting: RateLimitConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            request_timeout: 30,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            max_connections: 10,
            connection_timeout: 5,
            key_prefix: "throttler:".to_string(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            default_capacity: 100,
            default_refill_rate: 10,
            default_window_seconds: 60,
            max_capacity: 10000,
            cleanup_interval: 300,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ThrottlerError> {
        let mut config = Config::default();
        
        // Server configuration
        if let Ok(host) = env::var("THROTTLER_HOST") {
            config.server.host = host;
        }
        
        if let Ok(port) = env::var("THROTTLER_PORT") {
            config.server.port = port.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid port number".to_string()))?;
        }
        
        if let Ok(max_conn) = env::var("THROTTLER_MAX_CONNECTIONS") {
            config.server.max_connections = max_conn.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid max connections".to_string()))?;
        }
        
        if let Ok(timeout) = env::var("THROTTLER_REQUEST_TIMEOUT") {
            config.server.request_timeout = timeout.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid request timeout".to_string()))?;
        }
        
        // Redis configuration
        if let Ok(redis_url) = env::var("REDIS_URL") {
            config.redis.url = redis_url;
        }
        
        if let Ok(redis_max_conn) = env::var("REDIS_MAX_CONNECTIONS") {
            config.redis.max_connections = redis_max_conn.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid Redis max connections".to_string()))?;
        }
        
        if let Ok(redis_timeout) = env::var("REDIS_CONNECTION_TIMEOUT") {
            config.redis.connection_timeout = redis_timeout.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid Redis connection timeout".to_string()))?;
        }
        
        if let Ok(key_prefix) = env::var("REDIS_KEY_PREFIX") {
            config.redis.key_prefix = key_prefix;
        }
        
        // Rate limiting configuration
        if let Ok(default_capacity) = env::var("DEFAULT_RATE_LIMIT_CAPACITY") {
            config.rate_limiting.default_capacity = default_capacity.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid default capacity".to_string()))?;
        }
        
        if let Ok(default_refill) = env::var("DEFAULT_RATE_LIMIT_REFILL") {
            config.rate_limiting.default_refill_rate = default_refill.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid default refill rate".to_string()))?;
        }
        
        if let Ok(default_window) = env::var("DEFAULT_RATE_LIMIT_WINDOW") {
            config.rate_limiting.default_window_seconds = default_window.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid default window".to_string()))?;
        }
        
        if let Ok(max_capacity) = env::var("MAX_RATE_LIMIT_CAPACITY") {
            config.rate_limiting.max_capacity = max_capacity.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid max capacity".to_string()))?;
        }
        
        if let Ok(cleanup_interval) = env::var("CLEANUP_INTERVAL") {
            config.rate_limiting.cleanup_interval = cleanup_interval.parse()
                .map_err(|_| ThrottlerError::ConfigError("Invalid cleanup interval".to_string()))?;
        }
        
        config.validate()?;
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<(), ThrottlerError> {
        // Validate server config
        if self.server.port == 0 {
            return Err(ThrottlerError::ConfigError("Port must be greater than 0".to_string()));
        }
        
        if self.server.max_connections == 0 {
            return Err(ThrottlerError::ConfigError("Max connections must be greater than 0".to_string()));
        }
        
        // Validate Redis config
        if self.redis.url.is_empty() {
            return Err(ThrottlerError::ConfigError("Redis URL cannot be empty".to_string()));
        }
        
        if self.redis.max_connections == 0 {
            return Err(ThrottlerError::ConfigError("Redis max connections must be greater than 0".to_string()));
        }
        
        // Validate rate limiting config
        if self.rate_limiting.default_capacity == 0 {
            return Err(ThrottlerError::ConfigError("Default capacity must be greater than 0".to_string()));
        }
        
        if self.rate_limiting.default_refill_rate == 0 {
            return Err(ThrottlerError::ConfigError("Default refill rate must be greater than 0".to_string()));
        }
        
        if self.rate_limiting.default_capacity > self.rate_limiting.max_capacity {
            return Err(ThrottlerError::ConfigError("Default capacity cannot exceed max capacity".to_string()));
        }
        
        Ok(())
    }
    
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}