use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottleMetrics {
    pub total_requests: u64,
    pub allowed_requests: u64,
    pub throttled_requests: u64,
    pub last_reset: u64,
}

impl Default for ThrottleMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            allowed_requests: 0,
            throttled_requests: 0,
            last_reset: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientMetrics {
    pub client_id: String,
    pub metrics: ThrottleMetrics,
    pub current_tokens: f64,
    pub last_request: u64,
}

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    client_metrics: Arc<RwLock<HashMap<String, ThrottleMetrics>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            client_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_request(&self, client_id: &str, allowed: bool) {
        let mut metrics = self.client_metrics.write().await;
        let client_metrics = metrics.entry(client_id.to_string()).or_default();
        
        client_metrics.total_requests += 1;
        if allowed {
            client_metrics.allowed_requests += 1;
        } else {
            client_metrics.throttled_requests += 1;
        }
    }

    pub async fn get_client_metrics(&self, client_id: &str) -> Option<ThrottleMetrics> {
        let metrics = self.client_metrics.read().await;
        metrics.get(client_id).cloned()
    }

    pub async fn get_all_metrics(&self) -> HashMap<String, ThrottleMetrics> {
        let metrics = self.client_metrics.read().await;
        metrics.clone()
    }

    pub async fn reset_client_metrics(&self, client_id: &str) {
        let mut metrics = self.client_metrics.write().await;
        if let Some(client_metrics) = metrics.get_mut(client_id) {
            *client_metrics = ThrottleMetrics::default();
        }
    }

    pub async fn get_global_metrics(&self) -> ThrottleMetrics {
        let metrics = self.client_metrics.read().await;
        let mut global = ThrottleMetrics::default();
        
        for client_metrics in metrics.values() {
            global.total_requests += client_metrics.total_requests;
            global.allowed_requests += client_metrics.allowed_requests;
            global.throttled_requests += client_metrics.throttled_requests;
        }
        
        global
    }
}