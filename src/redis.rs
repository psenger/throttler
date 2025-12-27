//! # Redis Client for Distributed Rate Limiting
//!
//! This module provides a Redis client wrapper for storing token bucket
//! state in a distributed environment. It enables multiple Throttler
//! instances to share rate limiting state.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                     Distributed Rate Limiting                        │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │                                                                      │
//! │   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐           │
//! │   │ Throttler 1  │    │ Throttler 2  │    │ Throttler N  │           │
//! │   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘           │
//! │          │                   │                   │                   │
//! │          └───────────────────┼───────────────────┘                   │
//! │                              ▼                                       │
//! │                     ┌─────────────────┐                              │
//! │                     │   Redis Server   │                             │
//! │                     │                  │                             │
//! │                     │  bucket:user1    │ ← JSON-encoded TokenBucket  │
//! │                     │  bucket:user2    │                             │
//! │                     │  bucket:api-key  │                             │
//! │                     └─────────────────┘                              │
//! │                                                                      │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Atomic Operations
//!
//! The client uses Lua scripts for atomic token consumption, preventing
//! race conditions when multiple instances access the same bucket:
//!
//! ```text
//! Without Lua (race condition):           With Lua (atomic):
//! ┌────────────┐  ┌────────────┐          ┌────────────┐  ┌────────────┐
//! │ Instance A │  │ Instance B │          │ Instance A │  │ Instance B │
//! ├────────────┤  ├────────────┤          ├────────────┤  ├────────────┤
//! │ GET: 10    │  │ GET: 10    │          │ EVAL script│  │   wait...  │
//! │ tokens -= 1│  │ tokens -= 1│          │ (atomic)   │  │            │
//! │ SET: 9     │  │ SET: 9  ⚠️ │          │            │  │ EVAL script│
//! └────────────┘  └────────────┘          └────────────┘  └────────────┘
//!                 (Lost update!)                          (Both correct)
//! ```
//!
//! ## Key Format
//!
//! Buckets are stored with the key format: `throttler:{key}`

