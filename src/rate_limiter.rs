use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::config::Config;
use crate::error::ThrottlerError;
use crate::rate_limit_config::RateLimitConfig;
use crate::redis::RedisClient;
use crate::token_bucket::TokenBucket;

#[derive(Clone)]
pub struct RateLimiter {
    config: Arc<Config>,
    local_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    redis_client: Option<Arc<RedisClient>>,
}

impl RateLimiter {
    pub fn new(config: Config) -> Result<Self, ThrottlerError> {
        let redis_client = if let Some(redis_url) = &config.redis_url {
            Some(Arc::new(RedisClient::new(redis_url)?))
        } else {
            None
        };

        Ok(RateLimiter {
            config: Arc::new(config),
            local_buckets: Arc::new(RwLock::new(HashMap::new())),
            redis_client,
        })
    }

    pub fn check_rate_limit(&self, key: &str, rule_name: &str) -> Result<(bool, TokenBucket), ThrottlerError> {
        let rate_config = self.config.rate_limits
            .get(rule_name)
            .ok_or_else(|| ThrottlerError::Configuration(format!("Rate limit rule '{}' not found", rule_name)))?;

        self.check_rate_limit_with_config(key, rate_config)
    }

    pub fn check_rate_limit_with_config(&self, key: &str, config: &RateLimitConfig) -> Result<(bool, TokenBucket), ThrottlerError> {
        if let Some(redis_client) = &self.redis_client {
            self.check_distributed_rate_limit(redis_client, key, config)
        } else {
            self.check_local_rate_limit(key, config)
        }
    }

    fn check_distributed_rate_limit(
        &self,
        redis_client: &RedisClient,
        key: &str,
        config: &RateLimitConfig,
    ) -> Result<(bool, TokenBucket), ThrottlerError> {
        let redis_key = format!("throttler:{}:{}", config.name, key);
        
        // Use atomic consume operation to prevent race conditions
        let (allowed, bucket) = redis_client.atomic_consume_tokens(&redis_key, 1, config)?;
        
        // Update local cache with the authoritative bucket state from Redis
        if let Ok(mut buckets) = self.local_buckets.write() {
            buckets.insert(key.to_string(), bucket.clone());
        }
        
        Ok((allowed, bucket))
    }

    fn check_local_rate_limit(
        &self,
        key: &str,
        config: &RateLimitConfig,
    ) -> Result<(bool, TokenBucket), ThrottlerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut buckets = self.local_buckets.write()
            .map_err(|_| ThrottlerError::Internal("Failed to acquire write lock on buckets".to_string()))?;

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(
                config.burst_size,
                config.requests_per_window,
                config.window_ms,
                current_time,
            )
        });

        bucket.refill(current_time);
        let allowed = bucket.try_consume(1);

        Ok((allowed, bucket.clone()))
    }

    pub fn get_bucket_state(&self, key: &str, rule_name: &str) -> Result<Option<TokenBucket>, ThrottlerError> {
        let rate_config = self.config.rate_limits
            .get(rule_name)
            .ok_or_else(|| ThrottlerError::Configuration(format!("Rate limit rule '{}' not found", rule_name)))?;

        if let Some(redis_client) = &self.redis_client {
            let redis_key = format!("throttler:{}:{}", rate_config.name, key);
            redis_client.get_token_bucket(&redis_key)
        } else {
            let buckets = self.local_buckets.read()
                .map_err(|_| ThrottlerError::Internal("Failed to acquire read lock on buckets".to_string()))?;
            Ok(buckets.get(key).cloned())
        }
    }

    pub fn reset_bucket(&self, key: &str, rule_name: &str) -> Result<(), ThrottlerError> {
        let rate_config = self.config.rate_limits
            .get(rule_name)
            .ok_or_else(|| ThrottlerError::Configuration(format!("Rate limit rule '{}' not found", rule_name)))?;

        if let Some(redis_client) = &self.redis_client {
            let redis_key = format!("throttler:{}:{}", rate_config.name, key);
            redis_client.delete_token_bucket(&redis_key)?;
        }

        let mut buckets = self.local_buckets.write()
            .map_err(|_| ThrottlerError::Internal("Failed to acquire write lock on buckets".to_string()))?;
        buckets.remove(key);

        Ok(())
    }

    pub fn cleanup_expired_buckets(&self) -> Result<usize, ThrottlerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut buckets = self.local_buckets.write()
            .map_err(|_| ThrottlerError::Internal("Failed to acquire write lock on buckets".to_string()))?;

        let initial_count = buckets.len();
        
        // Remove buckets that haven't been used for more than their window duration
        buckets.retain(|_, bucket| {
            current_time - bucket.last_refill < bucket.window_ms * 2
        });

        let cleaned_count = initial_count - buckets.len();
        Ok(cleaned_count)
    }

    pub fn get_stats(&self) -> Result<HashMap<String, u64>, ThrottlerError> {
        let mut stats = HashMap::new();
        
        let buckets = self.local_buckets.read()
            .map_err(|_| ThrottlerError::Internal("Failed to acquire read lock on buckets".to_string()))?;
        
        stats.insert("local_buckets".to_string(), buckets.len() as u64);
        stats.insert("redis_enabled".to_string(), if self.redis_client.is_some() { 1 } else { 0 });
        
        Ok(stats)
    }

    pub fn is_redis_available(&self) -> bool {
        if let Some(redis_client) = &self.redis_client {
            redis_client.ping().is_ok()
        } else {
            false
        }
    }
}