use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::{
    config::Config,
    handlers,
    middleware::{logging_middleware, cors_middleware},
    throttler::Throttler,
};

pub fn create_app(config: Arc<Config>, throttler: Arc<Throttler>) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/throttle", post(handlers::check_throttle))
        .route("/limits", get(handlers::get_limits))
        .route("/limits", post(handlers::create_limit))
        .route("/limits/:id", put(handlers::update_limit))
        .route("/limits/:id", delete(handlers::delete_limit))
        .layer(middleware::from_fn(cors_middleware))
        .layer(middleware::from_fn(logging_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state((config, throttler))
}

pub async fn start_server(config: Arc<Config>, throttler: Arc<Throttler>) -> crate::Result<()> {
    let app = create_app(config.clone(), throttler);
    let addr = format!("{}:{}", config.host, config.port);
    
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await
        .map_err(|e| crate::Error::Config(format!("Failed to bind to {}: {}", addr, e)))?;
    
    axum::serve(listener, app).await
        .map_err(|e| crate::Error::Config(format!("Server error: {}", e)))?;
    
    Ok(())
}