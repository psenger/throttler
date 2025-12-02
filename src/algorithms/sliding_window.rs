//! Sliding window rate limiting algorithm
//!
//! Implements a sliding window counter algorithm for rate limiting.
//! This algorithm tracks the number of requests in a sliding time window.

use super::{AlgorithmConfig, AlgorithmState, RateLimitAlgorithm};
use crate::error::ThrottlerError;
use crate::redis::RedisManager;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Sliding window rate limiter implementation
pub struct SlidingWindowLimiter {
    redis: Arc<RedisManager>,
    config: AlgorithmConfig,
}

impl SlidingWindowLimiter {
    /// Create a new sliding window rate limiter
    pub fn new(redis: Arc<RedisManager>, config: AlgorithmConfig) -> Self {
        Self { redis, config }
    }
    
    /// Get the current timestamp in seconds
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Generate Redis key for request timestamps
    fn timestamps_key(&self, key: &str) -> String {
        format!("throttler:sliding_window:{}:timestamps", key)
    }
}

#[async_trait]
impl RateLimitAlgorithm for SlidingWindowLimiter {
    async fn is_allowed(&self, key: &str, tokens: u64) -> Result<bool, ThrottlerError> {
        let now = self.current_timestamp();
        let window_start = now - self.config.window_size.as_secs();
        let timestamps_key = self.timestamps_key(key);
        
        // Use Redis pipeline for atomic operations
        let mut conn = self.redis.get_connection().await?;
        
        // Remove expired timestamps
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&timestamps_key)
            .arg("-inf")
            .arg(window_start)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        
        // Count current requests in window
        let current_count: u64 = redis::cmd("ZCARD")
            .arg(&timestamps_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        
        if current_count + tokens > self.config.capacity {
            return Ok(false);
        }
        
        // Add current request timestamps
        for i in 0..tokens {
            redis::cmd("ZADD")
                .arg(&timestamps_key)
                .arg(now + i)  // Use slightly different timestamps for multiple tokens
                .arg(now + i)
                .query_async::<_, ()>(&mut conn)
                .await
                .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        }
        
        // Set expiration for cleanup
        redis::cmd("EXPIRE")
            .arg(&timestamps_key)
            .arg(self.config.window_size.as_secs() + 60) // Extra buffer
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        
        Ok(true)
    }
    
    async fn get_state(&self, key: &str) -> Result<AlgorithmState, ThrottlerError> {
        let now = self.current_timestamp();
        let window_start = now - self.config.window_size.as_secs();
        let timestamps_key = self.timestamps_key(key);
        
        let mut conn = self.redis.get_connection().await?;
        
        // Clean up expired timestamps
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&timestamps_key)
            .arg("-inf")
            .arg(window_start)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        
        // Get current request count
        let requests_in_window: u64 = redis::cmd("ZCARD")
            .arg(&timestamps_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        
        Ok(AlgorithmState {
            available_tokens: self.config.capacity.saturating_sub(requests_in_window),
            last_refill: now,
            requests_in_window,
        })
    }
    
    async fn reset(&self, key: &str) -> Result<(), ThrottlerError> {
        let timestamps_key = self.timestamps_key(key);
        let mut conn = self.redis.get_connection().await?;
        
        redis::cmd("DEL")
            .arg(&timestamps_key)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| ThrottlerError::Redis(e.to_string()))?;
        
        Ok(())
    }
}
