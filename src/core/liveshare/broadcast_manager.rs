//! Broadcast Manager for incremental updates tracking
//!
//! This module provides functionality for tracking broadcasted versions of graph elements
//! and determining which updates need to be sent to clients.
//!
//! # Overview
//!
//! The `BroadcastManager` implements an efficient incremental update system that:
//! - Tracks version numbers for each graph element (tables and relationships)
//! - Maintains per-user state to know what was already sent
//! - Automatically triggers full syncs every 20 seconds (configurable)
//! - Reduces bandwidth by only sending changed elements
//!
//! # Usage Example
//!
//! ```rust
//! use archischema::core::liveshare::BroadcastManager;
//! use archischema::core::liveshare::broadcast_manager::ElementId;
//!
//! let mut manager = BroadcastManager::new();
//!
//! // Register a user when they join
//! manager.register_user("user123".to_string());
//!
//! // Check if element should be sent
//! let element_id = ElementId::Table(1);
//! if manager.should_send_update("user123", element_id, 5) {
//!     // Send update to user
//!     manager.mark_sent("user123".to_string(), element_id, 5);
//! }
//!
//! // Check if full sync is needed (every 20 seconds)
//! if manager.needs_full_sync("user123") {
//!     // Send full state
//!     manager.mark_full_sync("user123".to_string());
//! }
//! ```
//!
//! # Integration with Room
//!
//! The `Room` struct automatically uses `BroadcastManager` to:
//! - Register users on join via `add_user()`
//! - Unregister users on leave via `remove_user()`
//! - Send incremental updates via `broadcast_incremental_update()`
//! - Send full state to new users via `send_full_graph_state()`

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Identifier for graph elements (tables or relationships)
///
/// Each element in the graph (table or relationship) has a unique ID and version.
/// This enum allows tracking both types uniformly in the broadcast system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementId {
    /// A table node with its node_id
    Table(u32),
    /// A relationship edge with its edge_id
    Relationship(u32),
}

/// Tracks the last broadcasted version for each element per user
#[derive(Debug, Clone)]
struct UserBroadcastState {
    /// Last versions sent to this user: element_id -> version
    last_sent_versions: HashMap<ElementId, u64>,
    /// Last time a full sync was sent to this user
    last_full_sync: Option<Instant>,
}

impl UserBroadcastState {
    fn new() -> Self {
        Self {
            last_sent_versions: HashMap::new(),
            last_full_sync: None,
        }
    }
}

/// Manages broadcast state for incremental updates
///
/// This is the core component for efficient delta synchronization.
/// It tracks what each user has received and determines what needs to be sent.
///
/// # Performance Characteristics
///
/// - Per-user state: O(1) lookup for version checks
/// - Memory usage: ~40 bytes per tracked element per user
/// - Full sync overhead: Negligible (just timestamp comparison)
#[derive(Debug)]
pub struct BroadcastManager {
    /// Tracks broadcast state per user
    user_states: HashMap<String, UserBroadcastState>,
    /// Interval for periodic full sync (default: 20 seconds)
    full_sync_interval: Duration,
}

impl BroadcastManager {
    /// Create a new BroadcastManager with default full sync interval (20 seconds)
    pub fn new() -> Self {
        Self {
            user_states: HashMap::new(),
            full_sync_interval: Duration::from_secs(20),
        }
    }

