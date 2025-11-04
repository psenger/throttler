use crate::error::{ThrottlerError, ThrottlerResult};
use crate::rate_limit_config::{RateLimitConfig, RateLimitRule};
use crate::rate_limiter::RateLimiter;
use crate::redis::RedisClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main throttler service that manages rate limiting
pub struct Throttler {
    rate_limiter: Arc<RateLimiter>,
    config: Arc<RwLock<RateLimitConfig>>,
    redis_client: Option<Arc<RedisClient>>,
}

impl Throttler {
    /// Create a new throttler instance
    pub async fn new(
        config: RateLimitConfig,
        redis_url: Option<String>,
    ) -> ThrottlerResult<Self> {
        let redis_client = if let Some(url) = redis_url {
            Some(Arc::new(RedisClient::new(&url).await?))
        } else {
            None
        };

        let rate_limiter = Arc::new(RateLimiter::new(redis_client.clone()).await?);
        
        Ok(Self {
            rate_limiter,
            config: Arc::new(RwLock::new(config)),
            redis_client,
        })
    }

    /// Check if a request should be throttled
    pub async fn should_throttle(&self, key: &str) -> ThrottlerResult<bool> {
        let config = self.config.read().await;
        let rule = config.get_rule(key);
        
        if !rule.enabled {
            return Ok(false);
        }

        self.rate_limiter
            .check_rate_limit(key, rule.requests_per_second, rule.burst_capacity)
            .await
    }

    /// Get current rate limit status for a key
    pub async fn get_rate_limit_status(
        &self,
        key: &str,
    ) -> ThrottlerResult<RateLimitStatus> {
        let config = self.config.read().await;
        let rule = config.get_rule(key);
        
        let remaining = self.rate_limiter.get_remaining_tokens(key).await?;
        
        Ok(RateLimitStatus {
            key: key.to_string(),
            limit: rule.requests_per_second,
            remaining,
            reset_time: self.rate_limiter.get_reset_time(key).await?,
            enabled: rule.enabled,
        })
    }

    /// Update rate limit configuration
    pub async fn update_config(&self, new_config: RateLimitConfig) -> ThrottlerResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    /// Add or update a specific rate limit rule
    pub async fn set_rule(&self, key: String, rule: RateLimitRule) -> ThrottlerResult<()> {
        rule.validate().map_err(ThrottlerError::ValidationError)?;
        
        let mut config = self.config.write().await;
        config.set_rule(key, rule);
        Ok(())
    }

    /// Remove a rate limit rule
    pub async fn remove_rule(&self, key: &str) -> ThrottlerResult<Option<RateLimitRule>> {
        let mut config = self.config.write().await;
        Ok(config.remove_rule(key))
    }

    /// Get all configured rules
    pub async fn get_all_rules(&self) -> ThrottlerResult<HashMap<String, RateLimitRule>> {
        let config = self.config.read().await;
        Ok(config.rules.clone())
    }

    /// Reset rate limit for a specific key
    pub async fn reset_rate_limit(&self, key: &str) -> ThrottlerResult<()> {
        self.rate_limiter.reset_rate_limit(key).await
    }

    /// Get throttler health status
    pub async fn health_check(&self) -> ThrottlerResult<HealthStatus> {
        let redis_healthy = if let Some(ref client) = self.redis_client {
            client.ping().await.is_ok()
        } else {
            true // In-memory mode is always healthy
        };

        let config = self.config.read().await;
        let total_rules = config.rules.len();
        let enabled_rules = config
            .rules
            .values()
            .filter(|rule| rule.enabled)
            .count();

        Ok(HealthStatus {
            healthy: redis_healthy,
            redis_connected: self.redis_client.is_some() && redis_healthy,
            total_rules,
            enabled_rules,
        })
    }
}

/// Rate limit status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitStatus {
    pub key: String,
    pub limit: u32,
    pub remaining: u32,
    pub reset_time: u64,
    pub enabled: bool,
}

/// Health status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub redis_connected: bool,
    pub total_rules: usize,
    pub enabled_rules: usize,
}