//! # Throttler Service Orchestrator
//!
//! This module contains the main [`Throttler`] service that orchestrates
//! rate limiting operations, rule management, and health monitoring.
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                        Throttler                               │
//! ├────────────────────────────────────────────────────────────────┤
//! │                                                                │
//! │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
//! │  │   RateLimiter    │  │      Rules       │  │ RedisClient  │  │
//! │  │                  │  │   HashMap<K,V>   │  │  (Optional)  │  │
//! │  │ • Token buckets  │  │                  │  │              │  │
//! │  │ • Consumption    │  │ • Per-key rules  │  │ • Ping       │  │
//! │  │ • Refill logic   │  │ • Enable/disable │  │ • Health     │  │
//! │  └──────────────────┘  └──────────────────┘  └──────────────┘  │
//! │                                                                │
//! │  Methods:                                                      │
//! │  ├── should_throttle(key)     → Check if request is throttled  │
//! │  ├── get_rate_limit_status()  → Get current limit status       │
//! │  ├── set_rule(key, rule)      → Add/update rate limit rule     │
//! │  ├── remove_rule(key)         → Remove rate limit rule         │
//! │  ├── reset_rate_limit(key)    → Reset bucket to full capacity  │
//! │  └── health_check()           → Get service health status      │
//! │                                                                │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Relationship to RateLimiter
//!
//! The `Throttler` is a higher-level orchestrator that wraps the [`RateLimiter`]:
//!
//! | Component     | Responsibility                           |
//! |---------------|------------------------------------------|
//! | `Throttler`   | Rule management, health checks, API      |
//! | `RateLimiter` | Token bucket operations, storage         |
//!
//! ## Thread Safety
//!
//! - Rules are stored in `Arc<RwLock<HashMap>>` for concurrent access
//! - Multiple readers can check rules simultaneously
//! - Writers get exclusive access for rule modifications

use crate::config::Config;
use crate::error::{ThrottlerError, ThrottlerResult};
use crate::rate_limit_config::RateLimitRule;
use crate::rate_limiter::RateLimiter;
use crate::redis::RedisClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main throttler service that orchestrates rate limiting operations.
///
/// The `Throttler` provides a high-level API for:
/// - Checking if requests should be throttled
/// - Managing per-key rate limit rules
/// - Monitoring service health
///
/// # Example
///
/// ```rust,no_run
/// use throttler::config::Config;
/// use throttler::throttler::Throttler;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::from_env()?;
/// let throttler = Throttler::new(config)?;
///
/// // Check if a request should be throttled
/// let should_block = throttler.should_throttle("api-client-123").await?;
/// if should_block {
///     println!("Rate limit exceeded!");
/// }
/// # Ok(())
/// # }
/// ```
pub struct Throttler {
    /// Core rate limiting engine with token bucket implementation
    rate_limiter: RateLimiter,
    /// Per-key rate limit rules (allows custom limits per client/endpoint)
    rules: Arc<RwLock<HashMap<String, RateLimitRule>>>,
    /// Optional Redis client for distributed health checks
    redis_client: Option<Arc<RedisClient>>,
}

impl Throttler {
    /// Creates a new Throttler instance with the given configuration.
    ///
    /// This constructor:
    /// 1. Optionally connects to Redis if `redis_url` is configured
    /// 2. Creates the underlying `RateLimiter` engine
    /// 3. Initializes an empty rules map
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    ///
    /// # Errors
    ///
    /// Returns an error if Redis connection fails (when configured).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use throttler::config::Config;
    /// use throttler::throttler::Throttler;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::from_env()?;
    /// let throttler = Throttler::new(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: Config) -> ThrottlerResult<Self> {
        // Connect to Redis if URL is provided
        let redis_client = if !config.redis_url.is_empty() {
            Some(Arc::new(RedisClient::new(&config.redis_url)?))
        } else {
            None
        };

        // Create the core rate limiting engine
        let rate_limiter = RateLimiter::new(config)?;

