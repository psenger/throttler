use crate::config::Config;
use crate::handlers::{
    check_rate_limit, delete_rate_limit, get_rate_limit, set_rate_limit,
    health_check, readiness_check, AppState, SharedState,
};
use crate::rate_limiter::RateLimiter;
use crate::validation::RequestValidator;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tokio::signal;

pub struct Server {
    app: Router,
    bind_address: String,
}

/// Create router for testing purposes
pub fn create_app(config: Config) -> Result<Router, Box<dyn std::error::Error>> {
    // Create rate limiter
    let rate_limiter = RateLimiter::new(config)?;

    // Create shared state
    let state: SharedState = Arc::new(RwLock::new(AppState {
        rate_limiter,
        validator: RequestValidator::new(),
    }));

    let app = Router::new()
        // Rate limiting endpoints
        .route("/rate-limit/:key", get(get_rate_limit))
        .route("/rate-limit/:key", post(set_rate_limit))
        .route("/rate-limit/:key", delete(delete_rate_limit))
        .route("/rate-limit/:key/check", post(check_rate_limit))
        // Health and readiness endpoints
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        );

    Ok(app)
}

impl Server {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let bind_address = config.bind_address.clone();
        let app = create_app(config)?;
        Ok(Self { app, bind_address })
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind(&self.bind_address).await?;

        tracing::info!("Throttler server starting on {}", self.bind_address);
        tracing::info!("Health check available at /health");
        tracing::info!("Readiness check available at /ready");

        // Run server with graceful shutdown
        axum::serve(listener, self.app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, initiating graceful shutdown");
        },
    }
}
