//! Folders module for Archischema
//!
//! This module provides REST API endpoints for folder management:
//! - Create, read, update, delete folders
//! - Tree structure support for nested folders
//! - Folder path (breadcrumb) queries

pub mod api;

pub use api::{FolderApiState, folder_api_router};
