use crate::config::RateLimitConfig;
use crate::error::ThrottlerError;
use crate::token_bucket::TokenBucket;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub type RateLimiterResult<T> = Result<T, ThrottlerError>;

#[derive(Debug)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            default_config: config,
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> RateLimiterResult<bool> {
        self.check_rate_limit_with_tokens(key, 1)
    }

    pub fn check_rate_limit_with_tokens(&self, key: &str, tokens: u32) -> RateLimiterResult<bool> {
        let mut buckets = self.buckets.write().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire write lock".to_string())
        })?;

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(
                self.default_config.burst_size,
                self.default_config.requests_per_second as f64,
            )
        });

        Ok(bucket.try_consume(tokens))
    }

    pub fn get_rate_limit_status(&self, key: &str) -> RateLimiterResult<RateLimitStatus> {
        let mut buckets = self.buckets.write().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire write lock".to_string())
        })?;

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(
                self.default_config.burst_size,
                self.default_config.requests_per_second as f64,
            )
        });

        let available = bucket.available_tokens();
        let reset_time = if available == 0 {
            Some(bucket.time_until_available(1))
        } else {
            None
        };

        Ok(RateLimitStatus {
            limit: self.default_config.burst_size,
            remaining: available,
            reset_after: reset_time,
        })
    }

    pub fn reset_rate_limit(&self, key: &str) -> RateLimiterResult<()> {
        let mut buckets = self.buckets.write().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire write lock".to_string())
        })?;

        if let Some(bucket) = buckets.get_mut(key) {
            bucket.reset();
        }

        Ok(())
    }

    pub fn cleanup_expired(&self) {
        // This would be implemented with actual expiration logic
        // For now, we'll keep all buckets
    }
}

#[derive(Debug)]
pub struct RateLimitStatus {
    pub limit: u32,
    pub remaining: u32,
    pub reset_after: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 10,
            burst_size: 20,
        }
    }

    #[test]
    fn test_rate_limiter_allows_requests() {
        let limiter = RateLimiter::new(test_config());
        assert!(limiter.check_rate_limit("test-key").unwrap());
    }

    #[test]
    fn test_rate_limiter_blocks_excess_requests() {
        let limiter = RateLimiter::new(test_config());
        
        // Consume all tokens
        for _ in 0..20 {
            limiter.check_rate_limit("test-key").unwrap();
        }
        
        // Next request should be blocked
        assert!(!limiter.check_rate_limit("test-key").unwrap());
    }

    #[test]
    fn test_rate_limit_status() {
        let limiter = RateLimiter::new(test_config());
        limiter.check_rate_limit_with_tokens("test-key", 5).unwrap();
        
        let status = limiter.get_rate_limit_status("test-key").unwrap();
        assert_eq!(status.limit, 20);
        assert_eq!(status.remaining, 15);
    }
}