    /// Create a BroadcastManager with custom full sync interval
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            user_states: HashMap::new(),
            full_sync_interval: interval,
        }
    }

    /// Register a new user (typically when they join the room)
    pub fn register_user(&mut self, user_id: String) {
        self.user_states
            .entry(user_id)
            .or_insert_with(UserBroadcastState::new);
    }

    /// Remove a user (typically when they leave the room)
    pub fn unregister_user(&mut self, user_id: &str) {
        self.user_states.remove(user_id);
    }

    /// Check if a user needs a full sync based on time interval
    pub fn needs_full_sync(&self, user_id: &str) -> bool {
        if let Some(state) = self.user_states.get(user_id) {
            if let Some(last_sync) = state.last_full_sync {
                last_sync.elapsed() >= self.full_sync_interval
            } else {
                // Never synced - needs full sync
                true
            }
        } else {
            // User not registered - needs full sync
            true
        }
    }

    /// Mark that a full sync was sent to a user
    pub fn mark_full_sync(&mut self, user_id: String) {
        let state = self
            .user_states
            .entry(user_id)
            .or_insert_with(UserBroadcastState::new);
        state.last_full_sync = Some(Instant::now());
    }

    /// Check if an element update should be sent to a user
    ///
    /// Returns `true` if:
    /// - The element version is newer than what was last sent to this user
    /// - The element was never sent to this user before
    /// - The user is not registered (defensive behavior)
    ///
    /// # Example
    /// ```
    /// # use archischema::core::liveshare::BroadcastManager;
    /// # use archischema::core::liveshare::broadcast_manager::ElementId;
    /// # let manager = BroadcastManager::new();
    /// let element_id = ElementId::Table(1);
    /// // First time - should send
    /// assert!(manager.should_send_update("user1", element_id, 1));
    /// ```
    pub fn should_send_update(&self, user_id: &str, element_id: ElementId, version: u64) -> bool {
        if let Some(state) = self.user_states.get(user_id) {
            if let Some(&last_version) = state.last_sent_versions.get(&element_id) {
                version > last_version
            } else {
                // Never sent this element to user
                true
            }
        } else {
            // User not registered - should send
            true
        }
    }

    /// Record that an element update was sent to a user
    ///
    /// This should be called after successfully sending an update to track
    /// that the user now has this version of the element.
    pub fn mark_sent(&mut self, user_id: String, element_id: ElementId, version: u64) {
        let state = self
            .user_states
            .entry(user_id)
            .or_insert_with(UserBroadcastState::new);
        state.last_sent_versions.insert(element_id, version);
    }

    /// Record that multiple elements were sent to a user (for batch updates)
    ///
    /// More efficient than calling `mark_sent()` multiple times.
    /// Use this when sending a full snapshot or large batch of updates.
    pub fn mark_batch_sent(&mut self, user_id: String, updates: Vec<(ElementId, u64)>) {
        let state = self
            .user_states
            .entry(user_id)
            .or_insert_with(UserBroadcastState::new);

        for (element_id, version) in updates {
            state.last_sent_versions.insert(element_id, version);
        }
    }

    /// Get the list of element IDs that have changed since last sent to user
    ///
    /// Compares current state with what was last sent and returns only changed elements.
    /// This is the core filtering logic for incremental updates.
    ///
    /// # Arguments
    /// * `user_id` - The user to check
    /// * `current_state` - Current state as (ElementId, version) pairs
    ///
    /// # Returns
    /// Vector of element IDs that have changed (newer version or new element)
    pub fn get_changed_elements(
        &self,
        user_id: &str,
        current_state: &[(ElementId, u64)],
    ) -> Vec<ElementId> {
        if let Some(state) = self.user_states.get(user_id) {
            current_state
                .iter()
                .filter(|(element_id, version)| {
                    if let Some(&last_version) = state.last_sent_versions.get(element_id) {
                        *version > last_version
                    } else {
                        true
                    }
                })
                .map(|(element_id, _)| *element_id)
                .collect()
        } else {
            // User not registered - all elements are changed
            current_state.iter().map(|(id, _)| *id).collect()
        }
    }

    /// Reset a user's state (useful for forcing a full resync)
    pub fn reset_user(&mut self, user_id: &str) {
        if let Some(state) = self.user_states.get_mut(user_id) {
            state.last_sent_versions.clear();
            state.last_full_sync = None;
        }
    }

    /// Get the number of tracked users
    pub fn user_count(&self) -> usize {
        self.user_states.len()
    }

    /// Check if a user is registered
    pub fn has_user(&self, user_id: &str) -> bool {
        self.user_states.contains_key(user_id)
    }
}

