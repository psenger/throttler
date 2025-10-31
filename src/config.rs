use crate::error::ThrottlerError;
use crate::validation::ConfigValidator;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub bind_address: String,
    pub default_capacity: u64,
    pub default_refill_rate: u64,
    pub environment: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ThrottlerError> {
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        
        let bind_address = env::var("BIND_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        
        let default_capacity = env::var("DEFAULT_CAPACITY")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .map_err(|_| ThrottlerError::ConfigError(
                "Invalid DEFAULT_CAPACITY value".to_string()
            ))?;
        
        let default_refill_rate = env::var("DEFAULT_REFILL_RATE")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .map_err(|_| ThrottlerError::ConfigError(
                "Invalid DEFAULT_REFILL_RATE value".to_string()
            ))?;
        
        let environment = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string());
        
        let log_level = env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());
        
        let config = Config {
            redis_url,
            bind_address,
            default_capacity,
            default_refill_rate,
            environment,
            log_level,
        };
        
        config.validate()?;
        Ok(config)
    }
    
    /// Validates all configuration values
    pub fn validate(&self) -> Result<(), ThrottlerError> {
        ConfigValidator::validate_redis_url(&self.redis_url)?;
        ConfigValidator::validate_bind_address(&self.bind_address)?;
        ConfigValidator::validate_rate_limit(self.default_capacity, self.default_refill_rate)?;
        ConfigValidator::validate_environment(&self.environment)?;
        
        Ok(())
    }
    
    /// Returns true if running in production environment
    pub fn is_production(&self) -> bool {
        self.environment.to_lowercase() == "production"
    }
    
    /// Returns true if running in development environment
    pub fn is_development(&self) -> bool {
        self.environment.to_lowercase() == "development"
    }
}