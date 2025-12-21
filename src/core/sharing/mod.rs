//! Sharing module for Archischema
//!
//! This module provides REST API endpoints for diagram sharing:
//! - Share diagrams with other users by email or username
//! - List shares for a diagram
//! - Update share permissions
//! - Remove shares

pub mod api;

pub use api::{ShareApiState, share_api_router};
