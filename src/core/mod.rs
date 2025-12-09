//! Core domain models and business logic for database schema management

#[cfg(feature = "ssr")]
pub mod config;
mod schema;
#[cfg(test)]
mod tests;
pub mod validation;

pub mod liveshare;

pub use schema::*;
pub use validation::{
    ValidationError, ValidationLevel, ValidationResult, validate_column_name, validate_identifier,
    validate_name, validate_table_name,
};
