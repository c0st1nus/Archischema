//! LiveShare module for real-time collaborative editing
//!
//! Provides WebSocket-based room management with:
//! - Up to 50 users per room
//! - Optional password protection
//! - CRDT-based state synchronization via Yrs
//! - Full CRUD API for room management
//! - Periodic snapshots for crash recovery

#[cfg(feature = "ssr")]
mod api;
#[cfg(feature = "ssr")]
mod auth;
#[cfg(feature = "ssr")]
pub mod broadcast_manager;
#[cfg(feature = "ssr")]
pub mod cursor_broadcaster;
pub mod idle_detection;
mod protocol;
#[cfg(feature = "ssr")]
pub mod rate_limiter;
#[cfg(feature = "ssr")]
pub mod reconciliation;
#[cfg(feature = "ssr")]
pub mod room;
#[cfg(feature = "ssr")]
pub mod snapshots;
#[cfg(feature = "ssr")]
pub mod throttling;
#[cfg(feature = "ssr")]
pub mod websocket;

#[cfg(feature = "ssr")]
pub use api::*;
#[cfg(feature = "ssr")]
pub use auth::*;
#[cfg(feature = "ssr")]
pub use broadcast_manager::*;
#[cfg(feature = "ssr")]
pub use cursor_broadcaster::*;
pub use idle_detection::*;
pub use protocol::*;
#[cfg(feature = "ssr")]
pub use rate_limiter::*;
#[cfg(feature = "ssr")]
pub use reconciliation::*;
pub use room::*;
#[cfg(feature = "ssr")]
pub use snapshots::*;
#[cfg(feature = "ssr")]
pub use throttling::*;
#[cfg(feature = "ssr")]
pub use websocket::*;
