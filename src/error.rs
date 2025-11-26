use actix_web::{HttpResponse, ResponseError};
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

impl ResponseError for ThrottlerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ThrottlerError::RateLimitExceeded { retry_after, limit, window_ms } => {
                HttpResponse::TooManyRequests()
                    .insert_header(("Retry-After", retry_after.to_string()))
                    .insert_header(("X-RateLimit-Limit", limit.to_string()))
                    .insert_header(("X-RateLimit-Window", window_ms.to_string()))
                    .json(serde_json::json!({
                        "error": "rate_limit_exceeded",
                        "message": self.to_string(),
                        "retry_after_seconds": retry_after,
                        "limit": limit,
                        "window_ms": window_ms
                    }))
            },
            ThrottlerError::ValidationError(_) | ThrottlerError::InvalidKey(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "validation_error",
                    "message": self.to_string()
                }))
            },
            ThrottlerError::ConfigError(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "configuration_error",
                    "message": self.to_string()
                }))
            },
            _ => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "internal_error",
                    "message": "An unexpected error occurred"
                }))
            }
        }
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
