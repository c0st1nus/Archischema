//! Periodic snapshot management for LiveShare sessions
//!
//! This module provides functionality to:
//! - Create periodic snapshots of session state
//! - Serialize SchemaGraph to binary format
//! - Recover state from snapshots on reconnection
//! - Clean up old snapshots automatically

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::protocol::{GraphStateSnapshot, RoomId};

// ============================================================================
// Constants
// ============================================================================

/// Interval between snapshots (20-30 seconds)
pub const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(25);

/// Maximum snapshot size (10 MB)
pub const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024;

/// Number of snapshots to keep per session
pub const SNAPSHOTS_TO_KEEP: usize = 10;

// ============================================================================
// Snapshot Structure
// ============================================================================

/// Represents a saved snapshot of room state
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Unique snapshot identifier
    pub id: Uuid,
    /// Associated room/session ID
    pub room_id: RoomId,
    /// Serialized state data
    pub data: Vec<u8>,
    /// When this snapshot was created
    pub created_at: DateTime<Utc>,
    /// Size of the snapshot in bytes
    pub size_bytes: usize,
    /// Number of elements in the snapshot
    pub element_count: usize,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(room_id: RoomId, data: Vec<u8>, element_count: usize) -> Self {
        let size_bytes = data.len();

        Self {
            id: Uuid::new_v4(),
            room_id,
            data,
            created_at: Utc::now(),
            size_bytes,
            element_count,
        }
    }

    /// Check if snapshot size is within acceptable limits
    pub fn is_valid(&self) -> bool {
        self.size_bytes > 0 && self.size_bytes <= MAX_SNAPSHOT_SIZE
    }
}

// ============================================================================
// Snapshot Serialization
// ============================================================================

/// Serializes and deserializes GraphStateSnapshot
pub struct SnapshotCodec;

impl SnapshotCodec {
    /// Serialize a GraphStateSnapshot to bytes
    pub fn serialize(state: &GraphStateSnapshot) -> Result<Vec<u8>, String> {
        // Serialize using JSON as fallback until bincode is available
        // In production, consider using serde_json or adding bincode to Cargo.toml
        match serde_json::to_vec(state) {
            Ok(bytes) => {
                if bytes.len() > MAX_SNAPSHOT_SIZE {
                    Err(format!(
                        "Snapshot too large: {} bytes (max: {})",
                        bytes.len(),
                        MAX_SNAPSHOT_SIZE
                    ))
                } else {
                    Ok(bytes)
                }
            }
            Err(e) => Err(format!("Serialization failed: {}", e)),
        }
    }

    /// Deserialize bytes back to GraphStateSnapshot
    pub fn deserialize(bytes: &[u8]) -> Result<GraphStateSnapshot, String> {
        match serde_json::from_slice::<GraphStateSnapshot>(bytes) {
            Ok(state) => Ok(state),
            Err(e) => Err(format!("Deserialization failed: {}", e)),
        }
    }
}

// ============================================================================
// Snapshot Manager
// ============================================================================

/// Manages periodic snapshots for a room
pub struct SnapshotManager {
    /// Room ID this manager is responsible for
    room_id: RoomId,
    /// Last snapshot creation time
    last_snapshot: RwLock<Option<Instant>>,
    /// In-memory snapshot cache (most recent snapshots)
    snapshots: Arc<RwLock<Vec<Snapshot>>>,
}

impl SnapshotManager {
    /// Create a new snapshot manager for a room
    pub fn new(room_id: RoomId) -> Self {
        Self {
            room_id,
            last_snapshot: RwLock::new(None),
            snapshots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if a new snapshot should be created
    pub async fn should_snapshot(&self) -> bool {
        let last = self.last_snapshot.read().await;
        match *last {
            None => true,
            Some(instant) => instant.elapsed() >= SNAPSHOT_INTERVAL,
        }
    }

    /// Create a new snapshot of the given state
    pub async fn create_snapshot(&self, state: &GraphStateSnapshot) -> Result<Snapshot, String> {
        // Serialize state
        let data = SnapshotCodec::serialize(state)?;

        let element_count = state.tables.len() + state.relationships.len();
        let snapshot = Snapshot::new(self.room_id, data, element_count);

        if !snapshot.is_valid() {
            return Err("Invalid snapshot".to_string());
        }

        // Update last snapshot time
        *self.last_snapshot.write().await = Some(Instant::now());

        // Store in cache
        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot.clone());

        // Keep only the latest snapshots
        if snapshots.len() > SNAPSHOTS_TO_KEEP {
            let to_remove = snapshots.len() - SNAPSHOTS_TO_KEEP;
            snapshots.drain(0..to_remove);
        }

        Ok(snapshot)
    }

    /// Get the most recent snapshot
    pub async fn get_latest_snapshot(&self) -> Option<Snapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.last().cloned()
    }

