use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    capacity: u32,
    tokens: f64,
    refill_rate: f64, // tokens per second
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

    pub fn consume(&mut self, tokens: u32) -> bool {
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

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        
        if elapsed > Duration::from_millis(1) {
            let tokens_to_add = self.refill_rate * elapsed.as_secs_f64();
            
            // Prevent overflow by capping at capacity
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
            self.last_refill = now;
        }
    }

    pub fn set_capacity(&mut self, capacity: u32) {
        self.capacity = capacity;
        // If current tokens exceed new capacity, cap them
        if self.tokens > capacity as f64 {
            self.tokens = capacity as f64;
        }
    }

    pub fn set_refill_rate(&mut self, rate: f64) {
        self.refill_rate = rate;
    }

    pub fn reset(&mut self) {
        self.tokens = self.capacity as f64;
        self.last_refill = Instant::now();
    }

    pub fn time_until_available(&mut self, required_tokens: u32) -> Option<Duration> {
        self.refill();
        
        if self.tokens >= required_tokens as f64 {
            return None;
        }
        
        let tokens_needed = required_tokens as f64 - self.tokens;
        let seconds_to_wait = tokens_needed / self.refill_rate;
        
        Some(Duration::from_secs_f64(seconds_to_wait))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10, 2.0);
        assert_eq!(bucket.capacity(), 10);
        assert_eq!(bucket.refill_rate(), 2.0);
    }

    #[test]
    fn test_token_consumption() {
        let mut bucket = TokenBucket::new(10, 2.0);
        assert!(bucket.consume(5));
        assert_eq!(bucket.available_tokens(), 5);
        assert!(bucket.consume(5));
        assert_eq!(bucket.available_tokens(), 0);
        assert!(!bucket.consume(1));
    }

    #[test]
    fn test_capacity_overflow_prevention() {
        let mut bucket = TokenBucket::new(5, 1000.0); // Very high refill rate
        bucket.consume(3);
        
        thread::sleep(Duration::from_millis(10));
        
        // Even with high refill rate, tokens should not exceed capacity
        assert!(bucket.available_tokens() <= 5);
        assert_eq!(bucket.available_tokens(), 5);
    }

    #[test]
    fn test_capacity_change() {
        let mut bucket = TokenBucket::new(10, 2.0);
        bucket.consume(5);
        
        // Reduce capacity below current tokens
        bucket.set_capacity(3);
        assert_eq!(bucket.available_tokens(), 3);
        
        // Increase capacity
        bucket.set_capacity(15);
        assert_eq!(bucket.capacity(), 15);
    }
}