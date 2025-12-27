//! Cursor Broadcaster for efficient cursor position updates
//!
//! This module provides intelligent broadcasting of cursor positions with spam protection
//! and throttling to prevent network congestion from high-frequency cursor movements.
//!
//! # Overview
//!
//! The `CursorBroadcaster` implements:
//! - Automatic throttling at ~50fps (20ms intervals)
//! - Deduplication of identical positions
//! - Batching of cursor updates when needed
//! - Protection against spam/rapid-fire updates
//!
//! # Usage Example
//!
//! ```rust
//! use archischema::core::liveshare::cursor_broadcaster::CursorBroadcaster;
//!
//! let mut broadcaster = CursorBroadcaster::new();
//!
//! // Update cursor position (may be throttled)
//! if let Some(position) = broadcaster.update_position(100.0, 200.0) {
//!     // Position should be broadcast
//!     // send_to_websocket(position);
//! }
//! ```

use std::time::Duration;

use super::throttling::CursorThrottler;

/// Default minimum distance to consider positions different (in pixels)
const MIN_POSITION_DELTA: f64 = 1.0;

/// Cursor position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
}

impl CursorPosition {
    /// Create a new cursor position
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculate distance to another position
    pub fn distance_to(&self, other: &CursorPosition) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Check if position is significantly different from another
    pub fn is_different_from(&self, other: &CursorPosition, threshold: f64) -> bool {
        self.distance_to(other) >= threshold
    }
}

impl From<(f64, f64)> for CursorPosition {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl From<CursorPosition> for (f64, f64) {
    fn from(pos: CursorPosition) -> Self {
        (pos.x, pos.y)
    }
}

/// Broadcaster for cursor position updates with spam protection
///
/// Manages cursor position broadcasting with intelligent throttling and deduplication.
/// Prevents unnecessary network traffic by only sending meaningful updates.
///
/// # Example
/// ```
/// # use archischema::core::liveshare::cursor_broadcaster::CursorBroadcaster;
/// let mut broadcaster = CursorBroadcaster::new();
///
/// // First position always broadcasts
/// assert!(broadcaster.update_position(100.0, 200.0).is_some());
///
/// // Identical position is deduplicated
/// assert!(broadcaster.update_position(100.0, 200.0).is_none());
///
/// // Similar position (within threshold) may be throttled
/// let result = broadcaster.update_position(100.5, 200.5);
/// ```
#[derive(Debug)]
pub struct CursorBroadcaster {
    /// Throttler for limiting update frequency
    throttler: CursorThrottler,
    /// Last broadcasted position
    last_position: Option<CursorPosition>,
    /// Minimum distance to consider positions different
    position_threshold: f64,
    /// Pending position (waiting to be sent after throttle)
    pending_position: Option<CursorPosition>,
}

impl CursorBroadcaster {
    /// Create a new cursor broadcaster with default settings
    pub fn new() -> Self {
        Self {
            throttler: CursorThrottler::new(),
            last_position: None,
            position_threshold: MIN_POSITION_DELTA,
            pending_position: None,
        }
    }

