//! # Throttler - A Distributed Rate Limiting Service
//!
//! Throttler is a high-performance, Redis-backed rate limiting service written in Rust.
//! It provides distributed rate limiting for APIs with microsecond-level latency.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────┐     ┌──────────────────────────┐     ┌─────────────┐
//! │   Client    │────▶│    Throttler Service     │────▶│    Redis    │
//! │   (APIs)    │     │                          │     │   (State)   │
//! └─────────────┘     └──────────────────────────┘     └─────────────┘
//!                                  │
//!                     ┌────────────┼────────────┐
//!                     ▼            ▼            ▼
//!                ┌─────────┐ ┌──────────┐ ┌──────────┐
//!                │  Token  │ │ Sliding  │ │  Health  │
//!                │  Bucket │ │  Window  │ │  Checks  │
//!                └─────────┘ └──────────┘ └──────────┘
//! ```
//!
//! ## Core Components
//!
//! - **[`Server`](server::Server)** - HTTP server built on Axum with graceful shutdown
//! - **[`Throttler`]** - Main service orchestrator for rate limiting operations
//! - **[`RateLimiter`]** - Core rate limiting engine (local or Redis-backed)
//! - **[`TokenBucket`](token_bucket::TokenBucket)** - Token bucket algorithm implementation
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use throttler::{Config, server::Server};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration from environment
//!     let config = Config::from_env()?;
//!
//!     // Create and run the server
//!     let server = Server::new(config)?;
//!     server.run().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Request Flow
//!
//! 1. HTTP request arrives at the Axum server
//! 2. Handler validates the request key and parameters
//! 3. [`RateLimiter`] checks/consumes tokens from the bucket
//! 4. Response includes rate limit headers (`X-RateLimit-*`)
//! 5. Returns 429 Too Many Requests if limit exceeded
//!
//! ## Storage Modes
//!
//! | Mode        | Use Case                        | State Persistence |
//! |-------------|--------------------------------|-------------------|
//! | Local       | Development, single instance    | In-memory         |
//! | Distributed | Production, multiple instances  | Redis             |
//!
//! ## Module Organization
//!
//! - [`algorithms`] - Pluggable rate limiting algorithms (token bucket, sliding window)
//! - [`config`] - Configuration loading and validation
//! - [`error`] - Custom error types with HTTP status mapping
//! - [`handlers`] - HTTP request handlers for all endpoints
//! - [`rate_limiter`] - Core rate limiting engine
//! - [`redis`] - Redis client wrapper for distributed state
//! - [`server`] - HTTP server setup and routing
//! - [`throttler`] - Service orchestrator
//! - [`token_bucket`] - Token bucket algorithm implementation
//! - [`validation`] - Request input validation

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
