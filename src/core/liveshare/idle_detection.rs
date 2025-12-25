//! Idle detection system for LiveShare
//!
//! This module provides functionality for detecting when users are idle, away, or active.
//! It tracks activity events and page visibility to manage user presence status.

use std::time::{Duration, Instant};

/// User activity status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityStatus {
    /// User is actively working
    Active,
    /// User has been idle for 30 seconds
    Idle,
    /// User's tab is not visible or they've been away for more than 30 seconds from idle
    Away,
}

impl ActivityStatus {
    /// Convert to boolean for protocol (is_active)
    pub fn to_is_active(&self) -> bool {
        matches!(self, ActivityStatus::Active)
    }

    /// Get display string
    pub fn display_name(&self) -> &'static str {
        match self {
            ActivityStatus::Active => "Active",
            ActivityStatus::Idle => "Idle",
            ActivityStatus::Away => "Away",
        }
    }
}

/// Idle detector for tracking user activity
pub struct IdleDetector {
    /// Last time user activity was detected
    last_activity_time: Instant,
    /// Current activity status
    status: ActivityStatus,
    /// Threshold for considering user idle (30 seconds)
    idle_threshold: Duration,
    /// Threshold for considering user away (10 minutes from idle state)
    away_threshold: Duration,
}

impl IdleDetector {
    /// Create a new idle detector
    pub fn new() -> Self {
        Self {
            last_activity_time: Instant::now(),
            status: ActivityStatus::Active,
            idle_threshold: Duration::from_secs(30),
            away_threshold: Duration::from_secs(600), // 10 minutes
        }
    }

    /// Record user activity
    pub fn record_activity(&mut self) {
        self.last_activity_time = Instant::now();
        if self.status != ActivityStatus::Active {
            self.status = ActivityStatus::Active;
        }
    }

    /// Record that the page is hidden/not visible
    pub fn record_page_hidden(&mut self) {
        self.status = ActivityStatus::Away;
    }

    /// Record that the page is visible again
    pub fn record_page_visible(&mut self) {
        // When page becomes visible, reset to active if they were away
        if self.status == ActivityStatus::Away {
            self.last_activity_time = Instant::now();
            self.status = ActivityStatus::Active;
        }
    }

    /// Update status based on elapsed time
    ///
    /// Returns true if status changed, false otherwise
    pub fn update(&mut self) -> bool {
        // If already away (from page visibility), don't change
        if self.status == ActivityStatus::Away {
            return false;
        }

        let elapsed = self.last_activity_time.elapsed();

        let new_status = if elapsed >= self.away_threshold {
            ActivityStatus::Away
        } else if elapsed >= self.idle_threshold {
            ActivityStatus::Idle
        } else {
            ActivityStatus::Active
        };

        if new_status != self.status {
            self.status = new_status;
            return true;
        }

        false
    }

    /// Get current activity status
    pub fn status(&self) -> ActivityStatus {
        self.status
    }

    /// Get elapsed time since last activity
    pub fn elapsed_since_activity(&self) -> Duration {
        self.last_activity_time.elapsed()
    }
}

impl Default for IdleDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_status() {
        let detector = IdleDetector::new();
        assert_eq!(detector.status(), ActivityStatus::Active);
    }

    #[test]
    fn test_record_activity() {
        let mut detector = IdleDetector::new();
        detector.status = ActivityStatus::Idle;
        detector.record_activity();
        assert_eq!(detector.status(), ActivityStatus::Active);
    }

    #[test]
    fn test_page_hidden_changes_to_away() {
        let mut detector = IdleDetector::new();
        detector.record_page_hidden();
        assert_eq!(detector.status(), ActivityStatus::Away);
    }

    #[test]
    fn test_page_visible_resets_to_active() {
        let mut detector = IdleDetector::new();
        detector.record_page_hidden();
        assert_eq!(detector.status(), ActivityStatus::Away);

        detector.record_page_visible();
        assert_eq!(detector.status(), ActivityStatus::Active);
    }

    #[test]
    fn test_activity_status_display() {
        assert_eq!(ActivityStatus::Active.display_name(), "Active");
        assert_eq!(ActivityStatus::Idle.display_name(), "Idle");
        assert_eq!(ActivityStatus::Away.display_name(), "Away");
    }

    #[test]
    fn test_activity_status_to_is_active() {
        assert!(ActivityStatus::Active.to_is_active());
        assert!(!ActivityStatus::Idle.to_is_active());
        assert!(!ActivityStatus::Away.to_is_active());
    }
}
