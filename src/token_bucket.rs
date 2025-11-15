use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::error::ThrottlerError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    pub capacity: u64,
    pub tokens: f64,
    pub refill_rate: f64, // tokens per second
    pub last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    pub fn try_consume(&mut self, tokens: u64) -> Result<bool, ThrottlerError> {
        self.refill()?;
        
        let tokens_f64 = tokens as f64;
        if self.tokens >= tokens_f64 {
            self.tokens -= tokens_f64;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn refill(&mut self) -> Result<(), ThrottlerError> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        
        // Prevent overflow and handle very large time differences
        let max_elapsed = Duration::from_secs(3600); // 1 hour max
        let safe_elapsed = if elapsed > max_elapsed {
            max_elapsed
        } else {
            elapsed
        };
        
        let seconds_elapsed = safe_elapsed.as_secs_f64();
        
        // Avoid floating point precision issues with very small durations
        if seconds_elapsed < 0.001 {
            return Ok(());
        }
        
        let tokens_to_add = self.refill_rate * seconds_elapsed;
        
        // Ensure we don't exceed capacity and handle potential NaN/infinity
        if tokens_to_add.is_finite() && tokens_to_add > 0.0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
        }
        
        self.last_refill = now;
        Ok(())
    }

    pub fn available_tokens(&mut self) -> Result<u64, ThrottlerError> {
        self.refill()?;
        Ok(self.tokens.floor() as u64)
    }

    pub fn time_until_tokens(&mut self, tokens: u64) -> Result<Duration, ThrottlerError> {
        self.refill()?;
        
        let tokens_f64 = tokens as f64;
        if self.tokens >= tokens_f64 {
            return Ok(Duration::from_secs(0));
        }
        
        let tokens_needed = tokens_f64 - self.tokens;
        
        // Handle edge case where refill_rate is zero or very small
        if self.refill_rate <= 0.0 {
            return Ok(Duration::from_secs(u64::MAX));
        }
        
        let seconds_needed = tokens_needed / self.refill_rate;
        
        // Cap the wait time to prevent overflow
        let max_wait_seconds = 86400.0; // 24 hours
        let safe_seconds = seconds_needed.min(max_wait_seconds);
        
        Ok(Duration::from_secs_f64(safe_seconds))
    }

    pub fn reset(&mut self) {
        self.tokens = self.capacity as f64;
        self.last_refill = Instant::now();
    }

    pub fn is_empty(&mut self) -> Result<bool, ThrottlerError> {
        self.refill()?;
        Ok(self.tokens < 1.0)
    }

    pub fn utilization(&mut self) -> Result<f64, ThrottlerError> {
        self.refill()?;
        Ok(1.0 - (self.tokens / self.capacity as f64))
    }
}