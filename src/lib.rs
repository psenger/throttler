//! Throttler - A lightweight Rust web API rate limiting service
//!
//! This crate provides rate limiting and request throttling capabilities
//! with Redis-backed distributed storage and token bucket algorithms.

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

pub use config::Config;
pub use error::ThrottlerError;
pub use key_generator::{KeyGenerator, KeyStrategy};
pub use rate_limiter::RateLimiter;
pub use server::Server;
pub use throttler::Throttler;