    /// Get the latest N snapshots
    pub async fn get_recent_snapshots(&self, count: usize) -> Vec<Snapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.iter().rev().take(count).cloned().collect()
    }

    /// Restore state from the latest snapshot
    pub async fn restore_from_latest(&self) -> Result<GraphStateSnapshot, String> {
        let snapshots = self.snapshots.read().await;
        let snapshot = snapshots.last().ok_or("No snapshots available")?;

        SnapshotCodec::deserialize(&snapshot.data)
    }

    /// Clear all snapshots (useful for cleanup)
    pub async fn clear_snapshots(&self) {
        let mut snapshots = self.snapshots.write().await;
        snapshots.clear();
    }

    /// Get snapshot statistics
    pub async fn get_stats(&self) -> SnapshotStats {
        let snapshots = self.snapshots.read().await;

        let total_size: usize = snapshots.iter().map(|s| s.size_bytes).sum();
        let avg_size = if snapshots.is_empty() {
            0
        } else {
            total_size / snapshots.len()
        };

        SnapshotStats {
            total_snapshots: snapshots.len(),
            total_size_bytes: total_size,
            avg_size_bytes: avg_size,
            oldest_snapshot_age: snapshots
                .first()
                .map(|s| Utc::now().signed_duration_since(s.created_at).num_seconds()),
            newest_snapshot_age: snapshots
                .last()
                .map(|s| Utc::now().signed_duration_since(s.created_at).num_seconds()),
        }
    }
}

/// Statistics about snapshots
#[derive(Debug, Clone)]
pub struct SnapshotStats {
    pub total_snapshots: usize,
    pub total_size_bytes: usize,
    pub avg_size_bytes: usize,
    pub oldest_snapshot_age: Option<i64>,
    pub newest_snapshot_age: Option<i64>,
}

// ============================================================================
// Global Snapshot Registry
// ============================================================================

/// Registry for managing snapshot managers across all rooms
pub struct SnapshotRegistry {
    managers: DashMap<RoomId, Arc<SnapshotManager>>,
}

impl SnapshotRegistry {
    /// Create a new snapshot registry
    pub fn new() -> Self {
        Self {
            managers: DashMap::new(),
        }
    }

    /// Get or create a snapshot manager for a room
    pub fn get_or_create(&self, room_id: RoomId) -> Arc<SnapshotManager> {
        self.managers
            .entry(room_id)
            .or_insert_with(|| Arc::new(SnapshotManager::new(room_id)))
            .clone()
    }

    /// Get existing manager (without creating)
    pub fn get(&self, room_id: &RoomId) -> Option<Arc<SnapshotManager>> {
        self.managers.get(room_id).map(|entry| entry.clone())
    }

    /// Remove a manager (when room is deleted)
    pub fn remove(&self, room_id: &RoomId) {
        self.managers.remove(room_id);
    }

    /// Get snapshot statistics for all rooms
    pub async fn get_global_stats(&self) -> GlobalSnapshotStats {
        let mut total_snapshots = 0;
        let mut total_size_bytes = 0;
        let mut rooms_with_snapshots = 0;

        for entry in self.managers.iter() {
            let manager = entry.value();
            let stats = manager.get_stats().await;

            if stats.total_snapshots > 0 {
                total_snapshots += stats.total_snapshots;
                total_size_bytes += stats.total_size_bytes;
                rooms_with_snapshots += 1;
            }
        }

        GlobalSnapshotStats {
            total_snapshots,
            total_size_bytes,
            rooms_with_snapshots,
        }
    }
}

impl Default for SnapshotRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global snapshot statistics
#[derive(Debug, Clone)]
pub struct GlobalSnapshotStats {
    pub total_snapshots: usize,
    pub total_size_bytes: usize,
    pub rooms_with_snapshots: usize,
}

