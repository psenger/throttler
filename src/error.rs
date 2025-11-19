use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThrottlerError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Redis connection error: {0}")]
    RedisConnection(#[from] redis::RedisError),
    
    #[error("Redis operation failed: {0}")]
    RedisOperation(String),
    
    #[error("Rate limiter error: {0}")]
    RateLimiter(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Environment error: {0}")]
    Environment(#[from] std::env::VarError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for ThrottlerError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ThrottlerError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ThrottlerError::RedisConnection(_) => (StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable".to_string()),
            ThrottlerError::RedisOperation(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal service error".to_string()),
            ThrottlerError::RateLimiter(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ThrottlerError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ThrottlerError::Serialization(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Data processing error".to_string()),
            ThrottlerError::Environment(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string()),
            ThrottlerError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO operation failed".to_string()),
            ThrottlerError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ThrottlerError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ThrottlerError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "type": self.error_type(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));

        (status, body).into_response()
    }
}

impl ThrottlerError {
    pub fn error_type(&self) -> &'static str {
        match self {
            ThrottlerError::Config(_) => "config_error",
            ThrottlerError::RedisConnection(_) => "redis_connection_error",
            ThrottlerError::RedisOperation(_) => "redis_operation_error",
            ThrottlerError::RateLimiter(_) => "rate_limiter_error",
            ThrottlerError::Validation(_) => "validation_error",
            ThrottlerError::Serialization(_) => "serialization_error",
            ThrottlerError::Environment(_) => "environment_error",
            ThrottlerError::Io(_) => "io_error",
            ThrottlerError::Internal(_) => "internal_error",
            ThrottlerError::NotFound(_) => "not_found",
            ThrottlerError::BadRequest(_) => "bad_request",
        }
    }
    
    pub fn is_redis_related(&self) -> bool {
        matches!(self, ThrottlerError::RedisConnection(_) | ThrottlerError::RedisOperation(_))
    }
}

// Helper function for creating validation errors
pub fn validation_error(message: impl Into<String>) -> ThrottlerError {
    ThrottlerError::Validation(message.into())
}

// Helper function for creating internal errors
pub fn internal_error(message: impl Into<String>) -> ThrottlerError {
    ThrottlerError::Internal(message.into())
}