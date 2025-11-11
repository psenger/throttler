use crate::config_validator::ConfigValidator;
use crate::error::ThrottlerError;
use crate::rate_limit_config::RateLimitConfig;
use serde_json::Value;
use std::collections::HashMap;

/// Request validation utilities
pub struct RequestValidator;

impl RequestValidator {
    /// Validates an incoming request body for rate limit configuration
    pub fn validate_create_config_request(body: &Value) -> Result<RateLimitConfig, ThrottlerError> {
        let capacity = body
            .get("capacity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ThrottlerError::ValidationError(
                "Missing or invalid 'capacity' field".to_string(),
            ))? as u32;

        let refill_rate = body
            .get("refill_rate")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ThrottlerError::ValidationError(
                "Missing or invalid 'refill_rate' field".to_string(),
            ))?;

        let window_size_seconds = body
            .get("window_size_seconds")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ThrottlerError::ValidationError(
                "Missing or invalid 'window_size_seconds' field".to_string(),
            ))? as u32;

        let config = RateLimitConfig {
            capacity,
            refill_rate,
            window_size_seconds,
        };

        ConfigValidator::validate_rate_limit_config(&config)?;
        Ok(config)
    }

    /// Validates request parameters
    pub fn validate_request_params(
        key: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<(String, String), ThrottlerError> {
        let key = key
            .ok_or_else(|| ThrottlerError::ValidationError(
                "Missing 'key' parameter".to_string(),
            ))?
            .trim();

        let user_id = user_id
            .ok_or_else(|| ThrottlerError::ValidationError(
                "Missing 'user_id' parameter".to_string(),
            ))?
            .trim();

        if key.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "Rate limit key cannot be empty".to_string(),
            ));
        }

        if user_id.is_empty() {
            return Err(ThrottlerError::ValidationError(
                "User ID cannot be empty".to_string(),
            ));
        }

        // Validate key format (alphanumeric, hyphens, underscores)
        if !key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(ThrottlerError::ValidationError(
                "Rate limit key can only contain alphanumeric characters, hyphens, and underscores".to_string(),
            ));
        }

        // Validate user_id format
        if !user_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '@' || c == '.') {
            return Err(ThrottlerError::ValidationError(
                "User ID contains invalid characters".to_string(),
            ));
        }

        Ok((key.to_string(), user_id.to_string()))
    }

    /// Validates batch configuration update request
    pub fn validate_batch_config_request(
        body: &Value,
    ) -> Result<HashMap<String, RateLimitConfig>, ThrottlerError> {
        let configs_obj = body
            .get("configs")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ThrottlerError::ValidationError(
                "Missing or invalid 'configs' object".to_string(),
            ))?;

        let mut configs = HashMap::new();

        for (key, value) in configs_obj {
            let config = Self::validate_create_config_request(value)?;
            configs.insert(key.clone(), config);
        }

        ConfigValidator::validate_rate_limit_configs(&configs)?;
        Ok(configs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_create_config_request() {
        let body = json!({
            "capacity": 100,
            "refill_rate": 10.0,
            "window_size_seconds": 60
        });

        let result = RequestValidator::validate_create_config_request(&body);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.capacity, 100);
        assert_eq!(config.refill_rate, 10.0);
        assert_eq!(config.window_size_seconds, 60);
    }

    #[test]
    fn test_validate_request_params() {
        let result = RequestValidator::validate_request_params(
            Some("api-key"),
            Some("user123"),
        );
        assert!(result.is_ok());

        let (key, user_id) = result.unwrap();
        assert_eq!(key, "api-key");
        assert_eq!(user_id, "user123");
    }

    #[test]
    fn test_invalid_key_format() {
        let result = RequestValidator::validate_request_params(
            Some("invalid key!"),
            Some("user123"),
        );
        assert!(result.is_err());
    }
}