    /// Create a cursor broadcaster with custom throttle interval
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            throttler: CursorThrottler::with_interval(interval),
            last_position: None,
            position_threshold: MIN_POSITION_DELTA,
            pending_position: None,
        }
    }

    /// Create a cursor broadcaster with custom position threshold
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            throttler: CursorThrottler::new(),
            last_position: None,
            position_threshold: threshold,
            pending_position: None,
        }
    }

    /// Create a cursor broadcaster with custom settings
    pub fn with_settings(interval: Duration, threshold: f64) -> Self {
        Self {
            throttler: CursorThrottler::with_interval(interval),
            last_position: None,
            position_threshold: threshold,
            pending_position: None,
        }
    }

    /// Update cursor position and determine if it should be broadcast
    ///
    /// Returns `Some(position)` if the position should be broadcast,
    /// `None` if it should be skipped (throttled or deduplicated).
    ///
    /// # Arguments
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    ///
    /// # Returns
    /// `Some((x, y))` if broadcast is needed, `None` otherwise
    pub fn update_position(&mut self, x: f64, y: f64) -> Option<(f64, f64)> {
        let new_position = CursorPosition::new(x, y);

        // Check if position is significantly different
        if let Some(last) = self.last_position
            && !new_position.is_different_from(&last, self.position_threshold)
        {
            // Position hasn't changed enough, skip
            return None;
        }

        // Check throttle
        if self.throttler.should_send() {
            // Can send immediately
            self.throttler.mark_sent();
            self.last_position = Some(new_position);
            self.pending_position = None;
            Some((x, y))
        } else {
            // Throttled, save as pending
            self.pending_position = Some(new_position);
            None
        }
    }

    /// Check if there's a pending position that should be sent
    ///
    /// Call this periodically (e.g., in a loop) to flush pending updates
    /// when the throttle interval has elapsed.
    ///
    /// Returns `Some(position)` if a pending position should be sent now.
    pub fn check_pending(&mut self) -> Option<(f64, f64)> {
        if let Some(pending) = self.pending_position
            && self.throttler.should_send()
        {
            self.throttler.mark_sent();
            self.last_position = Some(pending);
            self.pending_position = None;
            return Some(pending.into());
        }
        None
    }

    /// Force send the current pending position (if any)
    ///
    /// Bypasses throttling. Use sparingly, e.g., when user stops moving cursor.
    pub fn flush_pending(&mut self) -> Option<(f64, f64)> {
        self.pending_position.take().map(|pending| {
            self.last_position = Some(pending);
            self.throttler.mark_sent();
            pending.into()
        })
    }

    /// Get the last broadcasted position
    pub fn last_position(&self) -> Option<(f64, f64)> {
        self.last_position.map(|p| p.into())
    }

    /// Check if there's a pending position
    pub fn has_pending(&self) -> bool {
        self.pending_position.is_some()
    }

    /// Reset the broadcaster state
    pub fn reset(&mut self) {
        self.throttler.reset();
        self.last_position = None;
        self.pending_position = None;
    }

    /// Get the configured position threshold
    pub fn position_threshold(&self) -> f64 {
        self.position_threshold
    }

    /// Get the configured throttle interval
    pub fn throttle_interval(&self) -> Duration {
        self.throttler.interval()
    }
}

impl Default for CursorBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cursor_position_new() {
        let pos = CursorPosition::new(10.0, 20.0);
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_cursor_position_distance() {
        let pos1 = CursorPosition::new(0.0, 0.0);
        let pos2 = CursorPosition::new(3.0, 4.0);
        assert_eq!(pos1.distance_to(&pos2), 5.0);
    }

    #[test]
    fn test_cursor_position_is_different() {
        let pos1 = CursorPosition::new(0.0, 0.0);
        let pos2 = CursorPosition::new(0.5, 0.5);
        let pos3 = CursorPosition::new(2.0, 2.0);

        assert!(!pos1.is_different_from(&pos2, 1.0));
        assert!(pos1.is_different_from(&pos3, 1.0));
    }

