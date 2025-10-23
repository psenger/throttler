use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    capacity: u32,
    tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    pub fn try_consume(&mut self, tokens: u32) -> bool {
        self.refill();
        
        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    pub fn available_tokens(&mut self) -> u32 {
        self.refill();
        self.tokens.floor() as u32
    }

    pub fn time_until_available(&self, tokens: u32) -> Duration {
        let needed = tokens as f64 - self.tokens;
        if needed <= 0.0 {
            Duration::from_secs(0)
        } else {
            let seconds = needed / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.refill_rate;
        
        self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
        self.last_refill = now;
    }

    pub fn reset(&mut self) {
        self.tokens = self.capacity as f64;
        self.last_refill = Instant::now();
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
        assert_eq!(bucket.refill_rate, 1.0);
    }

    #[test]
    fn test_consume_tokens() {
        let mut bucket = TokenBucket::new(10, 1.0);
        assert!(bucket.try_consume(5));
        assert_eq!(bucket.available_tokens(), 5);
    }

    #[test]
    fn test_insufficient_tokens() {
        let mut bucket = TokenBucket::new(5, 1.0);
        assert!(!bucket.try_consume(10));
        assert_eq!(bucket.available_tokens(), 5);
    }

    #[test]
    fn test_token_refill() {
        let mut bucket = TokenBucket::new(10, 2.0);
        bucket.try_consume(10);
        thread::sleep(Duration::from_millis(500));
        assert!(bucket.available_tokens() > 0);
    }
}