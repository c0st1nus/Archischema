//! Diagrams module for Archischema
//!
//! This module provides REST API endpoints for diagram management:
//! - Create, read, update, delete diagrams
//! - Autosave support
//! - Permission-based access control

pub mod api;

pub use api::{DiagramApiState, diagram_api_router};
