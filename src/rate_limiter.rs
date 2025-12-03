use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::config::Config;
use crate::error::ThrottlerError;
use crate::redis::RedisClient;

#[derive(Clone)]
pub struct RateLimiter {
    config: Arc<Config>,
    local_buckets: Arc<RwLock<HashMap<String, LocalBucket>>>,
    redis_client: Option<Arc<RedisClient>>,
}

/// Local bucket state for in-memory rate limiting
#[derive(Clone)]
struct LocalBucket {
    tokens: f64,
    capacity: u64,
    refill_rate: f64,
    last_refill: u64,
}

impl RateLimiter {
    pub fn new(config: Config) -> Result<Self, ThrottlerError> {
        let redis_client = if !config.redis_url.is_empty() {
            Some(Arc::new(RedisClient::new(&config.redis_url)?))
        } else {
            None
        };

        Ok(RateLimiter {
            config: Arc::new(config),
            local_buckets: Arc::new(RwLock::new(HashMap::new())),
            redis_client,
        })
    }

    /// Check rate limit using default configuration
    pub fn check_rate_limit(&self, key: &str) -> Result<(bool, u64), ThrottlerError> {
        let capacity = self.config.default_capacity;
        let refill_rate = self.config.default_refill_rate as f64;

        self.check_rate_limit_with_params(key, capacity, refill_rate)
    }

    /// Check rate limit with specific parameters
    pub fn check_rate_limit_with_params(
        &self,
        key: &str,
        capacity: u64,
        refill_rate: f64,
    ) -> Result<(bool, u64), ThrottlerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut buckets = self.local_buckets.write()
            .map_err(|_| ThrottlerError::InternalError("Failed to acquire write lock on buckets".to_string()))?;

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            LocalBucket {
                tokens: capacity as f64,
                capacity,
                refill_rate,
                last_refill: current_time,
            }
        });

        // Refill tokens based on time elapsed
        let elapsed_ms = current_time.saturating_sub(bucket.last_refill);
        let elapsed_secs = elapsed_ms as f64 / 1000.0;
        let tokens_to_add = bucket.refill_rate * elapsed_secs;
        bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity as f64);
        bucket.last_refill = current_time;

        // Try to consume a token
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok((true, bucket.tokens.floor() as u64))
        } else {
            Ok((false, 0))
        }
    }

    /// Get remaining tokens for a key
    pub fn get_remaining_tokens(&self, key: &str) -> Result<u64, ThrottlerError> {
        let buckets = self.local_buckets.read()
            .map_err(|_| ThrottlerError::InternalError("Failed to acquire read lock on buckets".to_string()))?;

        match buckets.get(key) {
            Some(bucket) => Ok(bucket.tokens.floor() as u64),
            None => Ok(self.config.default_capacity),
        }
    }

    /// Reset rate limit for a specific key
    pub fn reset(&self, key: &str) -> Result<(), ThrottlerError> {
        if let Some(redis_client) = &self.redis_client {
            let redis_key = format!("throttler:{}", key);
            redis_client.delete_token_bucket(&redis_key)?;
        }

        let mut buckets = self.local_buckets.write()
            .map_err(|_| ThrottlerError::InternalError("Failed to acquire write lock on buckets".to_string()))?;
        buckets.remove(key);

        Ok(())
    }

    /// Cleanup expired buckets
    pub fn cleanup_expired_buckets(&self, max_age_ms: u64) -> Result<usize, ThrottlerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut buckets = self.local_buckets.write()
            .map_err(|_| ThrottlerError::InternalError("Failed to acquire write lock on buckets".to_string()))?;

        let initial_count = buckets.len();

        buckets.retain(|_, bucket| {
            current_time - bucket.last_refill < max_age_ms
        });

        let cleaned_count = initial_count - buckets.len();
        Ok(cleaned_count)
    }

    /// Get statistics about the rate limiter
    pub fn get_stats(&self) -> Result<HashMap<String, u64>, ThrottlerError> {
        let mut stats = HashMap::new();

        let buckets = self.local_buckets.read()
            .map_err(|_| ThrottlerError::InternalError("Failed to acquire read lock on buckets".to_string()))?;

        stats.insert("local_buckets".to_string(), buckets.len() as u64);
        stats.insert("redis_enabled".to_string(), if self.redis_client.is_some() { 1 } else { 0 });

        Ok(stats)
    }

    /// Check if Redis is available
    pub fn is_redis_available(&self) -> bool {
        if let Some(redis_client) = &self.redis_client {
            redis_client.ping().is_ok()
        } else {
            false
        }
    }
}
