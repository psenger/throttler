use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    capacity: u64,
    tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
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

    pub fn try_consume(&mut self, tokens: u64) -> bool {
        self.refill();
        
        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    pub fn available_tokens(&mut self) -> u64 {
        self.refill();
        self.tokens.floor() as u64
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        
        // Handle potential time overflow or system clock adjustments
        if elapsed > Duration::from_secs(3600) {
            // If more than an hour has passed, just fill to capacity
            self.tokens = self.capacity as f64;
        } else {
            let elapsed_seconds = elapsed.as_secs_f64();
            // Prevent floating point precision issues
            if elapsed_seconds > 0.0 {
                let tokens_to_add = elapsed_seconds * self.refill_rate;
                self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
            }
        }
        
        self.last_refill = now;
    }

    pub fn time_until_tokens(&mut self, tokens: u64) -> Option<Duration> {
        self.refill();
        
        if self.tokens >= tokens as f64 {
            return Some(Duration::ZERO);
        }
        
        if self.refill_rate <= 0.0 {
            return None; // Will never have enough tokens
        }
        
        let tokens_needed = tokens as f64 - self.tokens;
        let seconds_needed = tokens_needed / self.refill_rate;
        
        // Prevent negative durations and handle edge cases
        if seconds_needed <= 0.0 {
            Some(Duration::ZERO)
        } else {
            Some(Duration::from_secs_f64(seconds_needed.min(86400.0))) // Cap at 24 hours
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10, 1.0);
        assert_eq!(bucket.capacity, 10);
        assert_eq!(bucket.tokens, 10.0);
        assert_eq!(bucket.refill_rate, 1.0);
    }

    #[test]
    fn test_consume_tokens() {
        let mut bucket = TokenBucket::new(10, 1.0);
        assert!(bucket.try_consume(5));
        assert_eq!(bucket.available_tokens(), 5);
    }

    #[test]
    fn test_zero_refill_rate() {
        let mut bucket = TokenBucket::new(10, 0.0);
        bucket.try_consume(5);
        assert_eq!(bucket.time_until_tokens(10), None);
    }

    #[test]
    fn test_time_overflow_handling() {
        let mut bucket = TokenBucket::new(10, 1.0);
        bucket.last_refill = Instant::now() - Duration::from_secs(7200); // 2 hours ago
        bucket.tokens = 0.0;
        
        bucket.refill();
        assert_eq!(bucket.tokens, 10.0); // Should be full after long period
    }
}