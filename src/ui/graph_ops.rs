//! GraphOpsSender helper for reducing code duplication when sending graph operations
//!
//! This module provides a centralized way to send graph operations through LiveShare,
//! reducing boilerplate code across components.

use leptos::prelude::WithUntracked;

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
        if self.ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
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
        self.ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected
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
    // These are unit tests for the helper functions and data conversions

    // ========================================================================
    // Index Conversion Tests
    // ========================================================================

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

    #[test]
    fn test_node_index_zero() {
        let idx: NodeIndex<u32> = NodeIndex::new(0);
        assert_eq!(idx.index() as u32, 0);
    }

    #[test]
    fn test_edge_index_zero() {
        let idx: EdgeIndex<u32> = EdgeIndex::new(0);
        assert_eq!(idx.index() as u32, 0);
    }

    #[test]
    fn test_node_index_large_value() {
        let idx: NodeIndex<u32> = NodeIndex::new(u32::MAX as usize);
        assert_eq!(idx.index() as u32, u32::MAX);
    }

    #[test]
    fn test_edge_index_large_value() {
        let idx: EdgeIndex<u32> = EdgeIndex::new(u32::MAX as usize);
        assert_eq!(idx.index() as u32, u32::MAX);
    }

    // ========================================================================
    // GraphOperation Data Structure Tests
    // ========================================================================

    #[test]
    fn test_graph_operation_create_table_fields() {
        let op = GraphOperation::CreateTable {
            node_id: 5,
            name: "users".to_string(),
            position: (100.0, 200.0),
        };

        match op {
            GraphOperation::CreateTable {
                node_id,
                name,
                position,
            } => {
                assert_eq!(node_id, 5);
                assert_eq!(name, "users");
                assert_eq!(position, (100.0, 200.0));
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_delete_table_fields() {
        let op = GraphOperation::DeleteTable { node_id: 10 };

        match op {
            GraphOperation::DeleteTable { node_id } => {
                assert_eq!(node_id, 10);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_rename_table_fields() {
        let op = GraphOperation::RenameTable {
            node_id: 3,
            new_name: "customers".to_string(),
        };

        match op {
            GraphOperation::RenameTable { node_id, new_name } => {
                assert_eq!(node_id, 3);
                assert_eq!(new_name, "customers");
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_move_table_fields() {
        let op = GraphOperation::MoveTable {
            node_id: 7,
            position: (500.5, 300.5),
        };

        match op {
            GraphOperation::MoveTable { node_id, position } => {
                assert_eq!(node_id, 7);
                assert_eq!(position, (500.5, 300.5));
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_add_column_fields() {
        let column = ColumnData {
            name: "email".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            is_primary_key: false,
            is_nullable: true,
            is_unique: true,
            default_value: None,
            foreign_key: None,
        };

        let op = GraphOperation::AddColumn {
            node_id: 1,
            column: column.clone(),
        };

        match op {
            GraphOperation::AddColumn { node_id, column: c } => {
                assert_eq!(node_id, 1);
                assert_eq!(c.name, "email");
                assert!(c.is_unique);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_update_column_fields() {
        let column = ColumnData {
            name: "username".to_string(),
            data_type: "VARCHAR(100)".to_string(),
            is_primary_key: false,
            is_nullable: false,
            is_unique: true,
            default_value: Some("guest".to_string()),
            foreign_key: None,
        };

        let op = GraphOperation::UpdateColumn {
            node_id: 2,
            column_index: 3,
            column,
        };

        match op {
            GraphOperation::UpdateColumn {
                node_id,
                column_index,
                column: c,
            } => {
                assert_eq!(node_id, 2);
                assert_eq!(column_index, 3);
                assert_eq!(c.default_value, Some("guest".to_string()));
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_delete_column_fields() {
        let op = GraphOperation::DeleteColumn {
            node_id: 4,
            column_index: 2,
        };

        match op {
            GraphOperation::DeleteColumn {
                node_id,
                column_index,
            } => {
                assert_eq!(node_id, 4);
                assert_eq!(column_index, 2);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_create_relationship_fields() {
        let relationship = RelationshipData {
            name: "user_orders".to_string(),
            relationship_type: "one_to_many".to_string(),
            from_column: "id".to_string(),
            to_column: "user_id".to_string(),
        };

        let op = GraphOperation::CreateRelationship {
            edge_id: 100,
            from_node: 1,
            to_node: 2,
            relationship,
        };

        match op {
            GraphOperation::CreateRelationship {
                edge_id,
                from_node,
                to_node,
                relationship: r,
            } => {
                assert_eq!(edge_id, 100);
                assert_eq!(from_node, 1);
                assert_eq!(to_node, 2);
                assert_eq!(r.name, "user_orders");
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_delete_relationship_fields() {
        let op = GraphOperation::DeleteRelationship { edge_id: 55 };

        match op {
            GraphOperation::DeleteRelationship { edge_id } => {
                assert_eq!(edge_id, 55);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    // ========================================================================
    // ColumnData Tests
    // ========================================================================

    #[test]
    fn test_column_data_with_all_fields() {
        let column = ColumnData {
            name: "id".to_string(),
            data_type: "INT".to_string(),
            is_primary_key: true,
            is_nullable: false,
            is_unique: true,
            default_value: None,
            foreign_key: None,
        };

        assert_eq!(column.name, "id");
        assert!(column.is_primary_key);
        assert!(!column.is_nullable);
        assert!(column.is_unique);
    }

    #[test]
    fn test_column_data_with_default_value() {
        let column = ColumnData {
            name: "status".to_string(),
            data_type: "VARCHAR(20)".to_string(),
            is_primary_key: false,
            is_nullable: false,
            is_unique: false,
            default_value: Some("active".to_string()),
            foreign_key: None,
        };

        assert_eq!(column.default_value, Some("active".to_string()));
    }

    // ========================================================================
    // RelationshipData Tests
    // ========================================================================

    #[test]
    fn test_relationship_data_one_to_many() {
        let rel = RelationshipData {
            name: "posts_author".to_string(),
            relationship_type: "one_to_many".to_string(),
            from_column: "id".to_string(),
            to_column: "author_id".to_string(),
        };

        assert_eq!(rel.name, "posts_author");
        assert_eq!(rel.relationship_type, "one_to_many");
    }

    #[test]
    fn test_relationship_data_many_to_many() {
        let rel = RelationshipData {
            name: "users_roles".to_string(),
            relationship_type: "many_to_many".to_string(),
            from_column: "user_id".to_string(),
            to_column: "role_id".to_string(),
        };

        assert_eq!(rel.relationship_type, "many_to_many");
    }

    // ========================================================================
    // Position Coordinate Tests
    // ========================================================================

    #[test]
    fn test_position_tuple_positive() {
        let pos: (f64, f64) = (100.0, 200.0);
        assert_eq!(pos.0, 100.0);
        assert_eq!(pos.1, 200.0);
    }

    #[test]
    fn test_position_tuple_negative() {
        let pos: (f64, f64) = (-50.0, -75.0);
        assert_eq!(pos.0, -50.0);
        assert_eq!(pos.1, -75.0);
    }

    #[test]
    fn test_position_tuple_zero() {
        let pos: (f64, f64) = (0.0, 0.0);
        assert_eq!(pos.0, 0.0);
        assert_eq!(pos.1, 0.0);
    }

    #[test]
    fn test_position_tuple_fractional() {
        let pos: (f64, f64) = (123.456, 789.012);
        assert!((pos.0 - 123.456).abs() < f64::EPSILON);
        assert!((pos.1 - 789.012).abs() < f64::EPSILON);
    }
}
