use crate::config::Config;
use crate::error::{ThrottlerError, ThrottlerResult};
use crate::rate_limit_config::RateLimitRule;
use crate::rate_limiter::RateLimiter;
use crate::redis::RedisClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main throttler service that manages rate limiting
pub struct Throttler {
    rate_limiter: RateLimiter,
    rules: Arc<RwLock<HashMap<String, RateLimitRule>>>,
    redis_client: Option<Arc<RedisClient>>,
}

impl Throttler {
    /// Create a new throttler instance
    pub fn new(config: Config) -> ThrottlerResult<Self> {
        let redis_client = if !config.redis_url.is_empty() {
            Some(Arc::new(RedisClient::new(&config.redis_url)?))
        } else {
            None
        };

        let rate_limiter = RateLimiter::new(config)?;

        Ok(Self {
            rate_limiter,
            rules: Arc::new(RwLock::new(HashMap::new())),
            redis_client,
        })
    }

    /// Check if a request should be throttled
    pub async fn should_throttle(&self, key: &str) -> ThrottlerResult<bool> {
        let rules = self.rules.read().await;

        // Check if there's a specific rule for this key
        if let Some(rule) = rules.get(key) {
            if !rule.enabled {
                return Ok(false);
            }
        }

        let (allowed, _remaining) = self.rate_limiter.check_rate_limit(key)?;
        Ok(!allowed)
    }

    /// Get current rate limit status for a key
    pub async fn get_rate_limit_status(&self, key: &str) -> ThrottlerResult<RateLimitStatus> {
        let rules = self.rules.read().await;
        let rule = rules.get(key).cloned().unwrap_or_default();

        let remaining = self.rate_limiter.get_remaining_tokens(key)?;

        Ok(RateLimitStatus {
            key: key.to_string(),
            limit: rule.requests_per_second,
            remaining: remaining as u32,
            enabled: rule.enabled,
        })
    }

    /// Add or update a specific rate limit rule
    pub async fn set_rule(&self, key: String, rule: RateLimitRule) -> ThrottlerResult<()> {
        rule.validate().map_err(ThrottlerError::ValidationError)?;

        let mut rules = self.rules.write().await;
        rules.insert(key, rule);
        Ok(())
    }

    /// Remove a rate limit rule
    pub async fn remove_rule(&self, key: &str) -> ThrottlerResult<Option<RateLimitRule>> {
        let mut rules = self.rules.write().await;
        Ok(rules.remove(key))
    }

    /// Get all configured rules
    pub async fn get_all_rules(&self) -> ThrottlerResult<HashMap<String, RateLimitRule>> {
        let rules = self.rules.read().await;
        Ok(rules.clone())
    }

    /// Reset rate limit for a specific key
    pub async fn reset_rate_limit(&self, key: &str) -> ThrottlerResult<()> {
        self.rate_limiter.reset(key)
    }

    /// Get throttler health status
    pub fn health_check(&self) -> HealthStatus {
        let redis_healthy = if let Some(ref client) = self.redis_client {
            client.ping().is_ok()
        } else {
            true // In-memory mode is always healthy
        };

        HealthStatus {
            healthy: redis_healthy,
            redis_connected: self.redis_client.is_some() && redis_healthy,
        }
    }
}

/// Rate limit status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitStatus {
    pub key: String,
    pub limit: u32,
    pub remaining: u32,
    pub enabled: bool,
}

/// Health status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub redis_connected: bool,
}
