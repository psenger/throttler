use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub enum ApiError {
    InvalidRequest(String),
    RateLimitExceeded,
    InternalServerError(String),
    RedisConnectionError(String),
    ConfigurationError(String),
    ValidationError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            ApiError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            ApiError::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
            ApiError::RedisConnectionError(msg) => write!(f, "Redis connection error: {}", msg),
            ApiError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            ApiError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        ApiError::RedisConnectionError(err.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: u16,
}

impl ErrorResponse {
    pub fn new(error: &str, message: &str, code: u16) -> Self {
        Self {
            error: error.to_string(),
            message: message.to_string(),
            code,
        }
    }

    pub fn from_api_error(err: &ApiError) -> Self {
        match err {
            ApiError::InvalidRequest(msg) => Self::new("bad_request", msg, 400),
            ApiError::RateLimitExceeded => Self::new("rate_limit_exceeded", "Request rate limit exceeded", 429),
            ApiError::InternalServerError(msg) => Self::new("internal_error", msg, 500),
            ApiError::RedisConnectionError(msg) => Self::new("service_unavailable", msg, 503),
            ApiError::ConfigurationError(msg) => Self::new("configuration_error", msg, 500),
            ApiError::ValidationError(msg) => Self::new("validation_error", msg, 422),
        }
    }
}
