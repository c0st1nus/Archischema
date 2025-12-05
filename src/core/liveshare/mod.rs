//! LiveShare module for real-time collaborative editing
//!
//! Provides WebSocket-based room management with:
//! - Up to 50 users per room
//! - Optional password protection
//! - CRDT-based state synchronization via Yrs
//! - Full CRUD API for room management

#[cfg(feature = "ssr")]
mod api;
#[cfg(feature = "ssr")]
mod auth;
mod protocol;
#[cfg(feature = "ssr")]
mod room;
#[cfg(feature = "ssr")]
mod websocket;

#[cfg(feature = "ssr")]
pub use api::*;
#[cfg(feature = "ssr")]
pub use auth::*;
pub use protocol::*;
#[cfg(feature = "ssr")]
pub use room::*;
#[cfg(feature = "ssr")]
pub use websocket::*;
