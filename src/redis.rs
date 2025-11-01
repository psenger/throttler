use crate::error::{ThrottlerError, ThrottlerResult};
use redis::{aio::Connection, AsyncCommands, Client, RedisError};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct RedisClient {
    client: Client,
    connection_timeout: Duration,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> ThrottlerResult<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| ThrottlerError::RedisConnection(format!("Failed to create Redis client: {}", e)))?;

        info!("Redis client created with URL: {}", redis_url);
        
        Ok(Self {
            client,
            connection_timeout: Duration::from_secs(5),
        })
    }

    async fn get_connection(&self) -> ThrottlerResult<Connection> {
        timeout(self.connection_timeout, self.client.get_async_connection())
            .await
            .map_err(|_| ThrottlerError::RedisConnection("Connection timeout".to_string()))?
            .map_err(|e| ThrottlerError::RedisConnection(format!("Failed to get connection: {}", e)))
    }

    pub async fn ping(&self) -> ThrottlerResult<()> {
        let mut conn = self.get_connection().await?;
        let response: String = conn
            .ping()
            .await
            .map_err(|e| ThrottlerError::RedisConnection(format!("Ping failed: {}", e)))?;
        
        if response == "PONG" {
            debug!("Redis ping successful");
            Ok(())
        } else {
            Err(ThrottlerError::RedisConnection(
                "Unexpected ping response".to_string(),
            ))
        }
    }

    pub async fn get_token_count(&self, key: &str) -> ThrottlerResult<Option<f64>> {
        let mut conn = self.get_connection().await?;
        let result: Option<String> = conn
            .hget(key, "tokens")
            .await
            .map_err(|e| self.handle_redis_error(e, "get_token_count"))?;

        match result {
            Some(value) => {
                let tokens = value.parse::<f64>()
                    .map_err(|_| ThrottlerError::RedisConnection(
                        "Invalid token count format in Redis".to_string()
                    ))?;
                debug!("Retrieved token count for key {}: {}", key, tokens);
                Ok(Some(tokens))
            }
            None => {
                debug!("No token count found for key: {}", key);
                Ok(None)
            }
        }
    }

    pub async fn set_token_bucket(
        &self,
        key: &str,
        tokens: f64,
        last_refill: u64,
        limit: u64,
        refill_rate: f64,
        capacity: u64,
    ) -> ThrottlerResult<()> {
        let mut conn = self.get_connection().await?;
        
        let _: () = redis::pipe()
            .atomic()
            .hset(key, "tokens", tokens.to_string())
            .hset(key, "last_refill", last_refill.to_string())
            .hset(key, "limit", limit.to_string())
            .hset(key, "refill_rate", refill_rate.to_string())
            .hset(key, "capacity", capacity.to_string())
            .expire(key, 3600) // Expire after 1 hour of inactivity
            .query_async(&mut conn)
            .await
            .map_err(|e| self.handle_redis_error(e, "set_token_bucket"))?;

        debug!("Set token bucket for key {}: tokens={}, limit={}", key, tokens, limit);
        Ok(())
    }

    pub async fn get_token_bucket(&self, key: &str) -> ThrottlerResult<Option<TokenBucketData>> {
        let mut conn = self.get_connection().await?;
        
        let result: Vec<Option<String>> = conn
            .hmget(key, &["tokens", "last_refill", "limit", "refill_rate", "capacity"])
            .await
            .map_err(|e| self.handle_redis_error(e, "get_token_bucket"))?;

        if result.iter().all(|x| x.is_none()) {
            debug!("No token bucket data found for key: {}", key);
            return Ok(None);
        }

        let tokens = result[0].as_ref()
            .ok_or_else(|| ThrottlerError::RedisConnection("Missing tokens field".to_string()))?
            .parse::<f64>()
            .map_err(|_| ThrottlerError::RedisConnection("Invalid tokens format".to_string()))?;

        let last_refill = result[1].as_ref()
            .ok_or_else(|| ThrottlerError::RedisConnection("Missing last_refill field".to_string()))?
            .parse::<u64>()
            .map_err(|_| ThrottlerError::RedisConnection("Invalid last_refill format".to_string()))?;

        let limit = result[2].as_ref()
            .ok_or_else(|| ThrottlerError::RedisConnection("Missing limit field".to_string()))?
            .parse::<u64>()
            .map_err(|_| ThrottlerError::RedisConnection("Invalid limit format".to_string()))?;

        let refill_rate = result[3].as_ref()
            .ok_or_else(|| ThrottlerError::RedisConnection("Missing refill_rate field".to_string()))?
            .parse::<f64>()
            .map_err(|_| ThrottlerError::RedisConnection("Invalid refill_rate format".to_string()))?;

        let capacity = result[4].as_ref()
            .ok_or_else(|| ThrottlerError::RedisConnection("Missing capacity field".to_string()))?
            .parse::<u64>()
            .map_err(|_| ThrottlerError::RedisConnection("Invalid capacity format".to_string()))?;

        debug!("Retrieved token bucket for key {}: tokens={}, limit={}", key, tokens, limit);

        Ok(Some(TokenBucketData {
            tokens,
            last_refill,
            limit,
            refill_rate,
            capacity,
        }))
    }

    pub async fn delete_key(&self, key: &str) -> ThrottlerResult<bool> {
        let mut conn = self.get_connection().await?;
        let deleted: u64 = conn
            .del(key)
            .await
            .map_err(|e| self.handle_redis_error(e, "delete_key"))?;
        
        let was_deleted = deleted > 0;
        debug!("Delete key {}: success={}", key, was_deleted);
        Ok(was_deleted)
    }

    pub async fn list_keys(&self, pattern: &str) -> ThrottlerResult<Vec<String>> {
        let mut conn = self.get_connection().await?;
        let keys: Vec<String> = conn
            .keys(pattern)
            .await
            .map_err(|e| self.handle_redis_error(e, "list_keys"))?;
        
        debug!("Listed {} keys matching pattern: {}", keys.len(), pattern);
        Ok(keys)
    }

    fn handle_redis_error(&self, error: RedisError, operation: &str) -> ThrottlerError {
        match error.kind() {
            redis::ErrorKind::IoError => {
                warn!("Redis IO error during {}: {}", operation, error);
                ThrottlerError::RedisConnection(format!("Connection lost during {}", operation))
            }
            redis::ErrorKind::AuthenticationFailed => {
                error!("Redis authentication failed during {}: {}", operation, error);
                ThrottlerError::RedisConnection("Authentication failed".to_string())
            }
            _ => {
                error!("Redis error during {}: {}", operation, error);
                ThrottlerError::RedisConnection(format!("Redis error: {}", error))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenBucketData {
    pub tokens: f64,
    pub last_refill: u64,
    pub limit: u64,
    pub refill_rate: f64,
    pub capacity: u64,
}