//! GraphOpsSender helper for reducing code duplication when sending graph operations
//!
//! This module provides a centralized way to send graph operations through LiveShare,
//! reducing boilerplate code across components.

use leptos::prelude::GetUntracked;

use crate::ui::liveshare_client::{
    ColumnData, ConnectionState, GraphOperation, LiveShareContext, RelationshipData,
};
use petgraph::graph::{EdgeIndex, NodeIndex};

/// Helper struct for sending graph operations through LiveShare
///
/// Encapsulates the common pattern of checking connection state before sending operations.
#[derive(Clone, Copy)]
pub struct GraphOpsSender {
    ctx: LiveShareContext,
}

impl GraphOpsSender {
    /// Create a new GraphOpsSender with the given LiveShare context
    pub fn new(ctx: LiveShareContext) -> Self {
        Self { ctx }
    }

    /// Check if connected and send the operation
    #[inline]
    fn send(&self, op: GraphOperation) {
        if self.ctx.connection_state.get_untracked() == ConnectionState::Connected {
            self.ctx.send_graph_op(op);
        }
    }

    /// Send a CreateTable operation
    pub fn create_table(&self, node_idx: NodeIndex, name: String, position: (f64, f64)) {
        self.send(GraphOperation::CreateTable {
            node_id: node_idx.index() as u32,
            name,
            position,
        });
    }

    /// Send a DeleteTable operation
    pub fn delete_table(&self, node_idx: NodeIndex) {
        self.send(GraphOperation::DeleteTable {
            node_id: node_idx.index() as u32,
        });
    }

    /// Send a RenameTable operation
    pub fn rename_table(&self, node_idx: NodeIndex, new_name: String) {
        self.send(GraphOperation::RenameTable {
            node_id: node_idx.index() as u32,
            new_name,
        });
    }

    /// Send a MoveTable operation
    pub fn move_table(&self, node_idx: NodeIndex, position: (f64, f64)) {
        self.send(GraphOperation::MoveTable {
            node_id: node_idx.index() as u32,
            position,
        });
    }

    /// Send an AddColumn operation
    pub fn add_column(&self, node_idx: NodeIndex, column: ColumnData) {
        self.send(GraphOperation::AddColumn {
            node_id: node_idx.index() as u32,
            column,
        });
    }

    /// Send an UpdateColumn operation
    pub fn update_column(&self, node_idx: NodeIndex, column_index: usize, column: ColumnData) {
        self.send(GraphOperation::UpdateColumn {
            node_id: node_idx.index() as u32,
            column_index,
            column,
        });
    }

    /// Send a DeleteColumn operation
    pub fn delete_column(&self, node_idx: NodeIndex, column_index: usize) {
        self.send(GraphOperation::DeleteColumn {
            node_id: node_idx.index() as u32,
            column_index,
        });
    }

    /// Send a CreateRelationship operation
    pub fn create_relationship(
        &self,
        edge_idx: EdgeIndex,
        from_node: NodeIndex,
        to_node: NodeIndex,
        relationship: RelationshipData,
    ) {
        self.send(GraphOperation::CreateRelationship {
            edge_id: edge_idx.index() as u32,
            from_node: from_node.index() as u32,
            to_node: to_node.index() as u32,
            relationship,
        });
    }

    /// Send a DeleteRelationship operation
    pub fn delete_relationship(&self, edge_idx: EdgeIndex) {
        self.send(GraphOperation::DeleteRelationship {
            edge_id: edge_idx.index() as u32,
        });
    }

    /// Check if currently connected to a LiveShare room
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.ctx.connection_state.get_untracked() == ConnectionState::Connected
    }
}

/// Convenience function to create a GraphOpsSender from a LiveShareContext
pub fn use_graph_ops(ctx: LiveShareContext) -> GraphOpsSender {
    GraphOpsSender::new(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests would require mocking LiveShareContext
    // These are placeholder tests for the helper functions

    #[test]
    fn test_node_index_conversion() {
        let idx: NodeIndex<u32> = NodeIndex::new(42);
        assert_eq!(idx.index() as u32, 42);
    }

    #[test]
    fn test_edge_index_conversion() {
        let idx: EdgeIndex<u32> = EdgeIndex::new(99);
        assert_eq!(idx.index() as u32, 99);
    }
}
