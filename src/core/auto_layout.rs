//! Auto-layout module for automatic table arrangement
//!
//! This module provides a force-directed layout algorithm that arranges tables
//! so that related tables (connected by foreign keys) are positioned close together,
//! making relationships easy to understand visually.
//!
//! The algorithm simulates physical forces:
//! - **Attraction**: Connected tables are pulled together (like springs)
//! - **Repulsion**: All tables push each other apart (like electric charges)
//! - **Centering**: Tables are gently pulled toward the center to prevent drift
//!
//! This results in clusters of related tables with minimal edge crossings.

use crate::core::schema::SchemaGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Layout configuration
#[derive(Clone, Debug)]
pub struct LayoutConfig {
    /// Horizontal spacing between tables
    pub horizontal_spacing: f64,
    /// Vertical spacing between layers
    pub vertical_spacing: f64,
    /// Starting X position
    pub start_x: f64,
    /// Starting Y position
    pub start_y: f64,
    /// Estimated table width for spacing calculations
    pub table_width: f64,
    /// Estimated table height for spacing calculations
    pub table_height: f64,
    /// Number of iterations for force simulation
    pub iterations: usize,
    /// Initial temperature (movement speed) for simulated annealing
    pub initial_temperature: f64,
    /// Cooling rate per iteration
    pub cooling_rate: f64,
    /// Ideal distance between connected nodes
    pub ideal_edge_length: f64,
    /// Repulsion strength between all nodes
    pub repulsion_strength: f64,
    /// Attraction strength for connected nodes
    pub attraction_strength: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            horizontal_spacing: 80.0,
            vertical_spacing: 100.0,
            start_x: 100.0,
            start_y: 100.0,
            table_width: 280.0,
            table_height: 250.0,
            iterations: 300,
            initial_temperature: 100.0,
            cooling_rate: 0.95,
            ideal_edge_length: 400.0,
            repulsion_strength: 50000.0,
            attraction_strength: 0.1,
        }
    }
}

/// Result of auto-layout calculation
pub struct LayoutResult {
    /// New positions for each node: (node_index, (x, y))
    pub positions: Vec<(NodeIndex, (f64, f64))>,
}

/// 2D Vector for physics calculations
#[derive(Clone, Copy, Debug, Default)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    fn normalize(&self) -> Self {
        let len = self.length();
        if len < 0.0001 {
            Self::new(0.0, 0.0)
        } else {
            Self::new(self.x / len, self.y / len)
        }
    }

    fn add(&self, other: Vec2) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn sub(&self, other: Vec2) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    fn scale(&self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }
}

