use crate::config::Config;
use crate::error::ThrottlerError;
use crate::rate_limiter::RateLimiter;
use crate::redis::RedisClient;
use crate::response::ApiResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Reply};

pub type SharedState = Arc<RwLock<ServerState>>;

#[derive(Debug)]
pub struct ServerState {
    pub rate_limiters: HashMap<String, RateLimiter>,
    pub redis_client: Option<RedisClient>,
    pub config: Config,
}

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub key: String,
    pub tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_time: u64,
}

impl ServerState {
    pub fn new(config: Config) -> Self {
        let redis_client = if config.redis_enabled {
            match RedisClient::new(&config.redis_url) {
                Ok(client) => Some(client),
                Err(e) => {
                    log::warn!("Failed to connect to Redis: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            rate_limiters: HashMap::new(),
            redis_client,
            config,
        }
    }

    pub async fn get_or_create_limiter(&mut self, key: &str) -> &mut RateLimiter {
        if !self.rate_limiters.contains_key(key) {
            let limiter = RateLimiter::new(
                self.config.default_capacity,
                self.config.default_refill_rate,
            );
            self.rate_limiters.insert(key.to_string(), limiter);
        }
        self.rate_limiters.get_mut(key).unwrap()
    }
}

pub async fn check_rate_limit(
    request: CheckRequest,
    state: SharedState,
) -> Result<impl Reply, warp::Rejection> {
    let tokens = request.tokens.unwrap_or(1);
    let mut state_guard = state.write().await;
    
    // Try Redis first if available
    if let Some(ref mut redis_client) = state_guard.redis_client {
        match redis_client.check_rate_limit(&request.key, tokens).await {
            Ok((allowed, remaining, reset_time)) => {
                let response = CheckResponse {
                    allowed,
                    remaining,
                    reset_time,
                };
                return Ok(warp::reply::json(&ApiResponse::success(response)));
            }
            Err(e) => {
                log::warn!("Redis rate limit check failed: {}, falling back to local", e);
            }
        }
    }
    
    // Fall back to local rate limiter
    let limiter = state_guard.get_or_create_limiter(&request.key).await;
    let allowed = limiter.try_consume(tokens);
    let remaining = limiter.available_tokens();
    let reset_time = limiter.next_refill_time();
    
    let response = CheckResponse {
        allowed,
        remaining,
        reset_time,
    };
    
    Ok(warp::reply::json(&ApiResponse::success(response)))
}

pub async fn health_check() -> Result<impl Reply, warp::Rejection> {
    Ok(warp::reply::json(&ApiResponse::success("OK")))
}

pub fn routes(
    state: SharedState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let check = warp::path("check")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(check_rate_limit);

    let health = warp::path("health")
        .and(warp::get())
        .and_then(health_check);

    check.or(health)
}

fn with_state(
    state: SharedState,
) -> impl Filter<Extract = (SharedState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

pub async fn run_server(config: Config) -> Result<(), ThrottlerError> {
    let addr = ([127, 0, 0, 1], config.port);
    let state = Arc::new(RwLock::new(ServerState::new(config)));
    
    let routes = routes(state)
        .with(warp::cors().allow_any_origin())
        .with(warp::log("throttler"));
    
    log::info!("Starting server on http://{}", addr.1);
    warp::serve(routes).run(addr).await;
    
    Ok(())
}