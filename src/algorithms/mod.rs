//! Rate limiting algorithms module
//!
//! This module contains different rate limiting algorithm implementations
//! that can be used by the throttler service.

// Note: sliding_window requires Redis async features not currently configured
// pub mod sliding_window;

use crate::error::ThrottlerError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Re-export the token bucket from the crate root
pub use crate::token_bucket::TokenBucket;

/// Configuration for rate limiting algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    pub capacity: u64,
    pub refill_rate: u64,
    #[serde(with = "humantime_serde")]
    pub window_size: Duration,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            refill_rate: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

/// Trait for rate limiting algorithms
pub trait RateLimitAlgorithm: Send + Sync {
    /// Check if a request should be allowed
    fn is_allowed(&self, key: &str, tokens: u64) -> Result<bool, ThrottlerError>;

    /// Get the current state of the rate limiter for a key
    fn get_state(&self, key: &str) -> Result<AlgorithmState, ThrottlerError>;

    /// Reset the rate limiter for a key
    fn reset(&self, key: &str) -> Result<(), ThrottlerError>;
}

/// Current state of a rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmState {
    pub available_tokens: u64,
    pub last_refill: u64,
    pub requests_in_window: u64,
}
