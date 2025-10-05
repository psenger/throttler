use envconfig::Envconfig;
use std::net::SocketAddr;

#[derive(Debug, Envconfig, Clone)]
pub struct Config {
    /// Server bind address
    #[envconfig(from = "BIND_ADDR", default = "127.0.0.1:3000")]
    pub bind_addr: SocketAddr,
    
    /// Redis connection URL
    #[envconfig(from = "REDIS_URL", default = "redis://127.0.0.1:6379")]
    pub redis_url: String,
    
    /// Default rate limit requests per second
    #[envconfig(from = "DEFAULT_RATE_LIMIT", default = "100")]
    pub default_rate_limit: u32,
    
    /// Token bucket capacity multiplier
    #[envconfig(from = "BUCKET_CAPACITY_MULTIPLIER", default = "2")]
    pub bucket_capacity_multiplier: u32,
    
    /// Rate limiter cleanup interval in seconds
    #[envconfig(from = "CLEANUP_INTERVAL", default = "300")]
    pub cleanup_interval_secs: u64,
    
    /// Enable request tracing
    #[envconfig(from = "ENABLE_TRACING", default = "true")]
    pub enable_tracing: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, envconfig::Error> {
        Config::init_from_env()
    }
    
    /// Get bucket capacity based on rate limit
    pub fn bucket_capacity(&self, rate_limit: u32) -> u32 {
        rate_limit * self.bucket_capacity_multiplier
    }
}