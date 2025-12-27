//! # HTTP Server Module
//!
//! This module provides the HTTP server implementation for Throttler,
//! built on [Axum](https://github.com/tokio-rs/axum) with Tokio async runtime.
//!
//! ## Server Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         Server                              │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │                  Middleware Stack                   │    │
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
//! │  │  │ TraceLayer  │─▶│  CorsLayer  │─▶│   Router    │  │    │
//! │  │  │  (Logging)  │  │ (Permissive)│  │  (Routes)   │  │    │
//! │  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! │                                                             │
//! │  Routes:                                                    │
//! │  ├── GET    /health              → health_check             │
//! │  ├── GET    /ready               → readiness_check          │
//! │  ├── GET    /rate-limit/:key     → get_rate_limit           │
//! │  ├── POST   /rate-limit/:key     → set_rate_limit           │
//! │  ├── DELETE /rate-limit/:key     → delete_rate_limit        │
//! │  └── POST   /rate-limit/:key/check → check_rate_limit       │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Graceful Shutdown
//!
//! The server handles shutdown signals gracefully:
//! - `SIGINT` (Ctrl+C) - Interactive shutdown
//! - `SIGTERM` - Container/orchestrator shutdown (Unix only)
//!
//! In-flight requests are allowed to complete before the server exits.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use throttler::config::Config;
//! use throttler::server::Server;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::from_env()?;
//!     let server = Server::new(config)?;
//!     server.run().await
//! }
//! ```

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

/// HTTP server wrapper for the Throttler service.
///
/// The `Server` struct encapsulates the Axum router and bind address,
/// providing a clean interface for starting and running the HTTP server.
///
/// # Thread Safety
///
/// The server uses `Arc<RwLock<AppState>>` for shared state, allowing
/// concurrent read access with exclusive write access when needed.
///
/// # Example
///
/// ```rust,no_run
/// use throttler::config::Config;
/// use throttler::server::Server;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::from_env()?;
/// let server = Server::new(config)?;
/// server.run().await
/// # }
/// ```
pub struct Server {
    /// The configured Axum router with all routes and middleware
    app: Router,
    /// The address to bind the server to (e.g., "127.0.0.1:8080")
    bind_address: String,
}

/// Creates the Axum router with all routes and middleware configured.
///
/// This function is the primary entry point for building the application router.
/// It sets up:
/// - Rate limiting endpoints (`/rate-limit/:key/*`)
/// - Health check endpoints (`/health`, `/ready`)
/// - Middleware stack (tracing, CORS)
/// - Shared application state
///
/// # Arguments
///
/// * `config` - Application configuration containing Redis URL, bind address, etc.
///
/// # Returns
///
/// Returns a configured `Router` or an error if initialization fails.
///
/// # State Management
///
/// The router uses `Arc<RwLock<AppState>>` for thread-safe state sharing:
/// - **Read operations**: Multiple concurrent readers allowed
/// - **Write operations**: Exclusive access required
///
/// # Example
///
/// ```rust,no_run
/// use throttler::config::Config;
/// use throttler::server::create_app;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::from_env()?;
/// let app = create_app(config)?;
/// // Use app for testing with axum_test or similar
/// # Ok(())
/// # }
/// ```
pub fn create_app(config: Config) -> Result<Router, Box<dyn std::error::Error>> {
    // Create rate limiter - connects to Redis if URL is configured
    let rate_limiter = RateLimiter::new(config)?;

    // Create shared state wrapped in Arc<RwLock> for thread-safe access
    // - Arc: Allows multiple owners across async tasks
    // - RwLock: Allows concurrent reads, exclusive writes
    let state: SharedState = Arc::new(RwLock::new(AppState {
        rate_limiter,
        validator: RequestValidator::new(),
    }));

    // Build the router with all routes and middleware
    let app = Router::new()
        // Rate limiting endpoints - CRUD operations for rate limit configs
        .route("/rate-limit/:key", get(get_rate_limit))      // Get current limit status
        .route("/rate-limit/:key", post(set_rate_limit))     // Create/update limit config
        .route("/rate-limit/:key", delete(delete_rate_limit)) // Delete limit config
        .route("/rate-limit/:key/check", post(check_rate_limit)) // Check and consume tokens
        // Health and readiness endpoints - Kubernetes probes
        .route("/health", get(health_check))    // Liveness probe
        .route("/ready", get(readiness_check))  // Readiness probe (checks Redis)
        // Attach shared state to all routes
        .with_state(state)
        // Apply middleware stack (executed in reverse order)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http()) // Request/response tracing
                .layer(CorsLayer::permissive())    // Allow all CORS origins
        );

    Ok(app)
}

impl Server {
    /// Creates a new Server instance with the given configuration.
    ///
    /// This constructor:
    /// 1. Extracts the bind address from configuration
    /// 2. Creates the Axum router with all routes configured
    /// 3. Returns a ready-to-run Server instance
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Redis connection fails (when Redis URL is configured)
    /// - Router creation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use throttler::config::Config;
    /// use throttler::server::Server;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::from_env()?;
    /// let server = Server::new(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let bind_address = config.bind_address.clone();
        let app = create_app(config)?;
        Ok(Self { app, bind_address })
    }

    /// Starts the HTTP server and runs until a shutdown signal is received.
    ///
    /// This method:
    /// 1. Binds to the configured address
    /// 2. Logs startup information
    /// 3. Serves requests until shutdown signal
    /// 4. Performs graceful shutdown (completes in-flight requests)
    ///
    /// # Shutdown Behavior
    ///
    /// The server listens for:
    /// - `SIGINT` (Ctrl+C) - Immediate graceful shutdown
    /// - `SIGTERM` (Unix) - Container orchestrator shutdown
    ///
    /// All in-flight requests are allowed to complete before the server exits.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Binding to the address fails (port in use, permission denied)
    /// - Server encounters a fatal error during operation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use throttler::config::Config;
    /// use throttler::server::Server;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::from_env()?;
    ///     let server = Server::new(config)?;
    ///     server.run().await  // Blocks until shutdown signal
    /// }
    /// ```
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // Bind to the configured address
        let listener = tokio::net::TcpListener::bind(&self.bind_address).await?;

        // Log startup information
        tracing::info!("Throttler server starting on {}", self.bind_address);
        tracing::info!("Health check available at /health");
        tracing::info!("Readiness check available at /ready");

        // Run server with graceful shutdown support
        // - Handles incoming connections until shutdown signal
        // - Completes in-flight requests before exiting
        axum::serve(listener, self.app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

/// Waits for a shutdown signal (Ctrl+C or SIGTERM).
///
/// This function creates futures for both shutdown signals and
/// returns when either one is received. Used by the server for
/// graceful shutdown coordination.
///
/// # Platform Behavior
///
/// - **Unix**: Listens for both SIGINT (Ctrl+C) and SIGTERM
/// - **Windows**: Only listens for Ctrl+C (SIGTERM not available)
///
/// # Panics
///
/// Panics if signal handlers cannot be installed (rare system error).
async fn shutdown_signal() {
    // Future that completes on Ctrl+C
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    // Future that completes on SIGTERM (Unix only)
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    // On non-Unix platforms, create a future that never completes
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // Wait for either signal - first one wins
    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, initiating graceful shutdown");
        },
    }
}
