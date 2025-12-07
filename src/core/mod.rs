//! Core domain models and business logic for database schema management

#[cfg(feature = "ssr")]
pub mod config;
mod schema;
#[cfg(test)]
mod tests;

pub mod liveshare;

pub use schema::*;
