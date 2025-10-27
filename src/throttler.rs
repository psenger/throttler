use crate::error::ThrottlerError;
use crate::rate_limiter::RateLimiter;
use crate::redis::RedisClient;
use crate::token_bucket::TokenBucket;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Core throttling service that manages rate limiting across different clients
#[derive(Clone)]
pub struct ThrottlerService {
    redis_client: Arc<RedisClient>,
    local_buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    fallback_mode: Arc<Mutex<bool>>,
    last_redis_check: Arc<Mutex<Instant>>,
}

impl ThrottlerService {
    pub fn new(redis_client: RedisClient) -> Self {
        Self {
            redis_client: Arc::new(redis_client),
            local_buckets: Arc::new(Mutex::new(HashMap::new())),
            fallback_mode: Arc::new(Mutex::new(false)),
            last_redis_check: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Check if a request should be throttled
    pub async fn should_throttle(&self, client_id: &str, limit: u64, window_secs: u64) -> Result<bool, ThrottlerError> {
        // Check if we should attempt Redis connection recovery
        self.check_redis_recovery().await;

        let is_fallback = *self.fallback_mode.lock().unwrap();
        
        if is_fallback {
            self.throttle_local(client_id, limit, window_secs)
        } else {
            match self.throttle_redis(client_id, limit, window_secs).await {
                Ok(result) => Ok(result),
                Err(_) => {
                    // Redis failed, switch to fallback mode
                    *self.fallback_mode.lock().unwrap() = true;
                    log::warn!("Redis connection failed, switching to local fallback mode for client: {}", client_id);
                    self.throttle_local(client_id, limit, window_secs)
                }
            }
        }
    }

    /// Throttle using Redis backend
    async fn throttle_redis(&self, client_id: &str, limit: u64, window_secs: u64) -> Result<bool, ThrottlerError> {
        let key = format!("throttle:{}", client_id);
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let window_start = current_time - (current_time % window_secs);
        let redis_key = format!("{}:{}", key, window_start);
        
        match self.redis_client.incr_with_expiry(&redis_key, window_secs).await {
            Ok(count) => Ok(count > limit),
            Err(e) => {
                log::error!("Redis throttle check failed: {}", e);
                Err(e)
            }
        }
    }

    /// Throttle using local token buckets as fallback
    fn throttle_local(&self, client_id: &str, limit: u64, window_secs: u64) -> Result<bool, ThrottlerError> {
        let mut buckets = self.local_buckets.lock().unwrap();
        
        let bucket = buckets.entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(limit, Duration::from_secs(window_secs)));
        
        Ok(!bucket.try_consume())
    }

    /// Periodically check if Redis connection can be restored
    async fn check_redis_recovery(&self) {
        let mut last_check = self.last_redis_check.lock().unwrap();
        let now = Instant::now();
        
        // Check every 30 seconds
        if now.duration_since(*last_check) > Duration::from_secs(30) {
            *last_check = now;
            drop(last_check);
            
            if *self.fallback_mode.lock().unwrap() {
                // Try to ping Redis
                match self.redis_client.ping().await {
                    Ok(_) => {
                        *self.fallback_mode.lock().unwrap() = false;
                        log::info!("Redis connection restored, switching back from fallback mode");
                    }
                    Err(_) => {
                        log::debug!("Redis still unavailable, continuing in fallback mode");
                    }
                }
            }
        }
    }

    /// Get current service status
    pub fn get_status(&self) -> ServiceStatus {
        let is_fallback = *self.fallback_mode.lock().unwrap();
        let local_client_count = self.local_buckets.lock().unwrap().len();
        
        ServiceStatus {
            fallback_mode: is_fallback,
            local_clients: local_client_count,
        }
    }

    /// Clean up expired local buckets
    pub fn cleanup_expired_buckets(&self) {
        let mut buckets = self.local_buckets.lock().unwrap();
        buckets.retain(|_, bucket| !bucket.is_expired());
    }
}

#[derive(Debug)]
pub struct ServiceStatus {
    pub fallback_mode: bool,
    pub local_clients: usize,
}