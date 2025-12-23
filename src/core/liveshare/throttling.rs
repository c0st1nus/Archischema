//! Throttling mechanisms for LiveShare messages
//!
//! This module provides throttling functionality to limit the rate of outgoing messages,
//! preventing network congestion and reducing server load.
//!
//! # Overview
//!
//! Different types of messages have different throttling requirements:
//! - **Cursor updates**: High frequency (~30fps = 33ms throttle)
//! - **Schema updates**: Lower frequency (~100-300ms throttle)
//! - **Awareness updates**: Batched every 100ms
//!
//! # Usage Example
//!
//! ```rust
//! use archischema::core::liveshare::throttling::{CursorThrottler, SchemaThrottler};
//! use std::time::Duration;
//!
//! let mut cursor_throttler = CursorThrottler::new();
//! let mut schema_throttler = SchemaThrottler::new();
//!
//! // Try to send cursor update
//! if cursor_throttler.should_send() {
//!     // Send cursor position
//!     cursor_throttler.mark_sent();
//! }
//!
//! // Try to send schema update
//! if schema_throttler.should_send() {
//!     // Send schema changes
//!     schema_throttler.mark_sent();
//! }
//! ```

use std::time::{Duration, Instant};

/// Default throttle interval for cursor updates (33ms = ~30fps)
pub const DEFAULT_CURSOR_THROTTLE_MS: u64 = 33;

/// Default throttle interval for schema updates (150ms)
pub const DEFAULT_SCHEMA_THROTTLE_MS: u64 = 150;

/// Default batching interval for awareness updates (100ms)
pub const DEFAULT_AWARENESS_BATCH_MS: u64 = 100;

/// Throttler for cursor position updates
///
/// Limits cursor updates to approximately 30 frames per second to prevent
/// flooding the network with high-frequency position changes.
///
/// # Example
/// ```
/// # use archischema::core::liveshare::throttling::CursorThrottler;
/// let mut throttler = CursorThrottler::new();
///
/// // First call always returns true
/// assert!(throttler.should_send());
/// throttler.mark_sent();
///
/// // Immediate subsequent call returns false
/// assert!(!throttler.should_send());
/// ```
#[derive(Debug, Clone)]
pub struct CursorThrottler {
    last_sent: Option<Instant>,
    interval: Duration,
}

impl CursorThrottler {
    /// Create a new cursor throttler with default interval (33ms)
    pub fn new() -> Self {
        Self {
            last_sent: None,
            interval: Duration::from_millis(DEFAULT_CURSOR_THROTTLE_MS),
        }
    }

    /// Create a cursor throttler with custom interval
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            last_sent: None,
            interval,
        }
    }

    /// Check if a cursor update should be sent
    ///
    /// Returns `true` if enough time has elapsed since the last send,
    /// or if this is the first send.
    pub fn should_send(&self) -> bool {
        if let Some(last) = self.last_sent {
            last.elapsed() >= self.interval
        } else {
            true
        }
    }

    /// Mark that a cursor update was sent
    ///
    /// Should be called after successfully sending a cursor update.
    pub fn mark_sent(&mut self) {
        self.last_sent = Some(Instant::now());
    }

    /// Reset the throttler state
    pub fn reset(&mut self) {
        self.last_sent = None;
    }

    /// Get the configured throttle interval
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Get time since last send
    pub fn time_since_last_send(&self) -> Option<Duration> {
        self.last_sent.map(|last| last.elapsed())
    }
}

impl Default for CursorThrottler {
    fn default() -> Self {
        Self::new()
    }
}

/// Throttler for schema updates (table/relationship changes)
///
/// Limits schema updates to prevent excessive broadcasts during rapid editing.
/// Uses a longer interval than cursor updates since schema changes are less frequent
/// but more important to deliver reliably.
///
/// # Example
/// ```
/// # use archischema::core::liveshare::throttling::SchemaThrottler;
/// let mut throttler = SchemaThrottler::new();
///
/// if throttler.should_send() {
///     // Send schema update
///     throttler.mark_sent();
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SchemaThrottler {
    last_sent: Option<Instant>,
    interval: Duration,
}

impl SchemaThrottler {
    /// Create a new schema throttler with default interval (150ms)
    pub fn new() -> Self {
        Self {
            last_sent: None,
            interval: Duration::from_millis(DEFAULT_SCHEMA_THROTTLE_MS),
        }
    }