        Ok(Self {
            rate_limiter,
            rules: Arc::new(RwLock::new(HashMap::new())),
            redis_client,
        })
    }

    /// Checks if a request should be throttled (rate limit exceeded).
    ///
    /// This method:
    /// 1. Checks if a specific rule exists for the key
    /// 2. If rule exists and is disabled, allows the request
    /// 3. Otherwise, checks the rate limiter for token availability
    ///
    /// # Arguments
    ///
    /// * `key` - The rate limit key (e.g., client ID, API key)
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - Request should be blocked (rate limit exceeded)
    /// - `Ok(false)` - Request should be allowed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use throttler::throttler::Throttler;
    /// # async fn example(throttler: &Throttler) -> Result<(), Box<dyn std::error::Error>> {
    /// if throttler.should_throttle("client-123").await? {
    ///     // Return 429 Too Many Requests
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn should_throttle(&self, key: &str) -> ThrottlerResult<bool> {
        let rules = self.rules.read().await;

        // Check if there's a specific rule for this key
        if let Some(rule) = rules.get(key) {
            // If rate limiting is disabled for this key, allow the request
            if !rule.enabled {
                return Ok(false);
            }
        }

        // Check the rate limiter - returns (allowed, remaining)
        let (allowed, _remaining) = self.rate_limiter.check_rate_limit(key)?;

        // Return true if request should be throttled (not allowed)
        Ok(!allowed)
    }

    /// Gets the current rate limit status for a key.
    ///
    /// Returns information about:
    /// - The configured limit
    /// - Remaining tokens in the bucket
    /// - Whether rate limiting is enabled for this key
    ///
    /// # Arguments
    ///
    /// * `key` - The rate limit key
    ///
    /// # Returns
    ///
    /// A `RateLimitStatus` with current limit information.
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

    /// Adds or updates a rate limit rule for a specific key.
    ///
    /// Rules allow custom rate limits per client or endpoint, overriding
    /// the default configuration.
    ///
    /// # Arguments
    ///
    /// * `key` - The rate limit key
    /// * `rule` - The rate limit rule to apply
    ///
    /// # Errors
    ///
    /// Returns an error if rule validation fails.
    pub async fn set_rule(&self, key: String, rule: RateLimitRule) -> ThrottlerResult<()> {
        // Validate the rule before storing
        rule.validate().map_err(ThrottlerError::ValidationError)?;

        let mut rules = self.rules.write().await;
        rules.insert(key, rule);
        Ok(())
    }

    /// Removes a rate limit rule for a specific key.
    ///
    /// After removal, the key will use default rate limits.
    ///
    /// # Arguments
    ///
    /// * `key` - The rate limit key
    ///
    /// # Returns
    ///
    /// The removed rule, if it existed.
    pub async fn remove_rule(&self, key: &str) -> ThrottlerResult<Option<RateLimitRule>> {
        let mut rules = self.rules.write().await;
        Ok(rules.remove(key))
    }

    /// Gets all configured rate limit rules.
    ///
    /// # Returns
    ///
    /// A clone of the rules map.
    pub async fn get_all_rules(&self) -> ThrottlerResult<HashMap<String, RateLimitRule>> {
        let rules = self.rules.read().await;
        Ok(rules.clone())
    }

    /// Resets the rate limit bucket for a specific key.
    ///
    /// This restores the bucket to full capacity, allowing
    /// immediate requests. Useful for:
    /// - Manual intervention
    /// - Testing
    /// - Customer support actions
    ///
    /// # Arguments
    ///
    /// * `key` - The rate limit key to reset
    pub async fn reset_rate_limit(&self, key: &str) -> ThrottlerResult<()> {
        self.rate_limiter.reset(key)
    }

    /// Gets the current health status of the throttler service.
    ///
    /// Checks Redis connectivity if configured, otherwise reports
    /// as healthy (in-memory mode is always available).
    ///
    /// # Returns
    ///
    /// A `HealthStatus` indicating service health.
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

/// Current rate limit status for a specific key.
///
/// Provides a snapshot of the rate limit configuration and current
/// bucket state for monitoring and debugging.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitStatus {
    /// The rate limit key
    pub key: String,
    /// Maximum requests allowed per window
    pub limit: u32,
    /// Remaining requests in current window
    pub remaining: u32,
    /// Whether rate limiting is enabled for this key
    pub enabled: bool,
}

/// Service health status information.
///
/// Used by health check endpoints to report service availability.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    /// Overall service health (true = healthy)
    pub healthy: bool,
    /// Whether Redis is connected and responsive
    pub redis_connected: bool,
}
