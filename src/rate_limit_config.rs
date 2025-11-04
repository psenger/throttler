use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for rate limiting rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub rules: HashMap<String, RateLimitRule>,
    pub default_rule: RateLimitRule,
}

/// Individual rate limiting rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitRule {
    pub requests_per_second: u32,
    pub burst_capacity: u32,
    pub window_size: Duration,
    pub enabled: bool,
}

/// Rate limit strategy enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitStrategy {
    TokenBucket,
    FixedWindow,
    SlidingWindow,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            rules: HashMap::new(),
            default_rule: RateLimitRule::default(),
        }
    }
}

impl Default for RateLimitRule {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_capacity: 20,
            window_size: Duration::from_secs(60),
            enabled: true,
        }
    }
}

impl RateLimitConfig {
    /// Get rate limit rule for a specific key, falling back to default
    pub fn get_rule(&self, key: &str) -> &RateLimitRule {
        self.rules.get(key).unwrap_or(&self.default_rule)
    }

    /// Add or update a rate limit rule
    pub fn set_rule(&mut self, key: String, rule: RateLimitRule) {
        self.rules.insert(key, rule);
    }

    /// Remove a rate limit rule
    pub fn remove_rule(&mut self, key: &str) -> Option<RateLimitRule> {
        self.rules.remove(key)
    }

    /// Check if rate limiting is enabled for a key
    pub fn is_enabled(&self, key: &str) -> bool {
        self.get_rule(key).enabled
    }

    /// Get all configured rule keys
    pub fn get_rule_keys(&self) -> Vec<&String> {
        self.rules.keys().collect()
    }
}

impl RateLimitRule {
    /// Create a new rate limit rule
    pub fn new(
        requests_per_second: u32,
        burst_capacity: u32,
        window_size: Duration,
    ) -> Self {
        Self {
            requests_per_second,
            burst_capacity,
            window_size,
            enabled: true,
        }
    }

    /// Calculate refill rate in tokens per millisecond
    pub fn refill_rate_ms(&self) -> f64 {
        self.requests_per_second as f64 / 1000.0
    }

    /// Validate rule parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.requests_per_second == 0 {
            return Err("Requests per second must be greater than 0".to_string());
        }
        if self.burst_capacity == 0 {
            return Err("Burst capacity must be greater than 0".to_string());
        }
        if self.window_size.as_secs() == 0 {
            return Err("Window size must be greater than 0".to_string());
        }
        Ok(())
    }

    /// Create a disabled rule
    pub fn disabled() -> Self {
        Self {
            requests_per_second: 0,
            burst_capacity: 0,
            window_size: Duration::from_secs(0),
            enabled: false,
        }
    }
}