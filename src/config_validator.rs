use crate::error::ThrottlerError;

/// Validates configuration objects for consistency and correctness
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validates a Redis URL
    pub fn validate_redis_url(url: &str) -> Result<(), ThrottlerError> {
        if url.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "Redis URL cannot be empty".to_string(),
            ));
        }

        if !url.starts_with("redis://") && !url.starts_with("rediss://") {
            return Err(ThrottlerError::ValidationError(
                "Redis URL must start with 'redis://' or 'rediss://'".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates a bind address
    pub fn validate_bind_address(address: &str) -> Result<(), ThrottlerError> {
        if address.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "Bind address cannot be empty".to_string(),
            ));
        }

        // Check if it looks like host:port format
        if !address.contains(':') {
            return Err(ThrottlerError::ValidationError(
                "Bind address must be in host:port format".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates rate limit parameters
    pub fn validate_rate_limit(capacity: u64, refill_rate: u64) -> Result<(), ThrottlerError> {
        if capacity == 0 {
            return Err(ThrottlerError::ValidationError(
                "Rate limit capacity must be greater than 0".to_string(),
            ));
        }

        if refill_rate == 0 {
            return Err(ThrottlerError::ValidationError(
                "Refill rate must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates environment name
    pub fn validate_environment(env: &str) -> Result<(), ThrottlerError> {
        let valid_envs = ["development", "staging", "production", "test"];
        if !valid_envs.contains(&env.to_lowercase().as_str()) {
            return Err(ThrottlerError::ValidationError(
                format!("Invalid environment '{}'. Must be one of: {:?}", env, valid_envs),
            ));
        }

        Ok(())
    }

    /// Validates server configuration parameters
    pub fn validate_server_config(
        host: &str,
        port: u16,
        redis_url: &str,
    ) -> Result<(), ThrottlerError> {
        if host.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "Server host cannot be empty".to_string(),
            ));
        }

        if port == 0 {
            return Err(ThrottlerError::ValidationError(
                "Server port must be greater than 0".to_string(),
            ));
        }

        Self::validate_redis_url(redis_url)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_redis_url() {
        assert!(ConfigValidator::validate_redis_url("redis://localhost:6379").is_ok());
        assert!(ConfigValidator::validate_redis_url("rediss://localhost:6379").is_ok());
    }

    #[test]
    fn test_invalid_redis_url() {
        assert!(ConfigValidator::validate_redis_url("").is_err());
        assert!(ConfigValidator::validate_redis_url("http://localhost:6379").is_err());
    }

    #[test]
    fn test_valid_bind_address() {
        assert!(ConfigValidator::validate_bind_address("127.0.0.1:8080").is_ok());
        assert!(ConfigValidator::validate_bind_address("0.0.0.0:3000").is_ok());
    }

    #[test]
    fn test_invalid_bind_address() {
        assert!(ConfigValidator::validate_bind_address("").is_err());
        assert!(ConfigValidator::validate_bind_address("localhost").is_err());
    }

    #[test]
    fn test_valid_rate_limit() {
        assert!(ConfigValidator::validate_rate_limit(100, 10).is_ok());
    }

    #[test]
    fn test_invalid_rate_limit() {
        assert!(ConfigValidator::validate_rate_limit(0, 10).is_err());
        assert!(ConfigValidator::validate_rate_limit(100, 0).is_err());
    }

    #[test]
    fn test_valid_environment() {
        assert!(ConfigValidator::validate_environment("development").is_ok());
        assert!(ConfigValidator::validate_environment("production").is_ok());
    }

    #[test]
    fn test_invalid_environment() {
        assert!(ConfigValidator::validate_environment("invalid").is_err());
    }

    #[test]
    fn test_valid_server_config() {
        assert!(ConfigValidator::validate_server_config(
            "127.0.0.1",
            8080,
            "redis://localhost:6379"
        ).is_ok());
    }

    #[test]
    fn test_invalid_server_config() {
        assert!(ConfigValidator::validate_server_config(
            "127.0.0.1",
            8080,
            "invalid://localhost:6379"
        ).is_err());
    }
}
