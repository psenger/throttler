//! # Token Bucket Algorithm Implementation
//!
//! This module implements the [token bucket algorithm](https://en.wikipedia.org/wiki/Token_bucket)
//! for rate limiting. The token bucket is a simple, efficient algorithm that allows
//! controlled bursts while maintaining a long-term average rate.
//!
//! ## How It Works
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         TOKEN BUCKET VISUALIZATION                          │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │   Bucket State Over Time (capacity=10, refill_rate=2/sec):                  │
//! │                                                                             │
//! │   Time 0s:  [██████████] 10/10 tokens (full)                                │
//! │             ↓ Request arrives, consumes 1 token                             │
//! │                                                                             │
//! │   Time 0s:  [█████████ ] 9/10 tokens                                        │
//! │             ↓ 3 more requests arrive                                        │
//! │                                                                             │
//! │   Time 0s:  [██████   ] 6/10 tokens                                         │
//! │             ↓ 0.5 seconds pass (refill = 0.5 × 2 = 1 token)                 │
//! │                                                                             │
//! │   Time 0.5s: [███████  ] 7/10 tokens                                        │
//! │              ↓ 10 requests arrive (only 7 allowed)                          │
//! │                                                                             │
//! │   Time 0.5s: [         ] 0/10 tokens (3 requests DENIED)                    │
//! │              ↓ 1 second passes (refill = 1 × 2 = 2 tokens)                  │
//! │                                                                             │
//! │   Time 1.5s: [██       ] 2/10 tokens                                        │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Properties
//!
//! | Property       | Description                                    |
//! |----------------|------------------------------------------------|
//! | **Capacity**   | Maximum tokens (burst size)                    |
//! | **Refill Rate**| Tokens added per second (sustained rate)       |
//! | **Burst**      | Allows up to `capacity` requests instantly     |
//! | **Fairness**   | Each key has independent bucket                |
//!
//! ## Edge Case Handling
//!
//! The implementation handles several edge cases:
//! - **Overflow prevention**: Elapsed time capped at 1 hour max
//! - **NaN/Infinity protection**: Validates floating point arithmetic
//! - **Precision**: Uses f64 for fractional token accumulation
//! - **Time skew**: Saturating subtraction prevents underflow

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::error::ThrottlerError;

/// A token bucket for rate limiting with time-based refill.
///
/// The token bucket algorithm allows controlled bursts while maintaining
/// a long-term average rate. Tokens are consumed on each request and
/// refilled at a steady rate over time.
///
/// # Example
///
/// ```rust
/// use throttler::token_bucket::TokenBucket;
///
/// // Create a bucket: 100 tokens max, refill at 10/second
/// let mut bucket = TokenBucket::new(100, 10.0);
///
/// // Consume tokens
/// assert!(bucket.try_consume(1).unwrap());  // Allowed, 99 remaining
/// assert!(bucket.try_consume(99).unwrap()); // Allowed, 0 remaining
/// assert!(!bucket.try_consume(1).unwrap()); // Denied, no tokens
///
/// // After time passes, tokens refill automatically
/// ```
///
/// # Thread Safety
///
/// `TokenBucket` is `Clone` and serializable, but not thread-safe internally.
/// For concurrent access, wrap in `Arc<RwLock<TokenBucket>>` or use the
/// `RateLimiter` which handles synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    /// Maximum number of tokens the bucket can hold
    pub capacity: u64,
    /// Current token count (fractional for precise refill)
    pub tokens: f64,
    /// Rate at which tokens are added (tokens per second)
    pub refill_rate: f64,
    /// Timestamp of last refill calculation (milliseconds since UNIX epoch)
    pub last_refill: u64,
}