/// Performs force-directed layout on the schema graph
///
/// The algorithm:
/// 1. Initialize positions (use existing or arrange in circle)
/// 2. Simulate physical forces iteratively:
///    - Repulsion between all node pairs
///    - Attraction along edges (foreign key relationships)
///    - Centering force to prevent drift
/// 3. Apply simulated annealing (gradually reduce movement)
/// 4. Return final positions
pub fn calculate_hierarchical_layout(graph: &SchemaGraph, config: &LayoutConfig) -> LayoutResult {
    let node_count = graph.node_count();

    if node_count == 0 {
        return LayoutResult { positions: vec![] };
    }

    if node_count == 1 {
        let idx = graph.node_indices().next().unwrap();
        return LayoutResult {
            positions: vec![(idx, (config.start_x + 200.0, config.start_y + 200.0))],
        };
    }

    // Collect node indices
    let nodes: Vec<NodeIndex> = graph.node_indices().collect();
    let node_to_idx: HashMap<NodeIndex, usize> =
        nodes.iter().enumerate().map(|(i, &n)| (n, i)).collect();

    // Initialize positions in a circle or use existing positions
    let mut positions: Vec<Vec2> = initialize_positions(graph, &nodes, config);

    // Calculate center of canvas
    let center = Vec2::new(config.start_x + 600.0, config.start_y + 400.0);

    // Collect edges for attraction forces
    let edges: Vec<(usize, usize)> = graph
        .edge_indices()
        .filter_map(|e| {
            let (a, b) = graph.edge_endpoints(e)?;
            Some((*node_to_idx.get(&a).unwrap(), *node_to_idx.get(&b).unwrap()))
        })
        .collect();

    // Simulated annealing with force-directed layout
    let mut temperature = config.initial_temperature;

    for _iteration in 0..config.iterations {
        let mut forces: Vec<Vec2> = vec![Vec2::default(); node_count];

        // Calculate repulsion forces between all pairs
        for i in 0..node_count {
            for j in (i + 1)..node_count {
                let delta = positions[i].sub(positions[j]);
                let distance = delta.length().max(1.0);

                // Repulsion force (inverse square law, but with table size consideration)
                let min_distance = config.table_width + config.horizontal_spacing;
                let repulsion = if distance < min_distance {
                    // Strong repulsion when overlapping
                    config.repulsion_strength * 2.0 / (distance * distance).max(1.0)
                } else {
                    config.repulsion_strength / (distance * distance)
                };

                let force = delta.normalize().scale(repulsion);
                forces[i] = forces[i].add(force);
                forces[j] = forces[j].sub(force);
            }
        }

        // Calculate attraction forces along edges
        for &(a, b) in &edges {
            let delta = positions[b].sub(positions[a]);
            let distance = delta.length().max(1.0);

            // Attraction force (spring-like, proportional to distance from ideal)
            let displacement = distance - config.ideal_edge_length;
            let attraction = config.attraction_strength * displacement;

            let force = delta.normalize().scale(attraction);
            forces[a] = forces[a].add(force);
            forces[b] = forces[b].sub(force);
        }

        // Centering force (gentle pull toward center)
        for i in 0..node_count {
            let to_center = center.sub(positions[i]);
            let centering_force = to_center.scale(0.01);
            forces[i] = forces[i].add(centering_force);
        }

        // Apply forces with temperature limiting
        for i in 0..node_count {
            let force_magnitude = forces[i].length();
            if force_magnitude > 0.01 {
                // Limit movement by temperature
                let capped_magnitude = force_magnitude.min(temperature);
                let movement = forces[i].normalize().scale(capped_magnitude);
                positions[i] = positions[i].add(movement);
            }
        }

        // Cool down
        temperature *= config.cooling_rate;

        // Stop if temperature is very low
        if temperature < 0.1 {
            break;
        }
    }

    // Prevent overlapping with final adjustment pass
    positions = prevent_overlaps(&positions, config);

    // Normalize positions to start from config.start_x/start_y
    let min_x = positions.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let min_y = positions.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);

    let final_positions: Vec<(NodeIndex, (f64, f64))> = nodes
        .iter()
        .zip(positions.iter())
        .map(|(&node, pos)| {
            (
                node,
                (
                    pos.x - min_x + config.start_x,
                    pos.y - min_y + config.start_y,
                ),
            )
        })
        .collect();

    LayoutResult {
        positions: final_positions,
    }
}

/// Initialize positions - use existing or arrange in circle
fn initialize_positions(
    graph: &SchemaGraph,
    nodes: &[NodeIndex],
    config: &LayoutConfig,
) -> Vec<Vec2> {
    let node_count = nodes.len();
    let center_x = config.start_x + 600.0;
    let center_y = config.start_y + 400.0;
    let radius = (node_count as f64 * 100.0).max(300.0);

    // Check if existing positions are meaningful (not all at origin)
    let existing_positions: Vec<(f64, f64)> = nodes
        .iter()
        .filter_map(|&n| graph.node_weight(n).map(|node| node.position))
        .collect();

    let has_meaningful_positions = existing_positions.len() == node_count
        && existing_positions
            .iter()
            .any(|(x, y)| *x != 0.0 || *y != 0.0);

    if has_meaningful_positions {
        // Use existing positions but add some randomness to break symmetry
        existing_positions
            .iter()
            .enumerate()
            .map(|(i, (x, y))| {
                let jitter = (i as f64 * 0.1).sin() * 10.0;
                Vec2::new(*x + jitter, *y + jitter)
            })
            .collect()
    } else {
        // Arrange in a circle
        nodes
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (node_count as f64);
                Vec2::new(
                    center_x + radius * angle.cos(),
                    center_y + radius * angle.sin(),
                )
            })
            .collect()
    }
}

