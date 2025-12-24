//! Rate limiting for WebSocket connections
//!
//! This module provides connection-level rate limiting to prevent abuse and
//! protect the server from message flooding attacks.
//!
//! # Overview
//!
//! Uses a token bucket algorithm to limit the rate of incoming messages:
//! - Each connection has a bucket with a maximum capacity
//! - Tokens are consumed when messages arrive
//! - Tokens are refilled at a constant rate
//! - When the bucket is empty, messages are rejected
//!
//! # Usage Example
//!
//! ```rust
//! use archischema::core::liveshare::rate_limiter::RateLimiter;
//! use std::time::Duration;
//!
//! let mut limiter = RateLimiter::new(100, Duration::from_secs(1));
//!
//! // Try to consume tokens for a message
//! if limiter.check_and_consume(1) {
//!     // Process message
//! } else {
//!     // Reject message - rate limit exceeded
//! }
//! ```

use std::time::{Duration, Instant};

/// Default maximum tokens in the bucket
pub const DEFAULT_MAX_TOKENS: u32 = 100;

/// Default refill rate (tokens per second)
pub const DEFAULT_REFILL_RATE: u32 = 50;

/// Token bucket rate limiter
///
/// Implements the token bucket algorithm for rate limiting:
/// - Maintains a bucket with a maximum capacity
/// - Tokens are added to the bucket at a constant rate
/// - Each operation consumes tokens from the bucket
/// - Operations fail when insufficient tokens are available
///
/// # Example
/// ```
/// # use archischema::core::liveshare::rate_limiter::RateLimiter;
/// # use std::time::Duration;
/// let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
///
/// // First message succeeds
/// assert!(limiter.check_and_consume(1));
///
/// // Consuming all remaining tokens
/// for _ in 0..9 {
///     assert!(limiter.check_and_consume(1));
/// }
///
/// // Next message fails (bucket empty)
/// assert!(!limiter.check_and_consume(1));
/// ```
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Maximum number of tokens the bucket can hold
    max_tokens: u32,
    /// Current number of tokens in the bucket
    current_tokens: f64,
    /// Rate at which tokens are refilled (tokens per second)
    refill_rate: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum bucket capacity
    /// * `refill_interval` - Time to refill the entire bucket
    ///
    /// # Example
    /// ```
    /// # use archischema::core::liveshare::rate_limiter::RateLimiter;
    /// # use std::time::Duration;
    /// // 100 tokens max, refills in 1 second (100 tokens/sec)
    /// let limiter = RateLimiter::new(100, Duration::from_secs(1));
    /// ```
    pub fn new(max_tokens: u32, refill_interval: Duration) -> Self {
        let refill_rate = max_tokens as f64 / refill_interval.as_secs_f64();
        Self {
            max_tokens,
            current_tokens: max_tokens as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Create a rate limiter with specified refill rate (tokens per second)
    ///
    /// # Example
    /// ```
    /// # use archischema::core::liveshare::rate_limiter::RateLimiter;
    /// // 100 tokens max, refills at 50 tokens/sec
    /// let limiter = RateLimiter::with_rate(100, 50);
    /// ```
    pub fn with_rate(max_tokens: u32, tokens_per_second: u32) -> Self {
        Self {
            max_tokens,
            current_tokens: max_tokens as f64,
            refill_rate: tokens_per_second as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Calculate tokens to add
        let tokens_to_add = elapsed * self.refill_rate;

        // Add tokens, capped at max_tokens
        self.current_tokens = (self.current_tokens + tokens_to_add).min(self.max_tokens as f64);
        self.last_refill = now;
    }

    /// Check if tokens are available and consume them if so
    ///
    /// Returns `true` if the operation is allowed (tokens consumed),
    /// `false` if rate limit exceeded (tokens not consumed).
    ///
    /// # Arguments
    /// * `tokens` - Number of tokens to consume
    pub fn check_and_consume(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.current_tokens >= tokens as f64 {
            self.current_tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    /// Check if tokens are available without consuming them
    pub fn check(&mut self, tokens: u32) -> bool {
        self.refill();
        self.current_tokens >= tokens as f64
    }

    /// Get current token count (after refill)
    pub fn current_tokens(&mut self) -> u32 {
        self.refill();
        self.current_tokens as u32
    }

    /// Get maximum token capacity
    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    /// Get refill rate (tokens per second)
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }

    /// Reset the limiter to full capacity
    pub fn reset(&mut self) {
        self.current_tokens = self.max_tokens as f64;
        self.last_refill = Instant::now();
    }

    /// Check if the bucket is full
    pub fn is_full(&mut self) -> bool {
        self.refill();
        self.current_tokens >= self.max_tokens as f64
    }

    /// Check if the bucket is empty
    pub fn is_empty(&mut self) -> bool {
        self.refill();
        self.current_tokens < 1.0
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::with_rate(DEFAULT_MAX_TOKENS, DEFAULT_REFILL_RATE)
    }
}

/// Per-message-type rate limiter
///
/// Different message types can have different rate limits.
/// For example, cursor updates might be limited more strictly than schema updates.
#[derive(Debug, Clone)]
pub struct MessageRateLimiter {
    /// Limiter for volatile messages (cursor, viewport)
    volatile_limiter: RateLimiter,
    /// Limiter for normal messages (schema updates, awareness)
    normal_limiter: RateLimiter,
    /// Limiter for critical messages (auth, sync)
    critical_limiter: RateLimiter,
}

impl MessageRateLimiter {
    /// Create a new message rate limiter with default limits
    pub fn new() -> Self {
        Self {
            // Volatile: 60 messages/sec (very frequent cursor updates)
            volatile_limiter: RateLimiter::with_rate(120, 60),
            // Normal: 30 messages/sec (schema updates, awareness)
            normal_limiter: RateLimiter::with_rate(60, 30),
            // Critical: 10 messages/sec (auth, sync - should be rare)
            critical_limiter: RateLimiter::with_rate(20, 10),
        }
    }

    /// Create with custom limiters
    pub fn with_limiters(
        volatile_limiter: RateLimiter,
        normal_limiter: RateLimiter,
        critical_limiter: RateLimiter,
    ) -> Self {
        Self {
            volatile_limiter,
            normal_limiter,
            critical_limiter,
        }
    }

    /// Check and consume for volatile messages
    pub fn check_volatile(&mut self) -> bool {
        self.volatile_limiter.check_and_consume(1)
    }

    /// Check and consume for normal messages
    pub fn check_normal(&mut self) -> bool {
        self.normal_limiter.check_and_consume(1)
    }

    /// Check and consume for critical messages
    pub fn check_critical(&mut self) -> bool {
        self.critical_limiter.check_and_consume(1)
    }

    /// Reset all limiters
    pub fn reset_all(&mut self) {
        self.volatile_limiter.reset();
        self.normal_limiter.reset();
        self.critical_limiter.reset();
    }
}

impl Default for MessageRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_rate_limiter_new() {
        let limiter = RateLimiter::new(100, Duration::from_secs(1));
        assert_eq!(limiter.max_tokens(), 100);
        assert_eq!(limiter.refill_rate(), 100.0);
    }

    #[test]
    fn test_rate_limiter_with_rate() {
        let limiter = RateLimiter::with_rate(100, 50);
        assert_eq!(limiter.max_tokens(), 100);
        assert_eq!(limiter.refill_rate(), 50.0);
    }

    #[test]
    fn test_rate_limiter_initial_full() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        assert_eq!(limiter.current_tokens(), 10);
        assert!(limiter.is_full());
    }

    #[test]
    fn test_rate_limiter_consume_single() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        assert!(limiter.check_and_consume(1));
        assert_eq!(limiter.current_tokens(), 9);
    }

    #[test]
    fn test_rate_limiter_consume_multiple() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        assert!(limiter.check_and_consume(5));
        assert_eq!(limiter.current_tokens(), 5);
        assert!(limiter.check_and_consume(3));
        assert_eq!(limiter.current_tokens(), 2);
    }

    #[test]
    fn test_rate_limiter_exhaust() {
        let mut limiter = RateLimiter::new(5, Duration::from_secs(1));
        for _ in 0..5 {
            assert!(limiter.check_and_consume(1));
        }
        assert!(limiter.is_empty());
        assert!(!limiter.check_and_consume(1));
    }

    #[test]
    fn test_rate_limiter_exceed_capacity() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        assert!(!limiter.check_and_consume(11));
        assert_eq!(limiter.current_tokens(), 10);
    }

    #[test]
    fn test_rate_limiter_check_without_consume() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        assert!(limiter.check(5));
        assert_eq!(limiter.current_tokens(), 10); // No consumption
        assert!(limiter.check_and_consume(5));
        assert_eq!(limiter.current_tokens(), 5); // Now consumed
    }

    #[test]
    fn test_rate_limiter_refill() {
        let mut limiter = RateLimiter::with_rate(10, 10); // 10 tokens/sec
        assert!(limiter.check_and_consume(10));
        assert!(limiter.is_empty());

        thread::sleep(Duration::from_millis(500)); // Wait for ~5 tokens

        // Should have refilled some tokens
        let tokens = limiter.current_tokens();
        assert!((4..=6).contains(&tokens)); // Allow some variance
    }

    #[test]
    fn test_rate_limiter_refill_caps_at_max() {
        let mut limiter = RateLimiter::with_rate(10, 100); // Fast refill
        assert!(limiter.check_and_consume(5));

        thread::sleep(Duration::from_millis(200)); // More than enough to refill

        assert_eq!(limiter.current_tokens(), 10); // Capped at max
        assert!(limiter.is_full());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(1));
        assert!(limiter.check_and_consume(10));
        assert!(limiter.is_empty());

        limiter.reset();
        assert_eq!(limiter.current_tokens(), 10);
        assert!(limiter.is_full());
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter = RateLimiter::default();
        assert_eq!(limiter.max_tokens(), DEFAULT_MAX_TOKENS);
        assert_eq!(limiter.refill_rate(), DEFAULT_REFILL_RATE as f64);
    }

    #[test]
    fn test_message_rate_limiter_new() {
        let limiter = MessageRateLimiter::new();
        // Just check it creates successfully
        assert_eq!(limiter.volatile_limiter.max_tokens(), 120);
    }

    #[test]
    fn test_message_rate_limiter_volatile() {
        let mut limiter = MessageRateLimiter::new();
        assert!(limiter.check_volatile());
        // Should allow many volatile messages
        for _ in 0..100 {
            limiter.check_volatile();
        }
    }

    #[test]
    fn test_message_rate_limiter_normal() {
        let mut limiter = MessageRateLimiter::new();
        assert!(limiter.check_normal());
        // Should allow several normal messages
        for _ in 0..50 {
            limiter.check_normal();
        }
    }

    #[test]
    fn test_message_rate_limiter_critical() {
        let mut limiter = MessageRateLimiter::new();
        assert!(limiter.check_critical());
        // Should allow some critical messages
        for _ in 0..15 {
            limiter.check_critical();
        }
    }

    #[test]
    fn test_message_rate_limiter_reset_all() {
        let mut limiter = MessageRateLimiter::new();

        // Exhaust all limiters
        while limiter.check_volatile() {}
        while limiter.check_normal() {}
        while limiter.check_critical() {}

        limiter.reset_all();

        // Should all work again
        assert!(limiter.check_volatile());
        assert!(limiter.check_normal());
        assert!(limiter.check_critical());
    }

    #[test]
    fn test_message_rate_limiter_default() {
        let limiter = MessageRateLimiter::default();
        assert_eq!(limiter.volatile_limiter.max_tokens(), 120);
        assert_eq!(limiter.normal_limiter.max_tokens(), 60);
        assert_eq!(limiter.critical_limiter.max_tokens(), 20);
    }
}
