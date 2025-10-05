use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod rate_limiter;
mod redis;

use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment
    let config = Config::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    // Initialize tracing
    if config.enable_tracing {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "throttler=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    tracing::info!("Starting throttler service");
    tracing::info!("Configuration: bind_addr={}, redis_url={}", 
                   config.bind_addr, config.redis_url);

    // TODO: Initialize Redis connection pool
    // TODO: Initialize rate limiter
    // TODO: Start HTTP server
    
    tracing::info!("Service would start on {}", config.bind_addr);
    
    Ok(())
}