impl TokenBucket {
    /// Creates a new token bucket with full capacity.
    ///
    /// The bucket starts with `capacity` tokens and will refill at
    /// `refill_rate` tokens per second.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum tokens the bucket can hold
    /// * `refill_rate` - Tokens added per second
    ///
    /// # Example
    ///
    /// ```rust
    /// use throttler::token_bucket::TokenBucket;
    ///
    /// // 100 requests max, refill at 10 per second
    /// let bucket = TokenBucket::new(100, 10.0);
    /// assert_eq!(bucket.capacity, 100);
    /// assert_eq!(bucket.tokens, 100.0);
    /// ```
    pub fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Self::now_ms(),
        }
    }

    /// Gets the current timestamp in milliseconds since UNIX epoch.
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Attempts to consume tokens from the bucket.
    ///
    /// This is the primary method for rate limiting. It:
    /// 1. Refills tokens based on elapsed time
    /// 2. Checks if enough tokens are available
    /// 3. Consumes tokens if available
    ///
    /// # Arguments
    ///
    /// * `tokens` - Number of tokens to consume
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - Tokens were consumed, request allowed
    /// - `Ok(false)` - Insufficient tokens, request denied
    ///
    /// # Example
    ///
    /// ```rust
    /// use throttler::token_bucket::TokenBucket;
    ///
    /// let mut bucket = TokenBucket::new(10, 1.0);
    ///
    /// assert!(bucket.try_consume(5).unwrap());  // 5 remaining
    /// assert!(bucket.try_consume(5).unwrap());  // 0 remaining
    /// assert!(!bucket.try_consume(1).unwrap()); // Denied
    /// ```
    pub fn try_consume(&mut self, tokens: u64) -> Result<bool, ThrottlerError> {
        // First, add any tokens that have accumulated since last check
        self.refill()?;

        let tokens_f64 = tokens as f64;
        if self.tokens >= tokens_f64 {
            self.tokens -= tokens_f64;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Refills tokens based on elapsed time since last refill.
    ///
    /// This method calculates how many tokens should have been added
    /// based on the time elapsed and refill rate, then updates the
    /// bucket state.
    ///
    /// # Algorithm
    ///
    /// ```text
    /// elapsed_seconds = (now - last_refill) / 1000
    /// tokens_to_add = refill_rate × elapsed_seconds
    /// new_tokens = min(tokens + tokens_to_add, capacity)
    /// ```
    ///
    /// # Edge Cases Handled
    ///
    /// - **Overflow**: Elapsed time capped at 1 hour
    /// - **Precision**: Ignores durations < 1ms
    /// - **NaN/Infinity**: Validates arithmetic results
    pub fn refill(&mut self) -> Result<(), ThrottlerError> {
        let now = Self::now_ms();
        let elapsed_ms = now.saturating_sub(self.last_refill);

        // Cap elapsed time to prevent overflow (1 hour max)
        // This handles cases where system clock jumps or bucket is very stale
        let safe_elapsed_ms = elapsed_ms.min(3600_000);
        let seconds_elapsed = safe_elapsed_ms as f64 / 1000.0;

        // Avoid floating point precision issues with very small durations
        if seconds_elapsed < 0.001 {
            return Ok(());
        }

        let tokens_to_add = self.refill_rate * seconds_elapsed;

        // Ensure we don't exceed capacity and handle potential NaN/infinity
        if tokens_to_add.is_finite() && tokens_to_add > 0.0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
        }

        self.last_refill = now;
        Ok(())
    }

    /// Returns the number of whole tokens currently available.
    ///
    /// Triggers a refill before checking. This method does NOT consume tokens.
    ///
    /// # Returns
    ///
    /// The floor of current tokens (fractional tokens not counted).
    pub fn available_tokens(&mut self) -> Result<u64, ThrottlerError> {
        self.refill()?;
        Ok(self.tokens.floor() as u64)
    }

    /// Calculates time until the specified number of tokens are available.
    ///
    /// Useful for calculating `Retry-After` header values.
    ///
    /// # Arguments
    ///
    /// * `tokens` - Number of tokens needed
    ///
    /// # Returns
    ///
    /// Duration until tokens will be available. Returns `Duration::ZERO`
    /// if tokens are already available.
    ///
    /// # Example
    ///
    /// ```rust
    /// use throttler::token_bucket::TokenBucket;
    /// use std::time::Duration;
    ///
    /// let mut bucket = TokenBucket::new(10, 2.0); // 2 tokens/sec
    /// bucket.tokens = 0.0; // Empty bucket
    ///
    /// let wait = bucket.time_until_tokens(4).unwrap();
    /// // Need 4 tokens at 2/sec = 2 seconds
    /// assert!(wait >= Duration::from_secs(1)); // At least 1 second
    /// ```
    pub fn time_until_tokens(&mut self, tokens: u64) -> Result<Duration, ThrottlerError> {
        self.refill()?;

        let tokens_f64 = tokens as f64;
        if self.tokens >= tokens_f64 {
            return Ok(Duration::from_secs(0));
        }

        let tokens_needed = tokens_f64 - self.tokens;

        // Handle edge case where refill_rate is zero or very small
        if self.refill_rate <= 0.0 {
            return Ok(Duration::from_secs(u64::MAX));
        }

        let seconds_needed = tokens_needed / self.refill_rate;

        // Cap the wait time to prevent overflow (24 hours max)
        let max_wait_seconds = 86400.0;
        let safe_seconds = seconds_needed.min(max_wait_seconds);

        Ok(Duration::from_secs_f64(safe_seconds))
    }

    /// Resets the bucket to full capacity.
    ///
    /// Used for manual reset operations or testing.
    pub fn reset(&mut self) {
        self.tokens = self.capacity as f64;
        self.last_refill = Self::now_ms();
    }

    /// Checks if the bucket is empty (< 1 token).
    ///
    /// Triggers a refill before checking.
    pub fn is_empty(&mut self) -> Result<bool, ThrottlerError> {
        self.refill()?;
        Ok(self.tokens < 1.0)
    }

    /// Returns the bucket utilization as a percentage (0.0 to 1.0).
    ///
    /// - `0.0` = bucket is full
    /// - `1.0` = bucket is empty
    ///
    /// Useful for monitoring and metrics.
    pub fn utilization(&mut self) -> Result<f64, ThrottlerError> {
        self.refill()?;
        Ok(1.0 - (self.tokens / self.capacity as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bucket_has_full_capacity() {
        let bucket = TokenBucket::new(100, 10.0);
        assert_eq!(bucket.capacity, 100);
        assert_eq!(bucket.tokens, 100.0);
    }

    #[test]
    fn test_consume_tokens() {
        let mut bucket = TokenBucket::new(100, 10.0);
        assert!(bucket.try_consume(50).unwrap());
        assert_eq!(bucket.tokens, 50.0);
    }

    #[test]
    fn test_cannot_consume_more_than_available() {
        let mut bucket = TokenBucket::new(10, 1.0);
        assert!(!bucket.try_consume(20).unwrap());
        assert_eq!(bucket.tokens, 10.0);
    }

    #[test]
    fn test_reset() {
        let mut bucket = TokenBucket::new(100, 10.0);
        bucket.tokens = 10.0;
        bucket.reset();
        assert_eq!(bucket.tokens, 100.0);
    }

    #[test]
    fn test_serialization() {
        let bucket = TokenBucket::new(100, 10.0);
        let json = serde_json::to_string(&bucket).unwrap();
        let deserialized: TokenBucket = serde_json::from_str(&json).unwrap();
        assert_eq!(bucket.capacity, deserialized.capacity);
        assert_eq!(bucket.refill_rate, deserialized.refill_rate);
    }
}
