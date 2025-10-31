use crate::error::ThrottlerError;
use std::net::SocketAddr;
use std::str::FromStr;

/// Configuration validation utilities
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validates a Redis URL format
    pub fn validate_redis_url(url: &str) -> Result<(), ThrottlerError> {
        if !url.starts_with("redis://") && !url.starts_with("rediss://") {
            return Err(ThrottlerError::ConfigError(
                "Redis URL must start with redis:// or rediss://".to_string()
            ));
        }
        
        // Basic URL validation - more thorough validation happens during connection
        if url.len() < 10 {
            return Err(ThrottlerError::ConfigError(
                "Invalid Redis URL format".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validates server bind address
    pub fn validate_bind_address(addr: &str) -> Result<SocketAddr, ThrottlerError> {
        SocketAddr::from_str(addr)
            .map_err(|_| ThrottlerError::ConfigError(
                format!("Invalid bind address: {}", addr)
            ))
    }
    
    /// Validates rate limit parameters
    pub fn validate_rate_limit(capacity: u64, refill_rate: u64) -> Result<(), ThrottlerError> {
        if capacity == 0 {
            return Err(ThrottlerError::ConfigError(
                "Rate limit capacity must be greater than 0".to_string()
            ));
        }
        
        if refill_rate == 0 {
            return Err(ThrottlerError::ConfigError(
                "Rate limit refill rate must be greater than 0".to_string()
            ));
        }
        
        if capacity > 10_000 {
            return Err(ThrottlerError::ConfigError(
                "Rate limit capacity cannot exceed 10,000".to_string()
            ));
        }
        
        if refill_rate > 1_000 {
            return Err(ThrottlerError::ConfigError(
                "Rate limit refill rate cannot exceed 1,000 per second".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validates environment-specific settings
    pub fn validate_environment(env: &str) -> Result<(), ThrottlerError> {
        match env.to_lowercase().as_str() {
            "development" | "staging" | "production" => Ok(()),
            _ => Err(ThrottlerError::ConfigError(
                format!("Invalid environment: {}. Must be one of: development, staging, production", env)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_redis_url_validation() {
        assert!(ConfigValidator::validate_redis_url("redis://localhost:6379").is_ok());
        assert!(ConfigValidator::validate_redis_url("rediss://localhost:6380").is_ok());
        assert!(ConfigValidator::validate_redis_url("http://localhost:6379").is_err());
        assert!(ConfigValidator::validate_redis_url("redis://").is_err());
    }
    
    #[test]
    fn test_bind_address_validation() {
        assert!(ConfigValidator::validate_bind_address("127.0.0.1:8080").is_ok());
        assert!(ConfigValidator::validate_bind_address("0.0.0.0:3000").is_ok());
        assert!(ConfigValidator::validate_bind_address("invalid:address").is_err());
        assert!(ConfigValidator::validate_bind_address("127.0.0.1:99999").is_err());
    }
    
    #[test]
    fn test_rate_limit_validation() {
        assert!(ConfigValidator::validate_rate_limit(100, 10).is_ok());
        assert!(ConfigValidator::validate_rate_limit(0, 10).is_err());
        assert!(ConfigValidator::validate_rate_limit(100, 0).is_err());
        assert!(ConfigValidator::validate_rate_limit(20000, 10).is_err());
        assert!(ConfigValidator::validate_rate_limit(100, 2000).is_err());
    }
    
    #[test]
    fn test_environment_validation() {
        assert!(ConfigValidator::validate_environment("development").is_ok());
        assert!(ConfigValidator::validate_environment("PRODUCTION").is_ok());
        assert!(ConfigValidator::validate_environment("invalid").is_err());
    }
}