//! Database module for Archischema
//!
//! This module provides database connectivity, models, and repositories
//! for persistent storage using PostgreSQL and SQLx.

pub mod models;
pub mod pool;
pub mod repositories;

// Re-export commonly used items
pub use models::*;
pub use pool::{DbConfig, DbError, create_pool, create_pool_with_migrations};
pub use repositories::{
    DiagramAccess, DiagramRepository, DiagramRepositoryError, FolderNode, FolderRepository,
    FolderRepositoryError, FolderWithDepth, LiveShareRepository, LiveShareRepositoryError,
    SessionRepository, SessionRepositoryError, ShareRepository, ShareRepositoryError,
    SharedDiagramInfo, UserRepository, UserRepositoryError,
};

// Re-export sqlx types that might be needed
pub use sqlx::PgPool;