    #[test]
    fn test_cursor_position_from_tuple() {
        let pos: CursorPosition = (10.0, 20.0).into();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_cursor_position_to_tuple() {
        let pos = CursorPosition::new(10.0, 20.0);
        let tuple: (f64, f64) = pos.into();
        assert_eq!(tuple, (10.0, 20.0));
    }

    #[test]
    fn test_cursor_broadcaster_new() {
        let broadcaster = CursorBroadcaster::new();
        assert!(broadcaster.last_position().is_none());
        assert!(!broadcaster.has_pending());
    }

    #[test]
    fn test_cursor_broadcaster_first_position() {
        let mut broadcaster = CursorBroadcaster::new();
        let result = broadcaster.update_position(100.0, 200.0);
        assert_eq!(result, Some((100.0, 200.0)));
        assert_eq!(broadcaster.last_position(), Some((100.0, 200.0)));
    }

    #[test]
    fn test_cursor_broadcaster_deduplication() {
        let mut broadcaster = CursorBroadcaster::new();
        assert!(broadcaster.update_position(100.0, 200.0).is_some());
        assert!(broadcaster.update_position(100.0, 200.0).is_none());
    }

    #[test]
    fn test_cursor_broadcaster_small_movement() {
        let mut broadcaster = CursorBroadcaster::with_settings(Duration::from_millis(10), 2.0);
        assert!(broadcaster.update_position(100.0, 200.0).is_some());
        // Move less than threshold
        assert!(broadcaster.update_position(100.5, 200.5).is_none());

        // Wait for throttle to allow next update
        thread::sleep(Duration::from_millis(15));

        // Move more than threshold
        assert!(broadcaster.update_position(102.0, 202.0).is_some());
    }

    #[test]
    fn test_cursor_broadcaster_throttling() {
        let mut broadcaster = CursorBroadcaster::with_interval(Duration::from_millis(50));

        // First send works
        assert!(broadcaster.update_position(100.0, 200.0).is_some());

        // Second send immediately is throttled
        let result = broadcaster.update_position(150.0, 250.0);
        assert!(result.is_none());
        assert!(broadcaster.has_pending());
    }

    #[test]
    fn test_cursor_broadcaster_check_pending() {
        let mut broadcaster = CursorBroadcaster::with_interval(Duration::from_millis(10));

        broadcaster.update_position(100.0, 200.0);
        broadcaster.update_position(150.0, 250.0); // Throttled, becomes pending

        assert!(broadcaster.has_pending());

        thread::sleep(Duration::from_millis(15));

        let pending = broadcaster.check_pending();
        assert_eq!(pending, Some((150.0, 250.0)));
        assert!(!broadcaster.has_pending());
    }

    #[test]
    fn test_cursor_broadcaster_flush_pending() {
        let mut broadcaster = CursorBroadcaster::new();

        broadcaster.update_position(100.0, 200.0);
        broadcaster.update_position(150.0, 250.0); // Throttled

        let flushed = broadcaster.flush_pending();
        assert_eq!(flushed, Some((150.0, 250.0)));
        assert!(!broadcaster.has_pending());
    }

    #[test]
    fn test_cursor_broadcaster_reset() {
        let mut broadcaster = CursorBroadcaster::new();
        broadcaster.update_position(100.0, 200.0);
        broadcaster.update_position(150.0, 250.0);

        broadcaster.reset();

        assert!(broadcaster.last_position().is_none());
        assert!(!broadcaster.has_pending());
    }

    #[test]
    fn test_cursor_broadcaster_with_threshold() {
        let broadcaster = CursorBroadcaster::with_threshold(5.0);
        assert_eq!(broadcaster.position_threshold(), 5.0);
    }

    #[test]
    fn test_cursor_broadcaster_with_settings() {
        let broadcaster = CursorBroadcaster::with_settings(Duration::from_millis(100), 5.0);
        assert_eq!(broadcaster.throttle_interval(), Duration::from_millis(100));
        assert_eq!(broadcaster.position_threshold(), 5.0);
    }

    #[test]
    fn test_cursor_broadcaster_default() {
        let broadcaster = CursorBroadcaster::default();
        assert!(broadcaster.last_position().is_none());
    }

    #[test]
    fn test_cursor_broadcaster_multiple_updates() {
        let mut broadcaster = CursorBroadcaster::with_interval(Duration::from_millis(10));

        // First update
        assert!(broadcaster.update_position(0.0, 0.0).is_some());

        // Rapid updates (should be throttled)
        for i in 1..5 {
            let x = i as f64 * 10.0;
            let y = i as f64 * 10.0;
            broadcaster.update_position(x, y);
        }

        // Wait for throttle
        thread::sleep(Duration::from_millis(15));

        // Pending should be available
        let pending = broadcaster.check_pending();
        assert!(pending.is_some());
        assert_eq!(pending.unwrap(), (40.0, 40.0)); // Last position
    }

    #[test]
    fn test_cursor_broadcaster_no_pending_when_no_change() {
        let mut broadcaster = CursorBroadcaster::new();

        broadcaster.update_position(100.0, 200.0);
        broadcaster.update_position(100.0, 200.0); // Same position

        assert!(!broadcaster.has_pending());
    }
}
