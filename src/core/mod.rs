//! Core domain models and business logic for database schema management

#[cfg(feature = "ssr")]
pub mod ai_api;
pub mod ai_config;
pub mod ai_tools;
pub mod auto_layout;
#[cfg(feature = "ssr")]
pub mod config;
pub mod export;
mod schema;
#[cfg(test)]
mod tests;
pub mod validation;

pub mod liveshare;

#[cfg(feature = "ssr")]
pub use ai_api::{AiApiConfig, ai_api_router};
pub use ai_config::{
    AiConfig, AiMode, ChatMessage, ChatRequest, ChatResponse, build_tool_definitions,
};
pub use ai_tools::{ToolDefinition, ToolExecutor, ToolRequest, ToolResponse, get_tool_definitions};
pub use auto_layout::{
    LayoutConfig, LayoutResult, apply_layout, auto_layout, auto_layout_with_config,
    calculate_hierarchical_layout,
};
pub use export::{
    ExportFormat, ExportOptions, ExportedRelationship, ExportedSchema, ExportedTable,
    SchemaExporter, SchemaImporter, SqlDialect,
};
pub use schema::*;
pub use validation::{
    ValidationError, ValidationLevel, ValidationResult, validate_column_name, validate_identifier,
    validate_name, validate_table_name,
};
