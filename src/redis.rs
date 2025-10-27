use crate::error::ThrottlerError;
use redis::{Client, Commands, Connection};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

/// Redis client wrapper with connection pooling and error handling
#[derive(Clone)]
pub struct RedisClient {
    client: Client,
    connection: Arc<Mutex<Option<Connection>>>,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> Result<Self, ThrottlerError> {
        let client = Client::open(redis_url)
            .map_err(|e| ThrottlerError::Redis(format!("Failed to create Redis client: {}", e)))?;
        
        Ok(Self {
            client,
            connection: Arc::new(Mutex::new(None)),
        })
    }

    /// Get or create a Redis connection with timeout
    async fn get_connection(&self) -> Result<Connection, ThrottlerError> {
        let timeout_duration = Duration::from_secs(2);
        
        timeout(timeout_duration, async {
            let mut conn_guard = self.connection.lock().unwrap();
            
            // Check if we have a valid connection
            if let Some(ref mut conn) = *conn_guard {
                // Test the connection
                match redis::cmd("PING").query::<String>(conn) {
                    Ok(_) => return Ok(conn.clone()),
                    Err(_) => {
                        // Connection is stale, remove it
                        *conn_guard = None;
                    }
                }
            }
            
            // Create new connection
            match self.client.get_connection() {
                Ok(new_conn) => {
                    *conn_guard = Some(new_conn.clone());
                    Ok(new_conn)
                }
                Err(e) => Err(ThrottlerError::Redis(format!("Failed to connect to Redis: {}", e)))
            }
        })
        .await
        .map_err(|_| ThrottlerError::Redis("Redis connection timeout".to_string()))?
    }

    /// Increment a key with expiry, return current count
    pub async fn incr_with_expiry(&self, key: &str, expiry_secs: u64) -> Result<u64, ThrottlerError> {
        let mut conn = self.get_connection().await?;
        
        // Use Redis transaction to ensure atomicity
        let result: Result<(u64,), redis::RedisError> = redis::transaction(&mut conn, &[key], |conn, pipe| {
            let current_count: u64 = conn.get(key).unwrap_or(0);
            
            if current_count == 0 {
                // First request, set with expiry
                pipe.set_ex(key, 1u64, expiry_secs as usize)
                    .ignore()
                    .get(key)
            } else {
                // Increment existing key
                pipe.incr(key, 1u64)
            }
        });
        
        match result {
            Ok((count,)) => Ok(count),
            Err(e) => {
                // Clear connection on error
                *self.connection.lock().unwrap() = None;
                Err(ThrottlerError::Redis(format!("Redis operation failed: {}", e)))
            }
        }
    }

    /// Ping Redis to check connection
    pub async fn ping(&self) -> Result<(), ThrottlerError> {
        let mut conn = self.get_connection().await?;
        
        match redis::cmd("PING").query::<String>(&mut conn) {
            Ok(_) => Ok(()),
            Err(e) => {
                *self.connection.lock().unwrap() = None;
                Err(ThrottlerError::Redis(format!("Redis ping failed: {}", e)))
            }
        }
    }

    /// Get current count for a key
    pub async fn get_count(&self, key: &str) -> Result<u64, ThrottlerError> {
        let mut conn = self.get_connection().await?;
        
        match conn.get(key) {
            Ok(count) => Ok(count),
            Err(redis::RedisError { kind: redis::ErrorKind::TypeError, .. }) => Ok(0),
            Err(e) => {
                *self.connection.lock().unwrap() = None;
                Err(ThrottlerError::Redis(format!("Failed to get count: {}", e)))
            }
        }
    }

    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<(), ThrottlerError> {
        let mut conn = self.get_connection().await?;
        
        match conn.del(key) {
            Ok(_) => Ok(()),
            Err(e) => {
                *self.connection.lock().unwrap() = None;
                Err(ThrottlerError::Redis(format!("Failed to delete key: {}", e)))
            }
        }
    }
}