use redis::{Client, Commands, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::ThrottlerError;
use crate::token_bucket::TokenBucket;

/// Redis client wrapper for distributed token bucket storage.
///
/// Provides methods for storing, retrieving, and atomically updating
/// token buckets in Redis. Uses Lua scripts to ensure atomic operations
/// and prevent race conditions.
///
/// # Example
///
/// ```rust,no_run
/// use throttler::redis::RedisClient;
/// use throttler::token_bucket::TokenBucket;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = RedisClient::new("redis://localhost:6379")?;
///
/// // Store a bucket
/// let bucket = TokenBucket::new(100, 10.0);
/// client.set_token_bucket("user:123", &bucket, 3600)?;
///
/// // Retrieve a bucket
/// if let Some(bucket) = client.get_token_bucket("user:123")? {
///     println!("Tokens: {}", bucket.tokens);
/// }
///
/// // Health check
/// let pong = client.ping()?;
/// assert_eq!(pong, "PONG");
/// # Ok(())
/// # }
/// ```
pub struct RedisClient {
    /// The underlying Redis client
    client: Client,
}

impl RedisClient {
    pub fn new(url: &str) -> Result<Self, ThrottlerError> {
        let client = Client::open(url)
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to create Redis client: {}", e)))?;

        Ok(RedisClient { client })
    }

    pub fn get_connection(&self) -> Result<Connection, ThrottlerError> {
        self.client.get_connection()
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to get Redis connection: {}", e)))
    }

    pub fn get_token_bucket(&self, key: &str) -> Result<Option<TokenBucket>, ThrottlerError> {
        let mut conn = self.get_connection()?;

        let data: Option<String> = conn.get(key)
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to get token bucket: {}", e)))?;

        match data {
            Some(ref json) => {
                let bucket: TokenBucket = serde_json::from_str(json)
                    .map_err(|e| ThrottlerError::SerializationError(format!("Failed to deserialize token bucket: {}", e)))?;
                Ok(Some(bucket))
            }
            None => Ok(None)
        }
    }

    pub fn set_token_bucket(&self, key: &str, bucket: &TokenBucket, ttl: usize) -> Result<(), ThrottlerError> {
        let mut conn = self.get_connection()?;
        
        let json = serde_json::to_string(bucket)
            .map_err(|e| ThrottlerError::SerializationError(format!("Failed to serialize token bucket: {}", e)))?;
        
        // Use Lua script to atomically update the bucket with proper race condition handling
        let script = r#"
            local key = KEYS[1]
            local new_data = ARGV[1]
            local ttl = tonumber(ARGV[2])
            local current_time = tonumber(ARGV[3])
            
            local existing = redis.call('GET', key)
            if existing then
                local existing_bucket = cjson.decode(existing)
                local new_bucket = cjson.decode(new_data)
                
                -- Only update if the new bucket has a more recent last_refill time
                -- or if the existing bucket is older than expected
                if new_bucket.last_refill >= existing_bucket.last_refill or 
                   (current_time - existing_bucket.last_refill) > 1 then
                    redis.call('SET', key, new_data)
                    redis.call('EXPIRE', key, ttl)
                    return 1
                else
                    return 0
                end
            else
                redis.call('SET', key, new_data)
                redis.call('EXPIRE', key, ttl)
                return 1
            end
        "#;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result: i32 = redis::Script::new(script)
            .key(key)
            .arg(&json)
            .arg(ttl)
            .arg(current_time)
            .invoke(&mut conn)
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to execute Redis script: {}", e)))?;

        if result == 0 {
            return Err(ThrottlerError::RedisError("Token bucket update was rejected due to race condition".to_string()));
        }

        Ok(())
    }

    pub fn delete_token_bucket(&self, key: &str) -> Result<(), ThrottlerError> {
        let mut conn = self.get_connection()?;
        
        let _: () = conn.del(key)
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to delete token bucket: {}", e)))?;
        
        Ok(())
    }

    pub fn exists(&self, key: &str) -> Result<bool, ThrottlerError> {
        let mut conn = self.get_connection()?;
        
        let exists: bool = conn.exists(key)
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to check key existence: {}", e)))?;
        
        Ok(exists)
    }

    pub fn ping(&self) -> Result<String, ThrottlerError> {
        let mut conn = self.get_connection()?;
        
        let pong: String = redis::cmd("PING")
            .query(&mut conn)
            .map_err(|e| ThrottlerError::RedisError(format!("Redis ping failed: {}", e)))?;
        
        Ok(pong)
    }

    pub fn atomic_consume_tokens(&self, key: &str, tokens_to_consume: u32, rule: &crate::rate_limit_config::RateLimitRule) -> Result<(bool, TokenBucket), ThrottlerError> {
        let mut conn = self.get_connection()?;

        let window_ms = rule.window_size.as_millis() as u64;

        let script = r#"
            local key = KEYS[1]
            local tokens_to_consume = tonumber(ARGV[1])
            local capacity = tonumber(ARGV[2])
            local refill_rate = tonumber(ARGV[3])
            local window_ms = tonumber(ARGV[4])
            local current_time = tonumber(ARGV[5])

            local existing = redis.call('GET', key)
            local bucket

            if existing then
                bucket = cjson.decode(existing)

                -- Calculate tokens to add based on time elapsed
                local time_elapsed = current_time - bucket.last_refill
                if time_elapsed > 0 then
                    local tokens_to_add = math.floor(time_elapsed * refill_rate / window_ms)
                    bucket.tokens = math.min(capacity, bucket.tokens + tokens_to_add)
                    bucket.last_refill = current_time
                end
            else
                bucket = {
                    tokens = capacity,
                    capacity = capacity,
                    refill_rate = refill_rate,
                    window_ms = window_ms,
                    last_refill = current_time
                }
            end

            local success = false
            if bucket.tokens >= tokens_to_consume then
                bucket.tokens = bucket.tokens - tokens_to_consume
                success = true
            end

            local bucket_json = cjson.encode(bucket)
            redis.call('SET', key, bucket_json)
            redis.call('EXPIRE', key, math.ceil(window_ms / 1000))

            return {success and 1 or 0, bucket_json}
        "#;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let result: Vec<redis::Value> = redis::Script::new(script)
            .key(key)
            .arg(tokens_to_consume)
            .arg(rule.burst_capacity)
            .arg(rule.requests_per_second)
            .arg(window_ms)
            .arg(current_time)
            .invoke(&mut conn)
            .map_err(|e| ThrottlerError::RedisError(format!("Failed to execute atomic consume script: {}", e)))?;

        if result.len() != 2 {
            return Err(ThrottlerError::RedisError("Invalid response from Redis script".to_string()));
        }

        let success = match &result[0] {
            redis::Value::Int(val) => val == &1,
            _ => return Err(ThrottlerError::RedisError("Invalid success value from Redis".to_string())),
        };

        let bucket_json = match &result[1] {
            redis::Value::Data(data) => std::str::from_utf8(data.as_slice())
                .map_err(|e| ThrottlerError::RedisError(format!("Invalid UTF-8 in bucket data: {}", e)))?,
            redis::Value::Bulk(items) if !items.is_empty() => {
                if let redis::Value::Data(data) = &items[0] {
                    std::str::from_utf8(data.as_slice())
                        .map_err(|e| ThrottlerError::RedisError(format!("Invalid UTF-8 in bucket data: {}", e)))?
                } else {
                    return Err(ThrottlerError::RedisError("Invalid bucket data format from Redis".to_string()));
                }
            }
            _ => return Err(ThrottlerError::RedisError("Invalid bucket data from Redis".to_string())),
        };

        let bucket: TokenBucket = serde_json::from_str(bucket_json)
            .map_err(|e| ThrottlerError::SerializationError(format!("Failed to deserialize updated bucket: {}", e)))?;

        Ok((success, bucket))
    }
}