impl Default for BroadcastManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_manager_new() {
        let manager = BroadcastManager::new();
        assert_eq!(manager.user_count(), 0);
        assert_eq!(manager.full_sync_interval, Duration::from_secs(20));
    }

    #[test]
    fn test_broadcast_manager_with_interval() {
        let manager = BroadcastManager::with_interval(Duration::from_secs(30));
        assert_eq!(manager.full_sync_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_register_unregister_user() {
        let mut manager = BroadcastManager::new();

        manager.register_user("user1".to_string());
        assert_eq!(manager.user_count(), 1);
        assert!(manager.has_user("user1"));

        manager.register_user("user2".to_string());
        assert_eq!(manager.user_count(), 2);

        manager.unregister_user("user1");
        assert_eq!(manager.user_count(), 1);
        assert!(!manager.has_user("user1"));
        assert!(manager.has_user("user2"));
    }

    #[test]
    fn test_needs_full_sync_new_user() {
        let manager = BroadcastManager::new();
        assert!(manager.needs_full_sync("user1"));
    }

    #[test]
    fn test_mark_full_sync() {
        let mut manager = BroadcastManager::new();
        manager.register_user("user1".to_string());

        assert!(manager.needs_full_sync("user1"));

        manager.mark_full_sync("user1".to_string());
        assert!(!manager.needs_full_sync("user1"));
    }

    #[test]
    fn test_needs_full_sync_after_interval() {
        let mut manager = BroadcastManager::with_interval(Duration::from_millis(10));
        manager.register_user("user1".to_string());
        manager.mark_full_sync("user1".to_string());

        assert!(!manager.needs_full_sync("user1"));

        std::thread::sleep(Duration::from_millis(15));
        assert!(manager.needs_full_sync("user1"));
    }

    #[test]
    fn test_should_send_update_new_element() {
        let manager = BroadcastManager::new();
        let element_id = ElementId::Table(1);

        assert!(manager.should_send_update("user1", element_id, 1));
    }

    #[test]
    fn test_should_send_update_newer_version() {
        let mut manager = BroadcastManager::new();
        manager.register_user("user1".to_string());

        let element_id = ElementId::Table(1);
        manager.mark_sent("user1".to_string(), element_id, 5);

        assert!(!manager.should_send_update("user1", element_id, 3));
        assert!(!manager.should_send_update("user1", element_id, 5));
        assert!(manager.should_send_update("user1", element_id, 6));
    }

    #[test]
    fn test_mark_batch_sent() {
        let mut manager = BroadcastManager::new();
        manager.register_user("user1".to_string());

        let updates = vec![
            (ElementId::Table(1), 10),
            (ElementId::Table(2), 5),
            (ElementId::Relationship(1), 3),
        ];

        manager.mark_batch_sent("user1".to_string(), updates);

        assert!(!manager.should_send_update("user1", ElementId::Table(1), 10));
        assert!(manager.should_send_update("user1", ElementId::Table(1), 11));
        assert!(!manager.should_send_update("user1", ElementId::Table(2), 5));
        assert!(!manager.should_send_update("user1", ElementId::Relationship(1), 3));
    }

    #[test]
    fn test_get_changed_elements() {
        let mut manager = BroadcastManager::new();
        manager.register_user("user1".to_string());

        manager.mark_sent("user1".to_string(), ElementId::Table(1), 5);
        manager.mark_sent("user1".to_string(), ElementId::Table(2), 3);

        let current_state = vec![
            (ElementId::Table(1), 7), // Changed
            (ElementId::Table(2), 3), // Same
            (ElementId::Table(3), 1), // New
        ];

        let changed = manager.get_changed_elements("user1", &current_state);
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&ElementId::Table(1)));
        assert!(changed.contains(&ElementId::Table(3)));
    }

    #[test]
    fn test_get_changed_elements_unregistered_user() {
        let manager = BroadcastManager::new();

        let current_state = vec![(ElementId::Table(1), 7), (ElementId::Table(2), 3)];

        let changed = manager.get_changed_elements("user1", &current_state);
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn test_reset_user() {
        let mut manager = BroadcastManager::new();
        manager.register_user("user1".to_string());

        manager.mark_sent("user1".to_string(), ElementId::Table(1), 5);
        manager.mark_full_sync("user1".to_string());

        assert!(!manager.should_send_update("user1", ElementId::Table(1), 5));
        assert!(!manager.needs_full_sync("user1"));

        manager.reset_user("user1");

        assert!(manager.should_send_update("user1", ElementId::Table(1), 5));
        assert!(manager.needs_full_sync("user1"));
    }

    #[test]
    fn test_element_id_equality() {
        let table1 = ElementId::Table(1);
        let table1_copy = ElementId::Table(1);
        let table2 = ElementId::Table(2);
        let rel1 = ElementId::Relationship(1);

        assert_eq!(table1, table1_copy);
        assert_ne!(table1, table2);
        assert_ne!(table1, rel1);
    }
}
