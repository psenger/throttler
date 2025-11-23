//! Key generation utilities for rate limiting.

use crate::error::ThrottlerError;
use std::collections::HashMap;

/// Strategy for generating rate limit keys
#[derive(Debug, Clone, PartialEq)]
pub enum KeyStrategy {
    /// Use client IP address
    IpAddress,
    /// Use API key from header
    ApiKey,
    /// Use user ID from token/auth
    UserId,
    /// Composite key from multiple sources
    Composite(Vec<KeyStrategy>),
}

/// Generates rate limiting keys based on request context
pub struct KeyGenerator {
    default_strategy: KeyStrategy,
}

impl KeyGenerator {
    pub fn new(strategy: KeyStrategy) -> Self {
        Self {
            default_strategy: strategy,
        }
    }

    /// Generate a rate limit key from request headers and metadata
    pub fn generate_key(
        &self,
        headers: &HashMap<String, String>,
        client_ip: &str,
        path: &str,
    ) -> Result<String, ThrottlerError> {
        self.generate_key_with_strategy(&self.default_strategy, headers, client_ip, path)
    }

    /// Generate key using a specific strategy
    pub fn generate_key_with_strategy(
        &self,
        strategy: &KeyStrategy,
        headers: &HashMap<String, String>,
        client_ip: &str,
        path: &str,
    ) -> Result<String, ThrottlerError> {
        match strategy {
            KeyStrategy::IpAddress => Ok(format!("throttle:ip:{}:{}", client_ip, path)),
            KeyStrategy::ApiKey => {
                let api_key = headers
                    .get("x-api-key")
                    .or_else(|| headers.get("authorization"))
                    .ok_or_else(|| ThrottlerError::MissingApiKey)?;
                Ok(format!("throttle:api:{}:{}", api_key, path))
            }
            KeyStrategy::UserId => {
                let user_id = headers
                    .get("x-user-id")
                    .ok_or_else(|| ThrottlerError::MissingUserId)?;
                Ok(format!("throttle:user:{}:{}", user_id, path))
            }
            KeyStrategy::Composite(strategies) => {
                let mut key_parts = Vec::new();
                for sub_strategy in strategies {
                    let part = match sub_strategy {
                        KeyStrategy::IpAddress => client_ip.to_string(),
                        KeyStrategy::ApiKey => headers
                            .get("x-api-key")
                            .or_else(|| headers.get("authorization"))
                            .ok_or_else(|| ThrottlerError::MissingApiKey)?
                            .clone(),
                        KeyStrategy::UserId => headers
                            .get("x-user-id")
                            .ok_or_else(|| ThrottlerError::MissingUserId)?
                            .clone(),
                        KeyStrategy::Composite(_) => {
                            return Err(ThrottlerError::InvalidKeyStrategy(
                                "Nested composite keys not supported".to_string(),
                            ))
                        }
                    };
                    key_parts.push(part);
                }
                Ok(format!("throttle:composite:{}:{}", key_parts.join(":"), path))
            }
        }
    }

    /// Extract client IP from various header sources
    pub fn extract_client_ip(headers: &HashMap<String, String>) -> String {
        headers
            .get("x-forwarded-for")
            .and_then(|xff| xff.split(',').next().map(|ip| ip.trim()))
            .or_else(|| headers.get("x-real-ip"))
            .or_else(|| headers.get("cf-connecting-ip"))
            .unwrap_or("unknown")
            .to_string()
    }

    /// Sanitize key components to ensure valid Redis keys
    pub fn sanitize_key(key: &str) -> String {
        key.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ':' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}

impl Default for KeyGenerator {
    fn default() -> Self {
        Self::new(KeyStrategy::IpAddress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_headers() -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "test-api-key".to_string());
        headers.insert("x-user-id".to_string(), "user123".to_string());
        headers.insert("x-forwarded-for".to_string(), "192.168.1.1, 10.0.0.1".to_string());
        headers
    }

    #[test]
    fn test_ip_address_strategy() {
        let generator = KeyGenerator::new(KeyStrategy::IpAddress);
        let headers = create_test_headers();
        let key = generator.generate_key(&headers, "192.168.1.1", "/api/test").unwrap();
        assert_eq!(key, "throttle:ip:192.168.1.1:/api/test");
    }

    #[test]
    fn test_api_key_strategy() {
        let generator = KeyGenerator::new(KeyStrategy::ApiKey);
        let headers = create_test_headers();
        let key = generator.generate_key(&headers, "192.168.1.1", "/api/test").unwrap();
        assert_eq!(key, "throttle:api:test-api-key:/api/test");
    }

    #[test]
    fn test_user_id_strategy() {
        let generator = KeyGenerator::new(KeyStrategy::UserId);
        let headers = create_test_headers();
        let key = generator.generate_key(&headers, "192.168.1.1", "/api/test").unwrap();
        assert_eq!(key, "throttle:user:user123:/api/test");
    }

    #[test]
    fn test_composite_strategy() {
        let strategy = KeyStrategy::Composite(vec![KeyStrategy::UserId, KeyStrategy::IpAddress]);
        let generator = KeyGenerator::new(strategy);
        let headers = create_test_headers();
        let key = generator.generate_key(&headers, "192.168.1.1", "/api/test").unwrap();
        assert_eq!(key, "throttle:composite:user123:192.168.1.1:/api/test");
    }

    #[test]
    fn test_extract_client_ip() {
        let headers = create_test_headers();
        let ip = KeyGenerator::extract_client_ip(&headers);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_sanitize_key() {
        let key = "test@key#with$special%chars";
        let sanitized = KeyGenerator::sanitize_key(key);
        assert_eq!(sanitized, "test_key_with_special_chars");
    }
}