//! Database repositories for Archischema
//!
//! This module provides repository implementations for database operations.
//! Repositories encapsulate data access logic and provide a clean API for
//! business logic to interact with the database.

pub mod diagram;
pub mod folder;
pub mod liveshare;
pub mod session;
pub mod share;
pub mod user;

pub use diagram::{DiagramAccess, DiagramRepository, DiagramRepositoryError, SharedDiagramInfo};
pub use folder::{FolderNode, FolderRepository, FolderRepositoryError, FolderWithDepth};
pub use liveshare::{LiveShareRepository, LiveShareRepositoryError};
pub use session::{SessionRepository, SessionRepositoryError};
pub use share::{ShareRepository, ShareRepositoryError};
pub use user::{UserRepository, UserRepositoryError};