/// Final pass to prevent any remaining overlaps
fn prevent_overlaps(positions: &[Vec2], config: &LayoutConfig) -> Vec<Vec2> {
    let mut result = positions.to_vec();
    let min_dist_x = config.table_width + config.horizontal_spacing;
    let min_dist_y = config.table_height + config.vertical_spacing;

    // Multiple passes to resolve overlaps
    for _ in 0..50 {
        let mut any_overlap = false;

        for i in 0..result.len() {
            for j in (i + 1)..result.len() {
                let dx = (result[i].x - result[j].x).abs();
                let dy = (result[i].y - result[j].y).abs();

                // Check if rectangles overlap
                if dx < min_dist_x && dy < min_dist_y {
                    any_overlap = true;

                    // Push apart
                    let delta = result[i].sub(result[j]);
                    let push = if delta.length() < 1.0 {
                        // If exactly same position, push in arbitrary direction
                        Vec2::new(min_dist_x * 0.5, min_dist_y * 0.5)
                    } else {
                        // Push along the line between centers
                        let norm = delta.normalize();
                        Vec2::new(
                            norm.x * (min_dist_x - dx) * 0.5,
                            norm.y * (min_dist_y - dy) * 0.5,
                        )
                    };

                    result[i] = result[i].add(push);
                    result[j] = result[j].sub(push);
                }
            }
        }

        if !any_overlap {
            break;
        }
    }

    result
}

/// Applies the calculated layout to the graph
pub fn apply_layout(graph: &mut SchemaGraph, layout: &LayoutResult) {
    for (node_idx, (x, y)) in &layout.positions {
        if let Some(node) = graph.node_weight_mut(*node_idx) {
            node.position = (*x, *y);
        }
    }
}

/// Convenience function to auto-layout with default config
pub fn auto_layout(graph: &mut SchemaGraph) {
    let config = LayoutConfig::default();
    let layout = calculate_hierarchical_layout(graph, &config);
    apply_layout(graph, &layout);
}

