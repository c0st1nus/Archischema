//! Reconciliation algorithm for merging graph elements
//!
//! Provides conflict resolution based on version numbers and timestamps (Last-Write-Wins).
//! Supports soft-deletion using tombstones.

use super::protocol::{GraphStateSnapshot, RelationshipSnapshot, TableSnapshot};
use std::collections::HashMap;

/// Action to take after reconciliation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationAction {
    /// Keep the local version
    KeepLocal,
    /// Update with the remote version
    UpdateFromRemote,
    /// Both are identical, no action needed
    NoAction,
}

/// Helper to get current Unix timestamp in milliseconds
pub fn current_timestamp() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }
}

/// Trait for elements that can be reconciled
pub trait Reconcilable {
    fn version(&self) -> u64;
    fn last_modified_at(&self) -> i64;
    fn is_deleted(&self) -> bool;
}

impl Reconcilable for TableSnapshot {
    fn version(&self) -> u64 {
        self.version
    }
    fn last_modified_at(&self) -> i64 {
        self.last_modified_at
    }
    fn is_deleted(&self) -> bool {
        self.is_deleted
    }
}

impl Reconcilable for RelationshipSnapshot {
    fn version(&self) -> u64 {
        self.version
    }
    fn last_modified_at(&self) -> i64 {
        self.last_modified_at
    }
    fn is_deleted(&self) -> bool {
        self.is_deleted
    }
}

/// Reconciles a local element with a remote update.
///
/// Uses Last-Write-Wins strategy:
/// 1. Higher version wins.
/// 2. If versions are equal, higher timestamp wins.
/// 3. If everything is equal, no action is needed.
pub fn reconcile_element<T: Reconcilable>(local: &T, remote: &T) -> ReconciliationAction {
    if remote.version() > local.version() {
        return ReconciliationAction::UpdateFromRemote;
    }

    if remote.version() < local.version() {
        return ReconciliationAction::KeepLocal;
    }

    // Versions are equal, check timestamps
    if remote.last_modified_at() > local.last_modified_at() {
        return ReconciliationAction::UpdateFromRemote;
    }

    if remote.last_modified_at() < local.last_modified_at() {
        return ReconciliationAction::KeepLocal;
    }

    // Everything is equal (or we can't tell the difference)
    ReconciliationAction::NoAction
}

/// Reconciles a list of elements (e.g. from a GraphStateSnapshot)
pub fn reconcile_elements<T>(
    local_elements: &mut HashMap<u32, T>,
    remote_elements: Vec<T>,
    get_id: fn(&T) -> u32,
) -> Vec<u32>
where
    T: Reconcilable + Clone,
{
    let mut updated_ids = Vec::new();

    for remote in remote_elements {
        let id = get_id(&remote);

        if let Some(local) = local_elements.get(&id) {
            match reconcile_element(local, &remote) {
                ReconciliationAction::UpdateFromRemote => {
                    local_elements.insert(id, remote);
                    updated_ids.push(id);
                }
                ReconciliationAction::KeepLocal | ReconciliationAction::NoAction => {}
            }
        } else {
            // New element from remote
            local_elements.insert(id, remote);
            updated_ids.push(id);
        }
    }

    updated_ids
}

/// Reconciles a full state snapshot
pub fn reconcile_snapshot(
    current: &mut GraphStateSnapshot,
    remote: GraphStateSnapshot,
) -> (Vec<u32>, Vec<u32>) {
    let mut tables_map: HashMap<u32, TableSnapshot> =
        current.tables.drain(..).map(|t| (t.node_id, t)).collect();
    let mut rels_map: HashMap<u32, RelationshipSnapshot> = current
        .relationships
        .drain(..)
        .map(|r| (r.edge_id, r))
        .collect();

    let updated_tables = reconcile_elements(&mut tables_map, remote.tables, |t| t.node_id);
    let updated_rels = reconcile_elements(&mut rels_map, remote.relationships, |r| r.edge_id);

    current.tables = tables_map.into_values().collect();
    current.relationships = rels_map.into_values().collect();

    (updated_tables, updated_rels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_table(id: u32, version: u64, ts: i64) -> TableSnapshot {
        TableSnapshot {
            node_id: id,
            table_uuid: uuid::Uuid::new_v4(),
            name: format!("table_{}", id),
            position: (0.0, 0.0),
            columns: vec![],
            version,
            last_modified_at: ts,
            is_deleted: false,
        }
    }

    #[test]
    fn test_reconcile_higher_version_wins() {
        let local = create_test_table(1, 1, 100);
        let remote = create_test_table(1, 2, 50); // Higher version, lower timestamp

        assert_eq!(
            reconcile_element(&local, &remote),
            ReconciliationAction::UpdateFromRemote
        );
    }

    #[test]
    fn test_reconcile_same_version_higher_timestamp_wins() {
        let local = create_test_table(1, 1, 100);
        let remote = create_test_table(1, 1, 150); // Same version, higher timestamp

        assert_eq!(
            reconcile_element(&local, &remote),
            ReconciliationAction::UpdateFromRemote
        );
    }

    #[test]
    fn test_reconcile_same_version_lower_timestamp_loses() {
        let local = create_test_table(1, 1, 100);
        let remote = create_test_table(1, 1, 50); // Same version, lower timestamp

        assert_eq!(
            reconcile_element(&local, &remote),
            ReconciliationAction::KeepLocal
        );
    }

    #[test]
    fn test_reconcile_identical_no_action() {
        let local = create_test_table(1, 1, 100);
        let remote = create_test_table(1, 1, 100);

        assert_eq!(
            reconcile_element(&local, &remote),
            ReconciliationAction::NoAction
        );
    }

    #[test]
    fn test_reconcile_elements_list() {
        use std::collections::HashMap;

        let mut local_map = HashMap::new();
        local_map.insert(1, create_test_table(1, 1, 100));
        local_map.insert(2, create_test_table(2, 5, 200));

        let remote_list = vec![
            create_test_table(1, 2, 110), // Update
            create_test_table(2, 4, 300), // Ignore (lower version)
            create_test_table(3, 1, 50),  // New
        ];

        let updated = reconcile_elements(&mut local_map, remote_list, |t| t.node_id);

        assert_eq!(updated.len(), 2);
        assert!(updated.contains(&1));
        assert!(updated.contains(&3));

        assert_eq!(local_map.get(&1).unwrap().version, 2);
        assert_eq!(local_map.get(&2).unwrap().version, 5);
        assert!(local_map.contains_key(&3));
    }
}
