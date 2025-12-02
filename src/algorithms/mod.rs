//! Rate limiting algorithms module
//!
//! This module contains different rate limiting algorithm implementations
//! that can be used by the throttler service.

pub mod token_bucket;
pub mod sliding_window;

use crate::error::ThrottlerError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for rate limiting algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    pub capacity: u64,
    pub refill_rate: u64,
    pub window_size: Duration,
}

/// Trait for rate limiting algorithms
#[async_trait]
pub trait RateLimitAlgorithm: Send + Sync {
    /// Check if a request should be allowed
    async fn is_allowed(&self, key: &str, tokens: u64) -> Result<bool, ThrottlerError>;
    
    /// Get the current state of the rate limiter for a key
    async fn get_state(&self, key: &str) -> Result<AlgorithmState, ThrottlerError>;
    
    /// Reset the rate limiter for a key
    async fn reset(&self, key: &str) -> Result<(), ThrottlerError>;
}

/// Current state of a rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmState {
    pub available_tokens: u64,
    pub last_refill: u64,
    pub requests_in_window: u64,
}
