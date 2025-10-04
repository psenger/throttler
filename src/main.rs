mod config;
mod rate_limiter;
mod redis;

use config::Config;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment or defaults
    let config = Config::from_env();
    
    println!("Starting throttler service...");
    println!("Server will listen on: {}:{}", config.host, config.port);
    println!("Redis URL: {}", config.redis_url);
    
    // Initialize Redis client
    let redis_client = redis::RedisClient::new(&config.redis_url)?;
    println!("Redis connection established");
    
    // TODO: Initialize web server
    // TODO: Set up rate limiting middleware
    // TODO: Configure API routes
    
    println!("Throttler service started successfully!");
    
    // Keep the service running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down throttler service...");
    
    Ok(())
}