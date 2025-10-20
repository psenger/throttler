use crate::error::ThrottlerError;
use crate::redis::RedisClient;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time;

#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    pub max_requests: u32,
    pub window_duration: Duration,
    pub burst_allowance: u32,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            burst_allowance: 10,
        }
    }
}

#[derive(Debug)]
struct ThrottleEntry {
    count: u32,
    window_start: u64,
    last_refill: u64,
    tokens: u32,
}

pub struct DistributedThrottler {
    redis_client: Arc<RedisClient>,
    local_cache: Arc<RwLock<HashMap<String, ThrottleEntry>>>,
    config: ThrottleConfig,
}

impl DistributedThrottler {
    pub fn new(redis_client: Arc<RedisClient>, config: ThrottleConfig) -> Self {
        Self {
            redis_client,
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn check_throttle(&self, key: &str) -> Result<bool, ThrottlerError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ThrottlerError::Internal("System time error".into()))?
            .as_secs();

        // Try Redis first for distributed state
        match self.check_redis_throttle(key, now).await {
            Ok(allowed) => Ok(allowed),
            Err(_) => {
                // Fallback to local cache if Redis is unavailable
                self.check_local_throttle(key, now)
            }
        }
    }

    async fn check_redis_throttle(&self, key: &str, now: u64) -> Result<bool, ThrottlerError> {
        let redis_key = format!("throttle:{}", key);
        let window_size = self.config.window_duration.as_secs();
        let window_start = (now / window_size) * window_size;

        // Lua script for atomic Redis operations
        let lua_script = format!(
            r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local max_requests = tonumber(ARGV[3])
            local burst_allowance = tonumber(ARGV[4])
            
            local current = redis.call('HMGET', key, 'count', 'window_start', 'tokens', 'last_refill')
            local count = tonumber(current[1]) or 0
            local stored_window = tonumber(current[2]) or window_start
            local tokens = tonumber(current[3]) or burst_allowance
            local last_refill = tonumber(current[4]) or now
            
            -- Reset if new window
            if stored_window < window_start then
                count = 0
                stored_window = window_start
            end
            
            -- Refill tokens based on time passed
            local time_passed = now - last_refill
            local refill_rate = max_requests / 60  -- tokens per second
            tokens = math.min(burst_allowance, tokens + (time_passed * refill_rate))
            
            if tokens >= 1 and count < max_requests then
                count = count + 1
                tokens = tokens - 1
                redis.call('HMSET', key, 'count', count, 'window_start', stored_window, 'tokens', tokens, 'last_refill', now)
                redis.call('EXPIRE', key, 300)  -- 5 minute expiry
                return 1
            else
                redis.call('HMSET', key, 'tokens', tokens, 'last_refill', now)
                return 0
            end
            "#
        );

        let result: i32 = self.redis_client.eval_script(
            &lua_script,
            vec![redis_key.as_str()],
            vec![
                &now.to_string(),
                &window_start.to_string(),
                &self.config.max_requests.to_string(),
                &self.config.burst_allowance.to_string(),
            ],
        ).await?;

        Ok(result == 1)
    }

    fn check_local_throttle(&self, key: &str, now: u64) -> Result<bool, ThrottlerError> {
        let mut cache = self.local_cache.write().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire cache lock".into())
        })?;

        let window_size = self.config.window_duration.as_secs();
        let window_start = (now / window_size) * window_size;

        let entry = cache.entry(key.to_string()).or_insert_with(|| ThrottleEntry {
            count: 0,
            window_start,
            last_refill: now,
            tokens: self.config.burst_allowance,
        });

        // Reset if new window
        if entry.window_start < window_start {
            entry.count = 0;
            entry.window_start = window_start;
        }

        // Refill tokens
        let time_passed = now - entry.last_refill;
        let refill_rate = self.config.max_requests as f64 / 60.0;
        entry.tokens = (self.config.burst_allowance as f64)
            .min(entry.tokens as f64 + (time_passed as f64 * refill_rate)) as u32;
        entry.last_refill = now;

        if entry.tokens > 0 && entry.count < self.config.max_requests {
            entry.count += 1;
            entry.tokens -= 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn get_throttle_info(&self, key: &str) -> Result<ThrottleInfo, ThrottlerError> {
        let redis_key = format!("throttle:{}", key);
        
        match self.redis_client.hmget(&redis_key, &["count", "tokens"]).await {
            Ok(values) => {
                let count: u32 = values.get(0)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let tokens: u32 = values.get(1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(self.config.burst_allowance);
                
                Ok(ThrottleInfo {
                    current_count: count,
                    max_requests: self.config.max_requests,
                    remaining_tokens: tokens,
                    window_duration: self.config.window_duration,
                })
            }
            Err(_) => {
                // Fallback to local cache
                let cache = self.local_cache.read().map_err(|_| {
                    ThrottlerError::Internal("Failed to acquire cache lock".into())
                })?;
                
                if let Some(entry) = cache.get(key) {
                    Ok(ThrottleInfo {
                        current_count: entry.count,
                        max_requests: self.config.max_requests,
                        remaining_tokens: entry.tokens,
                        window_duration: self.config.window_duration,
                    })
                } else {
                    Ok(ThrottleInfo {
                        current_count: 0,
                        max_requests: self.config.max_requests,
                        remaining_tokens: self.config.burst_allowance,
                        window_duration: self.config.window_duration,
                    })
                }
            }
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ThrottleInfo {
    pub current_count: u32,
    pub max_requests: u32,
    pub remaining_tokens: u32,
    pub window_duration: Duration,
}