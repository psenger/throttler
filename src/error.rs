use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ThrottlerError {
    RedisError(String),
    ConfigError(String),
    ValidationError(String),
    RateLimitExceeded {
        retry_after: u64,
        limit: u64,
        window_ms: u64,
    },
    InternalError(String),
    InvalidKey(String),
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
