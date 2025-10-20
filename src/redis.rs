use crate::error::ThrottlerError;
use redis::{Client, Connection, RedisResult};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;

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

    pub async fn connect(&self) -> Result<(), ThrottlerError> {
        let conn = self.client.get_connection()
            .map_err(|e| ThrottlerError::Redis(format!("Failed to connect to Redis: {}", e)))?;
        
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        *connection_guard = Some(conn);
        
        Ok(())
    }

    pub async fn ping(&self) -> Result<String, ThrottlerError> {
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        
        if let Some(ref mut conn) = *connection_guard {
            let result: RedisResult<String> = redis::cmd("PING").query(conn);
            result.map_err(|e| ThrottlerError::Redis(format!("Ping failed: {}", e)))
        } else {
            Err(ThrottlerError::Redis("No Redis connection available".into()))
        }
    }

    pub async fn eval_script<T>(
        &self,
        script: &str,
        keys: Vec<&str>,
        args: Vec<&str>,
    ) -> Result<T, ThrottlerError>
    where
        T: redis::FromRedisValue,
    {
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        
        if let Some(ref mut conn) = *connection_guard {
            let mut cmd = redis::cmd("EVAL");
            cmd.arg(script).arg(keys.len());
            
            for key in keys {
                cmd.arg(key);
            }
            for arg in args {
                cmd.arg(arg);
            }
            
            let result: RedisResult<T> = cmd.query(conn);
            result.map_err(|e| ThrottlerError::Redis(format!("Script execution failed: {}", e)))
        } else {
            Err(ThrottlerError::Redis("No Redis connection available".into()))
        }
    }

    pub async fn hmget(
        &self,
        key: &str,
        fields: &[&str],
    ) -> Result<Vec<String>, ThrottlerError> {
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        
        if let Some(ref mut conn) = *connection_guard {
            let mut cmd = redis::cmd("HMGET");
            cmd.arg(key);
            for field in fields {
                cmd.arg(field);
            }
            
            let result: RedisResult<Vec<Option<String>>> = cmd.query(conn);
            match result {
                Ok(values) => {
                    Ok(values.into_iter()
                        .map(|v| v.unwrap_or_default())
                        .collect())
                }
                Err(e) => Err(ThrottlerError::Redis(format!("HMGET failed: {}", e)))
            }
        } else {
            Err(ThrottlerError::Redis("No Redis connection available".into()))
        }
    }

    pub async fn set_with_expiry(
        &self,
        key: &str,
        value: &str,
        expiry_seconds: u64,
    ) -> Result<(), ThrottlerError> {
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        
        if let Some(ref mut conn) = *connection_guard {
            let result: RedisResult<String> = redis::cmd("SETEX")
                .arg(key)
                .arg(expiry_seconds)
                .arg(value)
                .query(conn);
            
            result
                .map(|_| ())
                .map_err(|e| ThrottlerError::Redis(format!("SETEX failed: {}", e)))
        } else {
            Err(ThrottlerError::Redis("No Redis connection available".into()))
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, ThrottlerError> {
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        
        if let Some(ref mut conn) = *connection_guard {
            let result: RedisResult<Option<String>> = redis::cmd("GET").arg(key).query(conn);
            result.map_err(|e| ThrottlerError::Redis(format!("GET failed: {}", e)))
        } else {
            Err(ThrottlerError::Redis("No Redis connection available".into()))
        }
    }

    pub async fn delete(&self, key: &str) -> Result<bool, ThrottlerError> {
        let mut connection_guard = self.connection.lock().map_err(|_| {
            ThrottlerError::Internal("Failed to acquire connection lock".into())
        })?;
        
        if let Some(ref mut conn) = *connection_guard {
            let result: RedisResult<i32> = redis::cmd("DEL").arg(key).query(conn);
            result
                .map(|deleted_count| deleted_count > 0)
                .map_err(|e| ThrottlerError::Redis(format!("DEL failed: {}", e)))
        } else {
            Err(ThrottlerError::Redis("No Redis connection available".into()))
        }
    }
}