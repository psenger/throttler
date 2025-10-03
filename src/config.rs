use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
    pub default_limits: RateLimit,
    pub custom_limits: HashMap<String, RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_capacity: u32,
    pub window_seconds: u32,
}

impl Default for Config {
    fn default() -> Self {
        let mut custom_limits = HashMap::new();
        
        // Example custom limits for different API tiers
        custom_limits.insert(
            "premium".to_string(),
            RateLimit {
                requests_per_second: 100,
                burst_capacity: 200,
                window_seconds: 60,
            },
        );
        
        custom_limits.insert(
            "basic".to_string(),
            RateLimit {
                requests_per_second: 10,
                burst_capacity: 20,
                window_seconds: 60,
            },
        );

        Self {
            redis_url: "redis://localhost:6379".to_string(),
            default_limits: RateLimit {
                requests_per_second: 50,
                burst_capacity: 100,
                window_seconds: 60,
            },
            custom_limits,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path).required(false))
            .add_source(config::Environment::with_prefix("THROTTLER"))
            .build()?;

        match settings.try_deserialize::<Config>() {
            Ok(config) => Ok(config),
            Err(_) => {
                tracing::warn!("Failed to load config from {}, using defaults", path);
                Ok(Config::default())
            }
        }
    }

    pub fn get_limit_for_key(&self, key: &str) -> &RateLimit {
        // Try to extract tier from key (e.g., "user:123:premium")
        for (tier, limit) in &self.custom_limits {
            if key.contains(tier) {
                return limit;
            }
        }
        &self.default_limits
    }
}