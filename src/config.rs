use std::env;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub redis_url: String,
    pub default_rate_limit: u32,
    pub default_window_seconds: u64,
    pub max_bucket_size: u32,
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidPort(String),
    InvalidRateLimit(String),
    InvalidWindowSize(String),
    InvalidBucketSize(String),
    MissingRedisUrl,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidPort(msg) => write!(f, "Invalid port: {}", msg),
            ConfigError::InvalidRateLimit(msg) => write!(f, "Invalid rate limit: {}", msg),
            ConfigError::InvalidWindowSize(msg) => write!(f, "Invalid window size: {}", msg),
            ConfigError::InvalidBucketSize(msg) => write!(f, "Invalid bucket size: {}", msg),
            ConfigError::MissingRedisUrl => write!(f, "Redis URL is required"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            redis_url: "redis://127.0.0.1:6379".to_string(),
            default_rate_limit: 100,
            default_window_seconds: 3600,
            max_bucket_size: 1000,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        if let Ok(host) = env::var("THROTTLER_HOST") {
            config.server_host = host;
        }

        if let Ok(port_str) = env::var("THROTTLER_PORT") {
            config.server_port = port_str.parse()
                .map_err(|_| ConfigError::InvalidPort(port_str))?;
        }

        if let Ok(redis_url) = env::var("REDIS_URL") {
            config.redis_url = redis_url;
        } else if env::var("REDIS_URL").is_err() && config.redis_url.is_empty() {
            return Err(ConfigError::MissingRedisUrl);
        }

        if let Ok(rate_limit_str) = env::var("DEFAULT_RATE_LIMIT") {
            config.default_rate_limit = rate_limit_str.parse()
                .map_err(|_| ConfigError::InvalidRateLimit(rate_limit_str))?;
        }

        if let Ok(window_str) = env::var("DEFAULT_WINDOW_SECONDS") {
            config.default_window_seconds = window_str.parse()
                .map_err(|_| ConfigError::InvalidWindowSize(window_str))?;
        }

        if let Ok(bucket_str) = env::var("MAX_BUCKET_SIZE") {
            config.max_bucket_size = bucket_str.parse()
                .map_err(|_| ConfigError::InvalidBucketSize(bucket_str))?;
        }

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.server_port == 0 {
            return Err(ConfigError::InvalidPort("Port cannot be 0".to_string()));
        }

        if self.default_rate_limit == 0 {
            return Err(ConfigError::InvalidRateLimit("Rate limit must be greater than 0".to_string()));
        }

        if self.default_window_seconds == 0 {
            return Err(ConfigError::InvalidWindowSize("Window size must be greater than 0".to_string()));
        }

        if self.max_bucket_size == 0 {
            return Err(ConfigError::InvalidBucketSize("Bucket size must be greater than 0".to_string()));
        }

        Ok(())
    }
}