/// Convenience function to auto-layout with custom config
pub fn auto_layout_with_config(graph: &mut SchemaGraph, config: &LayoutConfig) {
    let layout = calculate_hierarchical_layout(graph, config);
    apply_layout(graph, &layout);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::schema::{Column, Relationship, RelationshipOps, RelationshipType, TableNode};

    #[test]
    fn test_empty_graph() {
        let graph = SchemaGraph::new();
        let config = LayoutConfig::default();
        let result = calculate_hierarchical_layout(&graph, &config);
        assert!(result.positions.is_empty());
    }

    #[test]
    fn test_single_table() {
        let mut graph = SchemaGraph::new();
        let table = TableNode::new("users");
        graph.add_node(table);

        let config = LayoutConfig::default();
        let result = calculate_hierarchical_layout(&graph, &config);

        assert_eq!(result.positions.len(), 1);
    }

    #[test]
    fn test_two_related_tables() {
        let mut graph = SchemaGraph::new();

        // Users table (parent)
        let users = TableNode::new("users").add_column(Column::new("id", "INT").primary_key());
        let users_idx = graph.add_node(users);

        // Orders table (child, references users)
        let orders = TableNode::new("orders")
            .add_column(Column::new("id", "INT").primary_key())
            .add_column(Column::new("user_id", "INT"));
        let orders_idx = graph.add_node(orders);

        // Add relationship: orders -> users
        graph
            .create_relationship(
                orders_idx,
                users_idx,
                Relationship::new(
                    "fk_orders_users",
                    RelationshipType::ManyToOne,
                    "user_id",
                    "id",
                ),
            )
            .unwrap();

        let config = LayoutConfig::default();
        let result = calculate_hierarchical_layout(&graph, &config);

        assert_eq!(result.positions.len(), 2);

        // Find positions
        let users_pos = result
            .positions
            .iter()
            .find(|(idx, _)| *idx == users_idx)
            .map(|(_, p)| p);
        let orders_pos = result
            .positions
            .iter()
            .find(|(idx, _)| *idx == orders_idx)
            .map(|(_, p)| p);

        // Both should have positions
        assert!(users_pos.is_some());
        assert!(orders_pos.is_some());

        // Connected tables should be relatively close (within ideal_edge_length * 2)
        let (ux, uy) = users_pos.unwrap();
        let (ox, oy) = orders_pos.unwrap();
        let distance = ((ux - ox).powi(2) + (uy - oy).powi(2)).sqrt();
        assert!(
            distance < config.ideal_edge_length * 2.5,
            "Connected tables should be close together, but distance is {}",
            distance
        );
    }

    #[test]
    fn test_layout_config_default() {
        let config = LayoutConfig::default();
        assert_eq!(config.horizontal_spacing, 80.0);
        assert_eq!(config.vertical_spacing, 100.0);
        assert_eq!(config.start_x, 100.0);
        assert_eq!(config.start_y, 100.0);
        assert_eq!(config.iterations, 300);
    }

    #[test]
    fn test_apply_layout() {
        let mut graph = SchemaGraph::new();
        let table = TableNode::new("test").with_position(0.0, 0.0);
        let idx = graph.add_node(table);

        let layout = LayoutResult {
            positions: vec![(idx, (500.0, 300.0))],
        };

        apply_layout(&mut graph, &layout);

        let node = graph.node_weight(idx).unwrap();
        assert_eq!(node.position, (500.0, 300.0));
    }

    #[test]
    fn test_no_overlaps() {
        let mut graph = SchemaGraph::new();

        // Create several unconnected tables
        for i in 0..5 {
            let table = TableNode::new(format!("table_{}", i)).with_position(0.0, 0.0);
            graph.add_node(table);
        }

        let config = LayoutConfig::default();
        let result = calculate_hierarchical_layout(&graph, &config);

        // Check that no two tables overlap
        let min_dist_x = config.table_width + config.horizontal_spacing;
        let min_dist_y = config.table_height + config.vertical_spacing;

        for i in 0..result.positions.len() {
            for j in (i + 1)..result.positions.len() {
                let (_, (x1, y1)) = result.positions[i];
                let (_, (x2, y2)) = result.positions[j];

                let dx = (x1 - x2).abs();
                let dy = (y1 - y2).abs();

                // At least one dimension should have enough distance (with small epsilon for floating point)
                let epsilon = 1.0;
                assert!(
                    dx >= min_dist_x - epsilon || dy >= min_dist_y - epsilon,
                    "Tables {} and {} overlap: dx={}, dy={}, min_x={}, min_y={}",
                    i,
                    j,
                    dx,
                    dy,
                    min_dist_x,
                    min_dist_y
                );
            }
        }
    }

    #[test]
    fn test_vec2_operations() {
        let a = Vec2::new(3.0, 4.0);
        let b = Vec2::new(1.0, 2.0);

        assert!((a.length() - 5.0).abs() < 0.001);

        let sum = a.add(b);
        assert!((sum.x - 4.0).abs() < 0.001);
        assert!((sum.y - 6.0).abs() < 0.001);

        let diff = a.sub(b);
        assert!((diff.x - 2.0).abs() < 0.001);
        assert!((diff.y - 2.0).abs() < 0.001);

        let scaled = a.scale(2.0);
        assert!((scaled.x - 6.0).abs() < 0.001);
        assert!((scaled.y - 8.0).abs() < 0.001);

        let norm = a.normalize();
        assert!((norm.length() - 1.0).abs() < 0.001);
    }
}