    /// Create a schema throttler with custom interval
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            last_sent: None,
            interval,
        }
    }

    /// Check if a schema update should be sent
    pub fn should_send(&self) -> bool {
        if let Some(last) = self.last_sent {
            last.elapsed() >= self.interval
        } else {
            true
        }
    }

    /// Mark that a schema update was sent
    pub fn mark_sent(&mut self) {
        self.last_sent = Some(Instant::now());
    }

    /// Reset the throttler state
    pub fn reset(&mut self) {
        self.last_sent = None;
    }

    /// Get the configured throttle interval
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Get time since last send
    pub fn time_since_last_send(&self) -> Option<Duration> {
        self.last_sent.map(|last| last.elapsed())
    }
}

impl Default for SchemaThrottler {
    fn default() -> Self {
        Self::new()
    }
}

/// Batcher for awareness updates
///
/// Collects awareness state changes and sends them in batches to reduce
/// the number of individual messages. Batches are sent when either:
/// - The batch interval elapses
/// - The batch is manually flushed
///
/// # Example
/// ```
/// # use archischema::core::liveshare::throttling::AwarenessBatcher;
/// # use serde_json::json;
/// let mut batcher = AwarenessBatcher::new();
///
/// // Add awareness states
/// batcher.add("user1".to_string(), json!({"cursor": {"x": 100, "y": 200}}));
/// batcher.add("user2".to_string(), json!({"cursor": {"x": 150, "y": 250}}));
///
/// // Check if batch should be sent
/// if batcher.should_flush() {
///     let batch = batcher.flush();
///     // Send batch...
/// }
/// ```
#[derive(Debug)]
pub struct AwarenessBatcher {
    pending: Vec<(String, serde_json::Value)>,
    last_flush: Option<Instant>,
    interval: Duration,
}

impl AwarenessBatcher {
    /// Create a new awareness batcher with default interval (100ms)
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            last_flush: Some(Instant::now()),
            interval: Duration::from_millis(DEFAULT_AWARENESS_BATCH_MS),
        }
    }

    /// Create an awareness batcher with custom interval
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            pending: Vec::new(),
            last_flush: Some(Instant::now()),
            interval,
        }
    }

    /// Add an awareness state to the batch
    pub fn add(&mut self, user_id: String, state: serde_json::Value) {
        self.pending.push((user_id, state));
    }

    /// Check if the batch should be flushed
    ///
    /// Returns `true` if:
    /// - The batch interval has elapsed
    /// - There are pending updates
    pub fn should_flush(&self) -> bool {
        if self.pending.is_empty() {
            return false;
        }

        if let Some(last) = self.last_flush {
            last.elapsed() >= self.interval
        } else {
            true
        }
    }

    /// Flush the batch and return all pending updates
    ///
    /// Resets the batch state and timer.
    pub fn flush(&mut self) -> Vec<(String, serde_json::Value)> {
        self.last_flush = Some(Instant::now());
        std::mem::take(&mut self.pending)
    }

    /// Get the number of pending updates
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Clear all pending updates without flushing
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Get the configured batch interval
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

