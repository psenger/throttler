use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

mod config;
mod rate_limiter;
mod redis_client;
mod routes;

use config::Config;
use rate_limiter::RateLimiter;
use redis_client::RedisClient;

#[derive(Parser)]
#[command(name = "throttler")]
#[command(about = "A lightweight Rust web API rate limiting service")]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    #[arg(short, long, default_value = "0.0.0.0:3000")]
    bind: String,
}

#[derive(Clone)]
pub struct AppState {
    pub rate_limiter: Arc<RateLimiter>,
    pub redis_client: Arc<RedisClient>,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    
    // Load configuration
    let config = Arc::new(Config::load(&cli.config)?);
    info!("Loaded configuration from {}", cli.config);

    // Initialize Redis client
    let redis_client = Arc::new(RedisClient::new(&config.redis_url).await?);
    info!("Connected to Redis at {}", config.redis_url);

    // Initialize rate limiter
    let rate_limiter = Arc::new(RateLimiter::new(redis_client.clone()));
    
    let state = AppState {
        rate_limiter,
        redis_client,
        config,
    };

    // Build the application routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/throttle/:key", post(routes::throttle_request))
        .route("/api/v1/config", get(routes::get_config))
        .route("/api/v1/config", post(routes::update_config))
        .route("/api/v1/stats/:key", get(routes::get_stats))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    info!("Starting throttler service on {}", cli.bind);
    let listener = TcpListener::bind(&cli.bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "throttler",
        "version": env!("CARGO_PKG_VERSION")
    })))
}