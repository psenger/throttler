use crate::error::{ThrottlerError, Result};
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RequestValidator {
    key_pattern: Regex,
    max_key_length: usize,
    max_requests_per_window: u64,
    min_window_ms: u64,
    max_window_ms: u64,
}

impl Default for RequestValidator {
    fn default() -> Self {
        Self {
            key_pattern: Regex::new(r"^[a-zA-Z0-9_.-]+$").unwrap(),
            max_key_length: 256,
            max_requests_per_window: 10000,
            min_window_ms: 1000,     // 1 second minimum
            max_window_ms: 3600000,  // 1 hour maximum
        }
    }
}

impl RequestValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate_key(&self, key: &str) -> Result<()> {
        if key.is_empty() {
            return Err(ThrottlerError::InvalidKey("Key cannot be empty".to_string()));
        }

        if key.len() > self.max_key_length {
            return Err(ThrottlerError::InvalidKey(
                format!("Key length exceeds maximum of {} characters", self.max_key_length)
            ));
        }

        if !self.key_pattern.is_match(key) {
            return Err(ThrottlerError::InvalidKey(
                "Key contains invalid characters. Only alphanumeric, underscore, dot, and dash allowed".to_string()
            ));
        }

        Ok(())
    }

    pub fn validate_rate_limit(&self, requests: u64, window_ms: u64) -> Result<()> {
        if requests == 0 {
            return Err(ThrottlerError::ValidationError(
                "Requests per window must be greater than 0".to_string()
            ));
        }

        if requests > self.max_requests_per_window {
            return Err(ThrottlerError::ValidationError(
                format!("Requests per window exceeds maximum of {}", self.max_requests_per_window)
            ));
        }

        if window_ms < self.min_window_ms {
            return Err(ThrottlerError::ValidationError(
                format!("Window duration must be at least {}ms", self.min_window_ms)
            ));
        }

        if window_ms > self.max_window_ms {
            return Err(ThrottlerError::ValidationError(
                format!("Window duration cannot exceed {}ms", self.max_window_ms)
            ));
        }

        Ok(())
    }

    pub fn validate_headers(&self, headers: &HashMap<String, String>) -> Result<()> {
        for (name, value) in headers {
            if name.is_empty() {
                return Err(ThrottlerError::ValidationError(
                    "Header name cannot be empty".to_string()
                ));
            }

            if value.len() > 1024 {
                return Err(ThrottlerError::ValidationError(
                    format!("Header '{}' value exceeds 1024 characters", name)
                ));
            }

            // Basic header name validation (simplified HTTP header name rules)
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(ThrottlerError::ValidationError(
                    format!("Header name '{}' contains invalid characters", name)
                ));
            }
        }

        Ok(())
    }

    pub fn sanitize_key(&self, key: &str) -> String {
        key.chars()
            .take(self.max_key_length)
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '-')
            .collect()
    }

    pub fn validate_ip_address(&self, ip: &str) -> Result<()> {
        use std::net::IpAddr;
        
        ip.parse::<IpAddr>()
            .map_err(|_| ThrottlerError::ValidationError(
                format!("Invalid IP address format: {}", ip)
            ))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_key() {
        let validator = RequestValidator::new();
        assert!(validator.validate_key("user123").is_ok());
        assert!(validator.validate_key("api-key_v1.2").is_ok());
    }

    #[test]
    fn test_invalid_key() {
        let validator = RequestValidator::new();
        assert!(validator.validate_key("").is_err());
        assert!(validator.validate_key("key with spaces").is_err());
        assert!(validator.validate_key(&"a".repeat(300)).is_err());
    }

    #[test]
    fn test_valid_rate_limit() {
        let validator = RequestValidator::new();
        assert!(validator.validate_rate_limit(100, 60000).is_ok());
    }

    #[test]
    fn test_invalid_rate_limit() {
        let validator = RequestValidator::new();
        assert!(validator.validate_rate_limit(0, 60000).is_err());
        assert!(validator.validate_rate_limit(100, 500).is_err());
        assert!(validator.validate_rate_limit(20000, 60000).is_err());
    }
}
