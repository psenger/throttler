use redis::{Client, Connection, RedisError};
use std::time::Duration;

/// Redis client wrapper for distributed rate limiting
pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    /// Create a new Redis client with the given connection string
    pub fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        Ok(RedisClient { client })
    }

    /// Get a connection to Redis
    pub fn get_connection(&self) -> Result<Connection, RedisError> {
        self.client.get_connection()
    }

    /// Get connection with timeout
    pub fn get_connection_with_timeout(&self, timeout: Duration) -> Result<Connection, RedisError> {
        self.client.get_connection_with_timeout(timeout)
    }

    /// Increment a counter for the given key with expiration
    pub fn increment_with_expire(&self, key: &str, expire_seconds: u64) -> Result<i64, RedisError> {
        let mut conn = self.get_connection()?;
        let count: i64 = redis::cmd("INCR")
            .arg(key)
            .query(&mut conn)?;
        
        if count == 1 {
            redis::cmd("EXPIRE")
                .arg(key)
                .arg(expire_seconds)
                .execute(&mut conn)?;
        }
        
        Ok(count)
    }

    /// Get the current count for a key
    pub fn get_count(&self, key: &str) -> Result<i64, RedisError> {
        let mut conn = self.get_connection()?;
        let count: Option<i64> = redis::cmd("GET")
            .arg(key)
            .query(&mut conn)?;
        Ok(count.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_client_creation() {
        let result = RedisClient::new("redis://127.0.0.1:6379");
        assert!(result.is_ok());
    }
}