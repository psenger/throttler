use crate::error::ThrottlerError;
use crate::rate_limit_config::RateLimitConfig;
use std::collections::HashMap;

/// Validates configuration objects for consistency and correctness
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validates a rate limit configuration
    pub fn validate_rate_limit_config(config: &RateLimitConfig) -> Result<(), ThrottlerError> {
        if config.capacity == 0 {
            return Err(ThrottlerError::ValidationError(
                "Rate limit capacity must be greater than 0".to_string(),
            ));
        }

        if config.refill_rate == 0.0 {
            return Err(ThrottlerError::ValidationError(
                "Refill rate must be greater than 0".to_string(),
            ));
        }

        if config.window_size_seconds == 0 {
            return Err(ThrottlerError::ValidationError(
                "Window size must be greater than 0 seconds".to_string(),
            ));
        }

        // Validate that refill rate doesn't exceed capacity per window
        let max_refill_per_window = config.refill_rate * config.window_size_seconds as f64;
        if max_refill_per_window > config.capacity as f64 * 2.0 {
            return Err(ThrottlerError::ValidationError(
                "Refill rate is too high for the given capacity and window size".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates multiple rate limit configurations for conflicts
    pub fn validate_rate_limit_configs(
        configs: &HashMap<String, RateLimitConfig>,
    ) -> Result<(), ThrottlerError> {
        if configs.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "At least one rate limit configuration is required".to_string(),
            ));
        }

        for (key, config) in configs {
            if key.is_empty() {
                return Err(ThrottlerError::ValidationError(
                    "Rate limit configuration key cannot be empty".to_string(),
                ));
            }

            Self::validate_rate_limit_config(config)
                .map_err(|e| ThrottlerError::ValidationError(
                    format!("Invalid configuration for key '{}': {}", key, e)
                ))?;
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

        if redis_url.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "Redis URL cannot be empty".to_string(),
            ));
        }

        // Basic Redis URL format validation
        if !redis_url.starts_with("redis://") && !redis_url.starts_with("rediss://") {
            return Err(ThrottlerError::ValidationError(
                "Redis URL must start with 'redis://' or 'rediss://'".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_rate_limit_config() {
        let config = RateLimitConfig {
            capacity: 100,
            refill_rate: 10.0,
            window_size_seconds: 60,
        };

        assert!(ConfigValidator::validate_rate_limit_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_capacity() {
        let config = RateLimitConfig {
            capacity: 0,
            refill_rate: 10.0,
            window_size_seconds: 60,
        };

        assert!(ConfigValidator::validate_rate_limit_config(&config).is_err());
    }

    #[test]
    fn test_invalid_refill_rate() {
        let config = RateLimitConfig {
            capacity: 100,
            refill_rate: 0.0,
            window_size_seconds: 60,
        };

        assert!(ConfigValidator::validate_rate_limit_config(&config).is_err());
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
    fn test_invalid_redis_url() {
        assert!(ConfigValidator::validate_server_config(
            "127.0.0.1",
            8080,
            "invalid://localhost:6379"
        ).is_err());
    }
}