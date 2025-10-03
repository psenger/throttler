use crate::redis_client::RedisClient;
use anyhow::Result;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub key: String,
    pub tokens: u32,
    pub capacity: u32,
    pub refill_rate: u32,
    pub last_refill: i64,
}

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub tokens_remaining: u32,
    pub retry_after: Option<u32>,
    pub reset_time: i64,
}

pub struct RateLimiter {
    redis: Arc<RedisClient>,
}

impl RateLimiter {
    pub fn new(redis: Arc<RedisClient>) -> Self {
        Self { redis }
    }

    /// Check if a request should be allowed using token bucket algorithm
    pub async fn check_rate_limit(
        &self,
        key: &str,
        capacity: u32,
        refill_rate: u32,
        tokens_requested: u32,
    ) -> Result<RateLimitResult> {
        let now = chrono::Utc::now().timestamp();
        let bucket_key = format!("throttler:bucket:{}", key);
        
        // Get current bucket state from Redis
        let bucket = self.get_or_create_bucket(&bucket_key, capacity, refill_rate, now).await?;
        
        // Calculate tokens to add based on time elapsed
        let time_elapsed = (now - bucket.last_refill).max(0) as u32;
        let tokens_to_add = (time_elapsed * refill_rate).min(capacity);
        let current_tokens = (bucket.tokens + tokens_to_add).min(capacity);
        
        debug!(
            "Rate limit check for {}: tokens={}, capacity={}, requested={}",
            key, current_tokens, capacity, tokens_requested
        );
        
        let allowed = current_tokens >= tokens_requested;
        let (new_tokens, reset_time) = if allowed {
            (current_tokens - tokens_requested, now + (capacity / refill_rate) as i64)
        } else {
            (current_tokens, now + ((tokens_requested - current_tokens) / refill_rate) as i64)
        };
        
        // Update bucket in Redis
        let updated_bucket = TokenBucket {
            key: bucket.key.clone(),
            tokens: new_tokens,
            capacity,
            refill_rate,
            last_refill: now,
        };
        
        self.save_bucket(&bucket_key, &updated_bucket).await?;
        
        let retry_after = if !allowed {
            Some(((tokens_requested - current_tokens) / refill_rate).max(1))
        } else {
            None
        };
        
        Ok(RateLimitResult {
            allowed,
            tokens_remaining: new_tokens,
            retry_after,
            reset_time,
        })
    }
    
    async fn get_or_create_bucket(
        &self,
        key: &str,
        capacity: u32,
        refill_rate: u32,
        now: i64,
    ) -> Result<TokenBucket> {
        match self.redis.get_bucket(key).await? {
            Some(bucket) => Ok(bucket),
            None => Ok(TokenBucket {
                key: key.to_string(),
                tokens: capacity,
                capacity,
                refill_rate,
                last_refill: now,
            }),
        }
    }
    
    async fn save_bucket(&self, key: &str, bucket: &TokenBucket) -> Result<()> {
        self.redis.save_bucket(key, bucket).await
    }
    
    pub async fn get_stats(&self, key: &str) -> Result<Option<TokenBucket>> {
        let bucket_key = format!("throttler:bucket:{}", key);
        self.redis.get_bucket(&bucket_key).await
    }
}