impl Default for AwarenessBatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cursor_throttler_new() {
        let throttler = CursorThrottler::new();
        assert_eq!(
            throttler.interval(),
            Duration::from_millis(DEFAULT_CURSOR_THROTTLE_MS)
        );
    }

    #[test]
    fn test_cursor_throttler_first_send() {
        let throttler = CursorThrottler::new();
        assert!(throttler.should_send());
    }

    #[test]
    fn test_cursor_throttler_immediate_resend() {
        let mut throttler = CursorThrottler::new();
        assert!(throttler.should_send());
        throttler.mark_sent();
        assert!(!throttler.should_send());
    }

    #[test]
    fn test_cursor_throttler_after_interval() {
        let mut throttler = CursorThrottler::with_interval(Duration::from_millis(10));
        assert!(throttler.should_send());
        throttler.mark_sent();
        assert!(!throttler.should_send());

        thread::sleep(Duration::from_millis(15));
        assert!(throttler.should_send());
    }

    #[test]
    fn test_cursor_throttler_reset() {
        let mut throttler = CursorThrottler::new();
        throttler.mark_sent();
        assert!(!throttler.should_send());

        throttler.reset();
        assert!(throttler.should_send());
    }

    #[test]
    fn test_cursor_throttler_time_since_last_send() {
        let mut throttler = CursorThrottler::new();
        assert!(throttler.time_since_last_send().is_none());

        throttler.mark_sent();
        thread::sleep(Duration::from_millis(5));

        let elapsed = throttler.time_since_last_send().unwrap();
        assert!(elapsed >= Duration::from_millis(5));
    }

    #[test]
    fn test_schema_throttler_new() {
        let throttler = SchemaThrottler::new();
        assert_eq!(
            throttler.interval(),
            Duration::from_millis(DEFAULT_SCHEMA_THROTTLE_MS)
        );
    }

    #[test]
    fn test_schema_throttler_first_send() {
        let throttler = SchemaThrottler::new();
        assert!(throttler.should_send());
    }

    #[test]
    fn test_schema_throttler_immediate_resend() {
        let mut throttler = SchemaThrottler::new();
        assert!(throttler.should_send());
        throttler.mark_sent();
        assert!(!throttler.should_send());
    }

    #[test]
    fn test_schema_throttler_after_interval() {
        let mut throttler = SchemaThrottler::with_interval(Duration::from_millis(10));
        assert!(throttler.should_send());
        throttler.mark_sent();
        assert!(!throttler.should_send());

        thread::sleep(Duration::from_millis(15));
        assert!(throttler.should_send());
    }

    #[test]
    fn test_schema_throttler_reset() {
        let mut throttler = SchemaThrottler::new();
        throttler.mark_sent();
        assert!(!throttler.should_send());

        throttler.reset();
        assert!(throttler.should_send());
    }

    #[test]
    fn test_awareness_batcher_new() {
        let batcher = AwarenessBatcher::new();
        assert_eq!(
            batcher.interval(),
            Duration::from_millis(DEFAULT_AWARENESS_BATCH_MS)
        );
        assert!(batcher.is_empty());
    }

    #[test]
    fn test_awareness_batcher_add() {
        let mut batcher = AwarenessBatcher::new();
        batcher.add("user1".to_string(), serde_json::json!({"x": 10}));
        assert_eq!(batcher.pending_count(), 1);
        assert!(!batcher.is_empty());
    }

    #[test]
    fn test_awareness_batcher_should_not_flush_empty() {
        let batcher = AwarenessBatcher::new();
        assert!(!batcher.should_flush());
    }

    #[test]
    fn test_awareness_batcher_should_flush_after_interval() {
        let mut batcher = AwarenessBatcher::with_interval(Duration::from_millis(10));
        batcher.add("user1".to_string(), serde_json::json!({"x": 10}));

        assert!(!batcher.should_flush());

        thread::sleep(Duration::from_millis(15));
        assert!(batcher.should_flush());
    }

    #[test]
    fn test_awareness_batcher_flush() {
        let mut batcher = AwarenessBatcher::new();
        batcher.add("user1".to_string(), serde_json::json!({"x": 10}));
        batcher.add("user2".to_string(), serde_json::json!({"x": 20}));

        assert_eq!(batcher.pending_count(), 2);

        let batch = batcher.flush();
        assert_eq!(batch.len(), 2);
        assert_eq!(batcher.pending_count(), 0);
        assert!(batcher.is_empty());
    }

    #[test]
    fn test_awareness_batcher_clear() {
        let mut batcher = AwarenessBatcher::new();
        batcher.add("user1".to_string(), serde_json::json!({"x": 10}));
        batcher.add("user2".to_string(), serde_json::json!({"x": 20}));

        assert_eq!(batcher.pending_count(), 2);

        batcher.clear();
        assert_eq!(batcher.pending_count(), 0);
        assert!(batcher.is_empty());
    }

    #[test]
    fn test_awareness_batcher_multiple_flushes() {
        let mut batcher = AwarenessBatcher::with_interval(Duration::from_millis(10));

        batcher.add("user1".to_string(), serde_json::json!({"x": 10}));
        thread::sleep(Duration::from_millis(15));

        let batch1 = batcher.flush();
        assert_eq!(batch1.len(), 1);

        batcher.add("user2".to_string(), serde_json::json!({"x": 20}));
        thread::sleep(Duration::from_millis(15));

        let batch2 = batcher.flush();
        assert_eq!(batch2.len(), 1);
    }

    #[test]
    fn test_default_implementations() {
        let cursor_throttler = CursorThrottler::default();
        assert_eq!(
            cursor_throttler.interval(),
            Duration::from_millis(DEFAULT_CURSOR_THROTTLE_MS)
        );

        let schema_throttler = SchemaThrottler::default();
        assert_eq!(
            schema_throttler.interval(),
            Duration::from_millis(DEFAULT_SCHEMA_THROTTLE_MS)
        );

        let batcher = AwarenessBatcher::default();
        assert_eq!(
            batcher.interval(),
            Duration::from_millis(DEFAULT_AWARENESS_BATCH_MS)
        );
    }
}