// ============================================================================
// Periodic Snapshot Task
// ============================================================================

/// Background task for creating periodic snapshots
#[allow(dead_code)]
pub struct SnapshotTask {
    registry: Arc<SnapshotRegistry>,
}

impl SnapshotTask {
    /// Create a new snapshot task
    pub fn new(registry: Arc<SnapshotRegistry>) -> Self {
        Self { registry }
    }

    /// Start the periodic snapshot task
    pub async fn start(self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(SNAPSHOT_INTERVAL);

            loop {
                interval.tick().await;

                // Note: This is a placeholder - actual implementation would
                // require access to room states. Integration with Room struct
                // is needed to make this work.
                tracing::debug!("Snapshot task tick - ready for snapshot creation");
            }
        });
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_state() -> GraphStateSnapshot {
        GraphStateSnapshot {
            tables: Vec::new(),
            relationships: Vec::new(),
        }
    }

    #[test]
    fn test_snapshot_creation() {
        let state = create_sample_state();
        let data = SnapshotCodec::serialize(&state).unwrap();

        assert!(!data.is_empty());
    }

    #[test]
    fn test_snapshot_serialization_roundtrip() {
        let original = create_sample_state();
        let data = SnapshotCodec::serialize(&original).unwrap();
        let restored = SnapshotCodec::deserialize(&data).unwrap();

        assert_eq!(original.tables.len(), restored.tables.len());
        assert_eq!(original.relationships.len(), restored.relationships.len());
    }

    #[tokio::test]
    async fn test_snapshot_manager_creation() {
        let room_id = Uuid::new_v4();
        let manager = SnapshotManager::new(room_id);

        let state = create_sample_state();
        let snapshot = manager.create_snapshot(&state).await.unwrap();

        assert_eq!(snapshot.room_id, room_id);
        assert!(snapshot.is_valid());
    }

    #[tokio::test]
    async fn test_snapshot_manager_should_snapshot() {
        let room_id = Uuid::new_v4();
        let manager = SnapshotManager::new(room_id);

        // First snapshot should always be created
        assert!(manager.should_snapshot().await);

        let state = create_sample_state();
        let _ = manager.create_snapshot(&state).await.unwrap();

        // Should not create again immediately
        assert!(!manager.should_snapshot().await);
    }

    #[tokio::test]
    async fn test_snapshot_manager_latest() {
        let room_id = Uuid::new_v4();
        let manager = SnapshotManager::new(room_id);

        let state = create_sample_state();
        let snapshot1 = manager.create_snapshot(&state).await.unwrap();

        // Wait a tiny bit to ensure different timestamps
        tokio::time::sleep(Duration::from_millis(10)).await;
        let snapshot2 = manager.create_snapshot(&state).await.unwrap();

        let latest = manager.get_latest_snapshot().await.unwrap();
        assert_eq!(latest.id, snapshot2.id);
        assert_ne!(snapshot1.id, snapshot2.id);
    }

    #[tokio::test]
    async fn test_snapshot_manager_capacity() {
        let room_id = Uuid::new_v4();
        let manager = SnapshotManager::new(room_id);

        let state = create_sample_state();

        // Create more snapshots than capacity
        for _ in 0..SNAPSHOTS_TO_KEEP + 5 {
            let _ = manager.create_snapshot(&state).await.unwrap();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Should only keep latest SNAPSHOTS_TO_KEEP
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_snapshots, SNAPSHOTS_TO_KEEP);
    }

    #[tokio::test]
    async fn test_snapshot_registry() {
        let registry = SnapshotRegistry::new();
        let room_id = Uuid::new_v4();

        let manager1 = registry.get_or_create(room_id);
        let manager2 = registry.get_or_create(room_id);

        // Should return the same manager
        assert_eq!(Arc::as_ptr(&manager1), Arc::as_ptr(&manager2));
    }

    #[tokio::test]
    async fn test_snapshot_restore() {
        let room_id = Uuid::new_v4();
        let manager = SnapshotManager::new(room_id);

        let original = create_sample_state();
        let _ = manager.create_snapshot(&original).await.unwrap();

        let restored = manager.restore_from_latest().await.unwrap();
        assert_eq!(original.tables.len(), restored.tables.len());
    }
}
