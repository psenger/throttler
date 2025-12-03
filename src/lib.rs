//! Throttler - A lightweight Rust web API rate limiting service
//!
//! This library provides rate limiting and request throttling capabilities
//! with support for multiple algorithms, Redis-backed distributed throttling,
//! and a RESTful configuration API.

pub mod algorithms;
pub mod config;
pub mod config_validator;
pub mod error;
pub mod handlers;
pub mod health;
pub mod key_generator;
pub mod metrics;
pub mod middleware;
pub mod rate_limit_config;
pub mod rate_limiter;
pub mod redis;
pub mod response;
pub mod server;
pub mod throttler;
pub mod token_bucket;
pub mod validation;

// Re-export commonly used types
pub use algorithms::{AlgorithmConfig, AlgorithmState, RateLimitAlgorithm};
pub use config::Config;
pub use rate_limit_config::{RateLimitConfig, RateLimitRule};
pub use error::ThrottlerError;
pub use rate_limiter::RateLimiter;
pub use throttler::Throttler;

/// Result type alias for throttler operations
pub type Result<T> = std::result::Result<T, ThrottlerError>;

/// Version of the throttler library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
