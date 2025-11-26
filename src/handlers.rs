use crate::error::{ThrottlerError, Result};
use crate::rate_limiter::RateLimiter;
use crate::response::ThrottleResponse;
use crate::validation::RequestValidator;
use crate::key_generator::KeyGenerator;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckRequest {
    pub key: String,
    pub requests: Option<u64>,
    pub window_ms: Option<u64>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub key: String,
    pub requests: u64,
    pub window_ms: u64,
}

pub struct ThrottleHandlers {
    rate_limiter: Arc<RateLimiter>,
    validator: RequestValidator,
    key_generator: KeyGenerator,
}

impl ThrottleHandlers {
    pub fn new(rate_limiter: Arc<RateLimiter>) -> Self {
        Self {
            rate_limiter,
            validator: RequestValidator::new(),
            key_generator: KeyGenerator::new(),
        }
    }

    pub async fn check_rate_limit(
        &self,
        req: web::Json<CheckRequest>,
        http_req: HttpRequest,
    ) -> Result<HttpResponse> {
        // Validate input
        self.validator.validate_key(&req.key)?;
        
        if let Some(headers) = Some(&req.headers) {
            self.validator.validate_headers(headers)?;
        }

        let requests = req.requests.unwrap_or(100);
        let window_ms = req.window_ms.unwrap_or(60000);

        self.validator.validate_rate_limit(requests, window_ms)?;

        // Generate full key with client info if needed
        let client_ip = http_req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let full_key = if req.key.contains(':') {
            req.key.clone()
        } else {
            self.key_generator.generate_key(&req.key, Some(&client_ip), &req.headers)
        };

        // Check rate limit
        match self.rate_limiter.check_rate_limit(&full_key, requests, window_ms).await {
            Ok(response) => {
                let http_response = HttpResponse::Ok()
                    .insert_header(("X-RateLimit-Limit", requests.to_string()))
                    .insert_header(("X-RateLimit-Remaining", response.remaining.to_string()))
                    .insert_header(("X-RateLimit-Reset", response.reset_time.to_string()))
                    .insert_header(("X-RateLimit-Window", window_ms.to_string()));

                Ok(http_response.json(response))
            },
            Err(ThrottlerError::RateLimitExceeded { retry_after, limit, window_ms }) => {
                Err(ThrottlerError::RateLimitExceeded { retry_after, limit, window_ms })
            },
            Err(e) => Err(e),
        }
    }

    pub async fn configure_rate_limit(
        &self,
        req: web::Json<ConfigRequest>,
    ) -> Result<HttpResponse> {
        // Validate input
        self.validator.validate_key(&req.key)?;
        self.validator.validate_rate_limit(req.requests, req.window_ms)?;

        // Store configuration
        self.rate_limiter
            .set_rate_limit_config(&req.key, req.requests, req.window_ms)
            .await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "Rate limit configuration updated",
            "key": req.key,
            "requests": req.requests,
            "window_ms": req.window_ms
        })))
    }

    pub async fn get_rate_limit_config(
        &self,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        let key = path.into_inner();
        self.validator.validate_key(&key)?;

        match self.rate_limiter.get_rate_limit_config(&key).await {
            Ok(Some(config)) => {
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "key": key,
                    "requests": config.requests,
                    "window_ms": config.window_ms,
                    "created_at": config.created_at
                })))
            },
            Ok(None) => {
                Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": "not_found",
                    "message": format!("No configuration found for key: {}", key)
                })))
            },
            Err(e) => Err(e),
        }
    }

    pub async fn delete_rate_limit_config(
        &self,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        let key = path.into_inner();
        self.validator.validate_key(&key)?;

        self.rate_limiter.delete_rate_limit_config(&key).await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "Rate limit configuration deleted",
            "key": key
        })))
    }

    pub async fn reset_rate_limit(
        &self,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        let key = path.into_inner();
        self.validator.validate_key(&key)?;

        self.rate_limiter.reset_rate_limit(&key).await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "Rate limit reset",
            "key": key
        })))
    }
}
