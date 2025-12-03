use anyhow::Result;
use throttler::config::Config;
use throttler::server::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Load configuration from environment
    let config = Config::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("throttler={},tower_http=debug", config.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting throttler service");
    tracing::info!(
        "Configuration: bind_address={}, redis_url={}",
        config.bind_address,
        config.redis_url
    );

    // Create and run the server
    let server = Server::new(config)
        .map_err(|e| anyhow::anyhow!("Failed to create server: {}", e))?;

    server
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
