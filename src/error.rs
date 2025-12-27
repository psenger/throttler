//! # Error Types and Handling
//!
//! This module defines the custom error types for Throttler and their
//! automatic conversion to HTTP responses.
//!
//! ## Error to HTTP Status Mapping
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    Error → HTTP Status Mapping                          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ThrottlerError Variant      │  HTTP Status        │  Response Type    │
//! │  ────────────────────────────┼─────────────────────┼───────────────────│
//! │  RateLimitExceeded           │  429 Too Many Reqs  │  + Retry-After    │
//! │  ValidationError             │  400 Bad Request    │  JSON error       │
//! │  InvalidKey                  │  400 Bad Request    │  JSON error       │
//! │  ConfigError                 │  400 Bad Request    │  JSON error       │
//! │  RedisError                  │  500 Internal Error │  Generic error    │
//! │  SerializationError          │  500 Internal Error │  Generic error    │
//! │  InternalError               │  500 Internal Error │  Generic error    │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Automatic Conversions
//!
//! The error type implements `From` for automatic conversion:
//! - `redis::RedisError` → `ThrottlerError::RedisError`
//! - `serde_json::Error` → `ThrottlerError::SerializationError`
//!
//! ## Axum Integration
//!
//! Implements `IntoResponse` for seamless use with Axum handlers:
//!
//! ```rust,ignore
//! async fn handler() -> Result<impl IntoResponse, ThrottlerError> {
//!     // Errors automatically convert to appropriate HTTP responses
//!     Err(ThrottlerError::ValidationError("Invalid key".to_string()))
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::fmt;

/// Custom error type for all Throttler operations.
///
/// This enum represents all possible errors that can occur in the Throttler
/// service. Each variant automatically maps to an appropriate HTTP status
/// code when converted to an Axum response.
///
/// # Example
///
/// ```rust
/// use throttler::error::ThrottlerError;
///
/// // Create a validation error
/// let err = ThrottlerError::ValidationError("Key too long".to_string());
///
/// // Create a rate limit error
/// let err = ThrottlerError::RateLimitExceeded {
///     retry_after: 60,
///     limit: 100,
///     window_ms: 60000,
/// };
/// ```
#[derive(Debug, Clone)]
pub enum ThrottlerError {
    /// Redis operation failed (connection, command, etc.)
    /// Maps to: 500 Internal Server Error
    RedisError(String),

    /// Configuration is invalid or missing
    /// Maps to: 400 Bad Request
    ConfigError(String),

    /// Request validation failed (parameters out of range, etc.)
    /// Maps to: 400 Bad Request
    ValidationError(String),

    /// Rate limit was exceeded for the requested key
    /// Maps to: 429 Too Many Requests (with Retry-After header)
    RateLimitExceeded {
        /// Seconds until more tokens are available
        retry_after: u64,
        /// Maximum allowed requests
        limit: u64,
        /// Window size in milliseconds
        window_ms: u64,
    },

    /// Unexpected internal error
    /// Maps to: 500 Internal Server Error
    InternalError(String),

    /// Rate limit key format is invalid
    /// Maps to: 400 Bad Request
    InvalidKey(String),

    /// JSON serialization/deserialization failed
    /// Maps to: 500 Internal Server Error
    SerializationError(String),
}

impl std::error::Error for ThrottlerError {}

impl fmt::Display for ThrottlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThrottlerError::RedisError(msg) => write!(f, "Redis error: {}", msg),
            ThrottlerError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ThrottlerError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ThrottlerError::RateLimitExceeded { retry_after, limit, window_ms } => {
                write!(f, "Rate limit exceeded: {} requests per {}ms window. Retry after {}s",
                       limit, window_ms, retry_after)
            },
            ThrottlerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            ThrottlerError::InvalidKey(key) => write!(f, "Invalid key format: {}", key),
            ThrottlerError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl IntoResponse for ThrottlerError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            ThrottlerError::RateLimitExceeded { retry_after, limit, window_ms } => {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    serde_json::json!({
                        "error": "rate_limit_exceeded",
                        "message": self.to_string(),
                        "retry_after_seconds": retry_after,
                        "limit": limit,
                        "window_ms": window_ms
                    })
                )
            },
            ThrottlerError::ValidationError(_) | ThrottlerError::InvalidKey(_) => {
                (
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({
                        "error": "validation_error",
                        "message": self.to_string()
                    })
                )
            },
            ThrottlerError::ConfigError(_) => {
                (
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({
                        "error": "configuration_error",
                        "message": self.to_string()
                    })
                )
            },
            _ => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({
                        "error": "internal_error",
                        "message": "An unexpected error occurred"
                    })
                )
            }
        };

        let mut response = (status, Json(body)).into_response();

        // Add Retry-After header for rate limit errors
        if let ThrottlerError::RateLimitExceeded { retry_after, limit, window_ms } = &self {
            let headers = response.headers_mut();
            if let Ok(val) = retry_after.to_string().parse() {
                headers.insert("Retry-After", val);
            }
            if let Ok(val) = limit.to_string().parse() {
                headers.insert("X-RateLimit-Limit", val);
            }
            if let Ok(val) = window_ms.to_string().parse() {
                headers.insert("X-RateLimit-Window", val);
            }
        }

        response
    }
}

impl From<redis::RedisError> for ThrottlerError {
    fn from(err: redis::RedisError) -> Self {
        ThrottlerError::RedisError(err.to_string())
    }
}

impl From<serde_json::Error> for ThrottlerError {
    fn from(err: serde_json::Error) -> Self {
        ThrottlerError::SerializationError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ThrottlerError>;
pub type ThrottlerResult<T> = std::result::Result<T, ThrottlerError>;
