//! Error handling for the throttler service.

use std::fmt;

/// Errors that can occur in the throttler service
#[derive(Debug, Clone)]
pub enum ThrottlerError {
    /// Redis connection or operation error
    Redis(String),
    /// Configuration validation error
    Config(String),
    /// Rate limit exceeded
    RateLimitExceeded {
        limit: u32,
        window_seconds: u32,
        retry_after: u32,
    },
    /// Invalid request format or parameters
    BadRequest(String),
    /// Resource not found
    NotFound(String),
    /// Internal server error
    Internal(String),
    /// Serialization/deserialization error
    Serialization(String),
    /// Validation error
    Validation(String),
    /// Health check failure
    HealthCheck(String),
    /// Missing API key in request
    MissingApiKey,
    /// Missing user ID in request
    MissingUserId,
    /// Invalid key generation strategy
    InvalidKeyStrategy(String),
}

impl fmt::Display for ThrottlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThrottlerError::Redis(msg) => write!(f, "Redis error: {}", msg),
            ThrottlerError::Config(msg) => write!(f, "Configuration error: {}", msg),
            ThrottlerError::RateLimitExceeded {
                limit,
                window_seconds,
                retry_after,
            } => write!(
                f,
                "Rate limit exceeded: {} requests per {} seconds, retry after {} seconds",
                limit, window_seconds, retry_after
            ),
            ThrottlerError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ThrottlerError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ThrottlerError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ThrottlerError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            ThrottlerError::Validation(msg) => write!(f, "Validation error: {}", msg),
            ThrottlerError::HealthCheck(msg) => write!(f, "Health check failed: {}", msg),
            ThrottlerError::MissingApiKey => write!(f, "Missing API key in request headers"),
            ThrottlerError::MissingUserId => write!(f, "Missing user ID in request headers"),
            ThrottlerError::InvalidKeyStrategy(msg) => write!(f, "Invalid key strategy: {}", msg),
        }
    }
}

impl std::error::Error for ThrottlerError {}

impl From<redis::RedisError> for ThrottlerError {
    fn from(error: redis::RedisError) -> Self {
        ThrottlerError::Redis(error.to_string())
    }
}

impl From<serde_json::Error> for ThrottlerError {
    fn from(error: serde_json::Error) -> Self {
        ThrottlerError::Serialization(error.to_string())
    }
}