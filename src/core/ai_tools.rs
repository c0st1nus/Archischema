//! AI Tools module for database schema manipulation
//!
//! This module provides a structured API for AI agents to manipulate database schemas.
//! It offers two approaches:
//! 1. Structured JSON-based operations (recommended for programmatic access)
//! 2. SQL-based operations (for agents that prefer working with SQL)
//!
//! ## Design Philosophy
//!
//! AI agents work best when they have:
//! - Clear, well-documented operations
//! - Structured input/output formats
//! - Comprehensive error messages
//! - Ability to read current state before making changes
//!
//! This module provides all of these through a set of "tools" that an AI agent can invoke.

use super::{Column, Relationship, RelationshipOps, RelationshipType, SchemaGraph, TableOps};
use crate::core::SqlDialect;
use crate::core::liveshare::{ColumnData, GraphOperation, RelationshipData};
use crate::core::sql_parser::{SqlValidationResult, validate_sql};
use serde::{Deserialize, Serialize};

// ============================================================================
// Tool Definitions - Structured descriptions for AI agent discovery
// ============================================================================

/// Tool metadata for AI agent discovery
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterDefinition>,
    pub returns: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ParameterDefinition {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// Get all available tools for AI agent
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Read operations
        ToolDefinition {
            name: "get_schema_sql".into(),
            description: "Get the current database schema as SQL DDL statements. Use this to understand the current structure before making changes.".into(),
            parameters: vec![],
            returns: "SQL string with CREATE TABLE statements and foreign key constraints".into(),
        },
        ToolDefinition {
            name: "get_schema_json".into(),
            description: "Get the current database schema as structured JSON. Includes tables, columns, relationships, and positions.".into(),
            parameters: vec![],
            returns: "JSON object with tables array and relationships array".into(),
        },
        ToolDefinition {
            name: "list_tables".into(),
            description: "List all table names in the schema".into(),
            parameters: vec![],
            returns: "Array of table names".into(),
        },
        ToolDefinition {
            name: "get_table".into(),
            description: "Get detailed information about a specific table including all columns".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "table_name".into(),
                    param_type: "string".into(),
                    description: "Name of the table to retrieve".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Table object with name, columns, and position".into(),
        },
        ToolDefinition {
            name: "get_relationships".into(),
            description: "Get all relationships involving a specific table".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "table_name".into(),
                    param_type: "string".into(),
                    description: "Name of the table".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Array of relationships (both outgoing and incoming)".into(),
        },
        // Table operations
        ToolDefinition {
            name: "create_table".into(),
            description: "Create a new table in the schema".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "name".into(),
                    param_type: "string".into(),
                    description: "Name of the new table (must be unique, valid SQL identifier)".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "columns".into(),
                    param_type: "array<Column>".into(),
                    description: "Array of column definitions".into(),
                    required: false,
                    default_value: Some("[]".into()),
                },
                ParameterDefinition {
                    name: "position_x".into(),
                    param_type: "number".into(),
                    description: "X position on canvas".into(),
                    required: false,
                    default_value: Some("100".into()),
                },
                ParameterDefinition {
                    name: "position_y".into(),
                    param_type: "number".into(),
                    description: "Y position on canvas".into(),
                    required: false,
                    default_value: Some("100".into()),
                },
            ],
            returns: "Success message or error".into(),
        },
        ToolDefinition {
            name: "rename_table".into(),
            description: "Rename an existing table".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "old_name".into(),
                    param_type: "string".into(),
                    description: "Current name of the table".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "new_name".into(),
                    param_type: "string".into(),
                    description: "New name for the table".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Success message or error".into(),
        },
        ToolDefinition {
            name: "delete_table".into(),
            description: "Delete a table and all its relationships".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "table_name".into(),
                    param_type: "string".into(),
                    description: "Name of the table to delete".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Success message or error".into(),
        },
        // Column operations
        ToolDefinition {
            name: "add_column".into(),
            description: "Add a new column to an existing table".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "table_name".into(),
                    param_type: "string".into(),
                    description: "Name of the table".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "column_name".into(),
                    param_type: "string".into(),
                    description: "Name of the new column".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "data_type".into(),
                    param_type: "string".into(),
                    description: "SQL data type (e.g., VARCHAR(255), INT, TEXT, TIMESTAMP)".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "is_primary_key".into(),
                    param_type: "boolean".into(),
                    description: "Whether this column is a primary key".into(),
                    required: false,
                    default_value: Some("false".into()),
                },
                ParameterDefinition {
                    name: "is_nullable".into(),
                    param_type: "boolean".into(),
                    description: "Whether this column allows NULL values".into(),
                    required: false,
                    default_value: Some("true".into()),
                },
                ParameterDefinition {
                    name: "is_unique".into(),
                    param_type: "boolean".into(),
                    description: "Whether this column has a UNIQUE constraint".into(),
                    required: false,
                    default_value: Some("false".into()),
                },
                ParameterDefinition {
                    name: "default_value".into(),
                    param_type: "string".into(),
                    description: "Default value for the column (SQL expression)".into(),
                    required: false,
                    default_value: None,
                },
            ],
            returns: "Success message or error".into(),
        },
        ToolDefinition {
            name: "modify_column".into(),
            description: "Modify an existing column's properties".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "table_name".into(),
                    param_type: "string".into(),
                    description: "Name of the table".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "column_name".into(),
                    param_type: "string".into(),
                    description: "Name of the column to modify".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "new_name".into(),
                    param_type: "string".into(),
                    description: "New name for the column (optional, for renaming)".into(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "data_type".into(),
                    param_type: "string".into(),
                    description: "New data type (optional)".into(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "is_primary_key".into(),
                    param_type: "boolean".into(),
                    description: "Set primary key status (optional)".into(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "is_nullable".into(),
                    param_type: "boolean".into(),
                    description: "Set nullable status (optional)".into(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "is_unique".into(),
                    param_type: "boolean".into(),
                    description: "Set unique constraint (optional)".into(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "default_value".into(),
                    param_type: "string".into(),
                    description: "New default value (optional, use 'NULL' to remove)".into(),
                    required: false,
                    default_value: None,
                },
            ],
            returns: "Success message or error".into(),
        },
        ToolDefinition {
            name: "delete_column".into(),
            description: "Delete a column from a table".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "table_name".into(),
                    param_type: "string".into(),
                    description: "Name of the table".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "column_name".into(),
                    param_type: "string".into(),
                    description: "Name of the column to delete".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Success message or error".into(),
        },
        // Relationship operations
        ToolDefinition {
            name: "create_relationship".into(),
            description: "Create a foreign key relationship between two tables".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "name".into(),
                    param_type: "string".into(),
                    description: "Name for the relationship (auto-generated if not provided)".into(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "from_table".into(),
                    param_type: "string".into(),
                    description: "Source table name (the one with the foreign key)".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "from_column".into(),
                    param_type: "string".into(),
                    description: "Column in the source table".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "to_table".into(),
                    param_type: "string".into(),
                    description: "Target table name (the one being referenced)".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "to_column".into(),
                    param_type: "string".into(),
                    description: "Column in the target table (usually primary key)".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "relationship_type".into(),
                    param_type: "string".into(),
                    description: "Type: 'OneToOne', 'OneToMany', 'ManyToOne', 'ManyToMany'".into(),
                    required: false,
                    default_value: Some("OneToMany".into()),
                },
            ],
            returns: "Success message or error".into(),
        },
        ToolDefinition {
            name: "delete_relationship".into(),
            description: "Delete a relationship between tables".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "from_table".into(),
                    param_type: "string".into(),
                    description: "Source table name".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "from_column".into(),
                    param_type: "string".into(),
                    description: "Column in the source table".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "to_table".into(),
                    param_type: "string".into(),
                    description: "Target table name".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "to_column".into(),
                    param_type: "string".into(),
                    description: "Column in the target table".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Success message or error".into(),
        },
        // Bulk operations
        ToolDefinition {
            name: "apply_sql".into(),
            description: "Apply SQL DDL statements to modify the schema. Supports CREATE TABLE, ALTER TABLE, and DROP TABLE. This is useful for agents that prefer working with SQL.".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "sql".into(),
                    param_type: "string".into(),
                    description: "SQL DDL statements to apply".into(),
                    required: true,
                    default_value: None,
                },
            ],
            returns: "Success message with applied changes or error".into(),
        },
        // Validation operations
        ToolDefinition {
            name: "validate_sql".into(),
            description: "Validate SQL DDL statements for syntax and semantic errors. Use this to check SQL before applying it. Returns detailed error information with line/column positions.".into(),
            parameters: vec![
                ParameterDefinition {
                    name: "sql".into(),
                    param_type: "string".into(),
                    description: "SQL DDL statements to validate".into(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "dialect".into(),
                    param_type: "string".into(),
                    description: "SQL dialect: 'mysql', 'postgresql', or 'sqlite'".into(),
                    required: false,
                    default_value: Some("mysql".into()),
                },
            ],
            returns: "Validation result with is_valid flag, error_count, warning_count, and detailed diagnostics array".into(),
        },
        ToolDefinition {
            name: "check_schema".into(),
            description: "Validate the current schema for consistency and correctness. Use this before saving or exporting.".into(),
            parameters: vec![],
            returns: "Validation result with is_valid flag and any issues found".into(),
        },
    ]
}

// ============================================================================
// Operation Request/Response types for structured communication
// ============================================================================

/// Request to execute an AI tool operation
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolRequest {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

/// Response from an AI tool operation
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Validation result for SQL operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<serde_json::Value>,
    /// Graph operations to sync with LiveShare clients
    #[serde(skip)]
    pub graph_ops: Vec<GraphOperation>,
}

impl ToolResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            error: None,
            validation: None,
            graph_ops: Vec::new(),
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            error: None,
            validation: None,
            graph_ops: Vec::new(),
        }
    }

    pub fn success_with_ops(message: impl Into<String>, ops: Vec<GraphOperation>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            error: None,
            validation: None,
            graph_ops: ops,
        }
    }

    pub fn success_with_validation(
        message: impl Into<String>,
        validation: SqlValidationResult,
    ) -> Self {
        Self {
            success: validation.is_valid,
            message: message.into(),
            data: None,
            error: if validation.is_valid {
                None
            } else {
                Some(format!("{} errors found", validation.stats.error_count))
            },
            validation: Some(validation.format_for_llm()),
            graph_ops: Vec::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            success: false,
            message: msg.clone(),
            data: None,
            error: Some(msg),
            validation: None,
            graph_ops: Vec::new(),
        }
    }

    /// Add validation result to an existing response
    pub fn with_validation(mut self, validation: SqlValidationResult) -> Self {
        self.validation = Some(validation.format_for_llm());
        self
    }
}

// ============================================================================
// Input types for specific operations
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CreateTableInput {
    pub name: String,
    #[serde(default)]
    pub columns: Vec<ColumnInput>,
    /// Position X - if None, will be auto-calculated
    pub position_x: Option<f64>,
    /// Position Y - if None, will be auto-calculated
    pub position_y: Option<f64>,
}

/// Constants for table layout
const TABLE_WIDTH: f64 = 250.0;
const TABLE_HEIGHT: f64 = 200.0;
const TABLE_SPACING: f64 = 50.0;
const TABLES_PER_ROW: usize = 4;
const START_X: f64 = 100.0;
const START_Y: f64 = 100.0;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ColumnInput {
    pub name: String,
    pub data_type: String,
    #[serde(default)]
    pub is_primary_key: bool,
    #[serde(default = "default_nullable")]
    pub is_nullable: bool,
    #[serde(default)]
    pub is_unique: bool,
    pub default_value: Option<String>,
}

fn default_nullable() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AddColumnInput {
    pub table_name: String,
    pub column_name: String,
    pub data_type: String,
    #[serde(default)]
    pub is_primary_key: bool,
    #[serde(default = "default_nullable")]
    pub is_nullable: bool,
    #[serde(default)]
    pub is_unique: bool,
    pub default_value: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ModifyColumnInput {
    pub table_name: String,
    pub column_name: String,
    pub new_name: Option<String>,
    pub data_type: Option<String>,
    pub is_primary_key: Option<bool>,
    pub is_nullable: Option<bool>,
    pub is_unique: Option<bool>,
    pub default_value: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CreateRelationshipInput {
    #[serde(default)]
    pub name: Option<String>,
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    #[serde(default = "default_relationship_type")]
    pub relationship_type: String,
}

fn default_relationship_type() -> String {
    "one_to_many".into()
}

// ============================================================================
// Tool Executor - Main entry point for executing operations
// ============================================================================

/// Execute an AI tool operation on the schema graph
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute a tool operation by name
    pub fn execute(graph: &mut SchemaGraph, request: &ToolRequest) -> ToolResponse {
        match request.tool_name.as_str() {
            // Read operations
            "get_schema_sql" => Self::get_schema_sql(graph),
            "get_schema_json" => Self::get_schema_json(graph),
            "list_tables" => Self::list_tables(graph),
            "get_table" => Self::get_table(graph, &request.parameters),
            "get_relationships" => Self::get_relationships(graph, &request.parameters),

            // Table operations
            "create_table" => Self::create_table(graph, &request.parameters),
            "rename_table" => Self::rename_table(graph, &request.parameters),
            "delete_table" => Self::delete_table(graph, &request.parameters),

            // Column operations
            "add_column" => Self::add_column(graph, &request.parameters),
            "modify_column" => Self::modify_column(graph, &request.parameters),
            "delete_column" => Self::delete_column(graph, &request.parameters),

            // Relationship operations
            "create_relationship" => Self::create_relationship(graph, &request.parameters),
            "delete_relationship" => Self::delete_relationship(graph, &request.parameters),

            // SQL operations
            "apply_sql" => Self::apply_sql(graph, &request.parameters),

            // Validation operations
            "validate_sql" => Self::validate_sql_tool(&request.parameters),
            "check_schema" => Self::check_schema(graph),

            _ => ToolResponse::error(format!("Unknown tool: {}", request.tool_name)),
        }
    }

    // ========================================================================
    // Read operations
    // ========================================================================

    fn get_schema_sql(graph: &SchemaGraph) -> ToolResponse {
        use super::export::{ExportOptions, SchemaExporter};

        let options = ExportOptions::default();
        match SchemaExporter::export_sql(graph, &options) {
            Ok(sql) => ToolResponse::success_with_data(
                "Schema exported as SQL",
                serde_json::Value::String(sql),
            ),
            Err(e) => ToolResponse::error(e),
        }
    }

    fn get_schema_json(graph: &SchemaGraph) -> ToolResponse {
        use super::export::SchemaExporter;

        let schema = SchemaExporter::to_exported_schema(graph);
        match serde_json::to_value(&schema) {
            Ok(json) => ToolResponse::success_with_data("Schema exported as JSON", json),
            Err(e) => ToolResponse::error(e.to_string()),
        }
    }

    fn list_tables(graph: &SchemaGraph) -> ToolResponse {
        let tables: Vec<String> = graph.node_weights().map(|t| t.name.clone()).collect();
        ToolResponse::success_with_data(
            format!("Found {} tables", tables.len()),
            serde_json::json!(tables),
        )
    }

    fn get_table(graph: &SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let table_name = match params.get("table_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: table_name"),
        };

        match graph.find_table_by_name(table_name) {
            Some(idx) => {
                if let Some(table) = graph.node_weight(idx) {
                    match serde_json::to_value(table) {
                        Ok(json) => ToolResponse::success_with_data(
                            format!("Table '{}' found", table_name),
                            json,
                        ),
                        Err(e) => ToolResponse::error(e.to_string()),
                    }
                } else {
                    ToolResponse::error(format!("Table '{}' not found", table_name))
                }
            }
            None => ToolResponse::error(format!("Table '{}' not found", table_name)),
        }
    }

    fn get_relationships(graph: &SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let table_name = match params.get("table_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: table_name"),
        };

        let idx = match graph.find_table_by_name(table_name) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Table '{}' not found", table_name)),
        };

        let mut relationships = Vec::new();

        // Outgoing relationships
        for (_, target_idx, rel) in graph.find_relationships_from(idx) {
            let target_name = graph
                .node_weight(target_idx)
                .map(|t| t.name.clone())
                .unwrap_or_default();
            relationships.push(serde_json::json!({
                "direction": "outgoing",
                "name": rel.name,
                "type": rel.relationship_type.to_string(),
                "from_table": table_name,
                "from_column": rel.from_column,
                "to_table": target_name,
                "to_column": rel.to_column,
            }));
        }

        // Incoming relationships
        for (_, source_idx, rel) in graph.find_relationships_to(idx) {
            let source_name = graph
                .node_weight(source_idx)
                .map(|t| t.name.clone())
                .unwrap_or_default();
            relationships.push(serde_json::json!({
                "direction": "incoming",
                "name": rel.name,
                "type": rel.relationship_type.to_string(),
                "from_table": source_name,
                "from_column": rel.from_column,
                "to_table": table_name,
                "to_column": rel.to_column,
            }));
        }

        ToolResponse::success_with_data(
            format!(
                "Found {} relationships for table '{}'",
                relationships.len(),
                table_name
            ),
            serde_json::json!(relationships),
        )
    }

    // ========================================================================
    // Table operations
    // ========================================================================

    fn create_table(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let input: CreateTableInput = match serde_json::from_value(params.clone()) {
            Ok(input) => input,
            Err(e) => return ToolResponse::error(format!("Invalid parameters: {}", e)),
        };

        // Calculate position - use provided values or auto-calculate
        let position = match (input.position_x, input.position_y) {
            (Some(x), Some(y)) => (x, y),
            _ => Self::calculate_next_position(graph),
        };

        // Create the table
        match graph.create_table(&input.name, position) {
            Ok(idx) => {
                let mut ops = Vec::new();

                // Add CreateTable operation
                let table_uuid = graph
                    .node_weight(idx)
                    .map(|n| n.uuid)
                    .unwrap_or_else(uuid::Uuid::new_v4);
                ops.push(GraphOperation::CreateTable {
                    node_id: idx.index() as u32,
                    table_uuid,
                    name: input.name.clone(),
                    position,
                });

                // Add columns
                if let Some(table) = graph.node_weight_mut(idx) {
                    for col_input in &input.columns {
                        let mut col = Column::new(&col_input.name, &col_input.data_type);
                        col.is_primary_key = col_input.is_primary_key;
                        col.is_nullable = col_input.is_nullable;
                        col.is_unique = col_input.is_unique;
                        col.default_value = col_input.default_value.clone();
                        table.columns.push(col);

                        // Add AddColumn operation for each column
                        ops.push(GraphOperation::AddColumn {
                            node_id: idx.index() as u32,
                            table_uuid,
                            column: ColumnData {
                                name: col_input.name.clone(),
                                data_type: col_input.data_type.clone(),
                                is_primary_key: col_input.is_primary_key,
                                is_nullable: col_input.is_nullable,
                                is_unique: col_input.is_unique,
                                default_value: col_input.default_value.clone(),
                                foreign_key: None,
                            },
                        });
                    }
                }
                ToolResponse::success_with_ops(
                    format!("Table '{}' created successfully", input.name),
                    ops,
                )
            }
            Err(e) => ToolResponse::error(e),
        }
    }

    /// Calculate the next available position for a new table
    /// Uses a grid layout, finding the first non-overlapping position
    fn calculate_next_position(graph: &SchemaGraph) -> (f64, f64) {
        // Collect all existing table positions
        let existing_positions: Vec<(f64, f64)> = graph
            .node_indices()
            .filter_map(|idx| graph.node_weight(idx))
            .map(|table| table.position)
            .collect();

        if existing_positions.is_empty() {
            return (START_X, START_Y);
        }

        // Find the next grid position that doesn't overlap
        let table_count = existing_positions.len();

        // Calculate position in a grid layout
        let row = table_count / TABLES_PER_ROW;
        let col = table_count % TABLES_PER_ROW;

        let x = START_X + col as f64 * (TABLE_WIDTH + TABLE_SPACING);
        let y = START_Y + row as f64 * (TABLE_HEIGHT + TABLE_SPACING);

        // Check if this position overlaps with any existing table
        let overlaps = existing_positions
            .iter()
            .any(|(ex, ey)| (x - ex).abs() < TABLE_WIDTH && (y - ey).abs() < TABLE_HEIGHT);

        if overlaps {
            // Find max X and Y to place after all existing tables
            let max_x = existing_positions
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::NEG_INFINITY, f64::max);
            let max_y = existing_positions
                .iter()
                .filter(|(x, _)| (*x - max_x).abs() < TABLE_WIDTH)
                .map(|(_, y)| *y)
                .fold(f64::NEG_INFINITY, f64::max);

            // Place to the right of the rightmost table, or start new row
            if (table_count + 1).is_multiple_of(TABLES_PER_ROW) {
                (START_X, max_y + TABLE_HEIGHT + TABLE_SPACING)
            } else {
                (max_x + TABLE_WIDTH + TABLE_SPACING, max_y)
            }
        } else {
            (x, y)
        }
    }

    fn rename_table(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let old_name = match params.get("old_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: old_name"),
        };

        let new_name = match params.get("new_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: new_name"),
        };

        let idx = match graph.find_table_by_name(old_name) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Table '{}' not found", old_name)),
        };

        match graph.rename_table(idx, new_name) {
            Ok(_) => {
                let table_uuid = graph
                    .node_weight(idx)
                    .map(|n| n.uuid)
                    .unwrap_or_else(uuid::Uuid::new_v4);
                ToolResponse::success_with_ops(
                    format!("Table renamed from '{}' to '{}'", old_name, new_name),
                    vec![GraphOperation::RenameTable {
                        node_id: idx.index() as u32,
                        table_uuid,
                        new_name: new_name.to_string(),
                    }],
                )
            }
            Err(e) => ToolResponse::error(e),
        }
    }

    fn delete_table(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let table_name = match params.get("table_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: table_name"),
        };

        let idx = match graph.find_table_by_name(table_name) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Table '{}' not found", table_name)),
        };

        let node_id = idx.index() as u32;
        match graph.delete_table(idx) {
            Ok(_) => {
                let table_uuid = graph
                    .node_weight(idx)
                    .map(|n| n.uuid)
                    .unwrap_or_else(uuid::Uuid::new_v4);
                ToolResponse::success_with_ops(
                    format!("Table '{}' deleted successfully", table_name),
                    vec![GraphOperation::DeleteTable {
                        node_id,
                        table_uuid,
                    }],
                )
            }
            Err(e) => ToolResponse::error(e),
        }
    }

    // ========================================================================
    // Column operations
    // ========================================================================

    fn add_column(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let input: AddColumnInput = match serde_json::from_value(params.clone()) {
            Ok(input) => input,
            Err(e) => return ToolResponse::error(format!("Invalid parameters: {}", e)),
        };

        let idx = match graph.find_table_by_name(&input.table_name) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Table '{}' not found", input.table_name)),
        };

        if let Some(table) = graph.node_weight_mut(idx) {
            // Check if column already exists
            if table.find_column(&input.column_name).is_some() {
                return ToolResponse::error(format!(
                    "Column '{}' already exists in table '{}'",
                    input.column_name, input.table_name
                ));
            }

            let mut col = Column::new(&input.column_name, &input.data_type);
            col.is_primary_key = input.is_primary_key;
            col.is_nullable = input.is_nullable;
            col.is_unique = input.is_unique;
            col.default_value = input.default_value.clone();
            table.columns.push(col);

            let table_uuid = graph
                .node_weight(idx)
                .map(|n| n.uuid)
                .unwrap_or_else(uuid::Uuid::new_v4);
            ToolResponse::success_with_ops(
                format!(
                    "Column '{}' added to table '{}'",
                    input.column_name, input.table_name
                ),
                vec![GraphOperation::AddColumn {
                    node_id: idx.index() as u32,
                    table_uuid,
                    column: ColumnData {
                        name: input.column_name,
                        data_type: input.data_type,
                        is_primary_key: input.is_primary_key,
                        is_nullable: input.is_nullable,
                        is_unique: input.is_unique,
                        default_value: input.default_value,
                        foreign_key: None,
                    },
                }],
            )
        } else {
            ToolResponse::error(format!("Table '{}' not found", input.table_name))
        }
    }

    fn modify_column(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let input: ModifyColumnInput = match serde_json::from_value(params.clone()) {
            Ok(input) => input,
            Err(e) => return ToolResponse::error(format!("Invalid parameters: {}", e)),
        };

        let idx = match graph.find_table_by_name(&input.table_name) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Table '{}' not found", input.table_name)),
        };

        if let Some(table) = graph.node_weight_mut(idx) {
            let col_idx = match table.find_column(&input.column_name) {
                Some((idx, _)) => idx,
                None => {
                    return ToolResponse::error(format!(
                        "Column '{}' not found in table '{}'",
                        input.column_name, input.table_name
                    ));
                }
            };

            if let Some(col) = table.get_column_mut(col_idx) {
                if let Some(ref new_name) = input.new_name {
                    col.name = new_name.clone();
                }
                if let Some(ref data_type) = input.data_type {
                    col.data_type = data_type.clone();
                }
                if let Some(is_pk) = input.is_primary_key {
                    col.is_primary_key = is_pk;
                }
                if let Some(is_nullable) = input.is_nullable {
                    col.is_nullable = is_nullable;
                }
                if let Some(is_unique) = input.is_unique {
                    col.is_unique = is_unique;
                }
                if let Some(ref default) = input.default_value {
                    col.default_value = if default == "NULL" {
                        None
                    } else {
                        Some(default.clone())
                    };
                }

                // Build the updated column data for sync
                let column_data = ColumnData {
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                    is_primary_key: col.is_primary_key,
                    is_nullable: col.is_nullable,
                    is_unique: col.is_unique,
                    default_value: col.default_value.clone(),
                    foreign_key: None,
                };

                let table_uuid = graph
                    .node_weight(idx)
                    .map(|n| n.uuid)
                    .unwrap_or_else(uuid::Uuid::new_v4);
                ToolResponse::success_with_ops(
                    format!(
                        "Column '{}' in table '{}' modified successfully",
                        input.column_name, input.table_name
                    ),
                    vec![GraphOperation::UpdateColumn {
                        node_id: idx.index() as u32,
                        table_uuid,
                        column_index: col_idx,
                        column: column_data,
                    }],
                )
            } else {
                ToolResponse::error(format!(
                    "Column '{}' not found in table '{}'",
                    input.column_name, input.table_name
                ))
            }
        } else {
            ToolResponse::error(format!("Table '{}' not found", input.table_name))
        }
    }

    fn delete_column(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let table_name = match params.get("table_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: table_name"),
        };

        let column_name = match params.get("column_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: column_name"),
        };

        let idx = match graph.find_table_by_name(table_name) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Table '{}' not found", table_name)),
        };

        if let Some(table) = graph.node_weight_mut(idx) {
            let col_idx = match table.find_column(column_name) {
                Some((idx, _)) => idx,
                None => {
                    return ToolResponse::error(format!(
                        "Column '{}' not found in table '{}'",
                        column_name, table_name
                    ));
                }
            };

            match table.delete_column(col_idx) {
                Ok(_) => {
                    let table_uuid = graph
                        .node_weight(idx)
                        .map(|n| n.uuid)
                        .unwrap_or_else(uuid::Uuid::new_v4);
                    ToolResponse::success_with_ops(
                        format!(
                            "Column '{}' deleted from table '{}'",
                            column_name, table_name
                        ),
                        vec![GraphOperation::DeleteColumn {
                            node_id: idx.index() as u32,
                            table_uuid,
                            column_index: col_idx,
                        }],
                    )
                }
                Err(e) => ToolResponse::error(e),
            }
        } else {
            ToolResponse::error(format!("Table '{}' not found", table_name))
        }
    }

    // ========================================================================
    // Relationship operations
    // ========================================================================

    fn create_relationship(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let input: CreateRelationshipInput = match serde_json::from_value(params.clone()) {
            Ok(input) => input,
            Err(e) => return ToolResponse::error(format!("Invalid parameters: {}", e)),
        };

        let from_idx = match graph.find_table_by_name(&input.from_table) {
            Some(idx) => idx,
            None => {
                return ToolResponse::error(format!(
                    "Source table '{}' not found",
                    input.from_table
                ));
            }
        };

        let to_idx = match graph.find_table_by_name(&input.to_table) {
            Some(idx) => idx,
            None => {
                return ToolResponse::error(format!("Target table '{}' not found", input.to_table));
            }
        };

        let rel_type = match input.relationship_type.to_lowercase().as_str() {
            "one_to_one" | "onetoone" | "1:1" => RelationshipType::OneToOne,
            "one_to_many" | "onetomany" | "1:n" => RelationshipType::OneToMany,
            "many_to_one" | "manytoone" | "n:1" => RelationshipType::ManyToOne,
            "many_to_many" | "manytomany" | "n:m" => RelationshipType::ManyToMany,
            _ => {
                return ToolResponse::error(format!(
                    "Invalid relationship type: {}. Use: OneToOne, OneToMany, ManyToOne, ManyToMany",
                    input.relationship_type
                ));
            }
        };

        // Auto-generate relationship name if not provided
        let rel_name = input.name.unwrap_or_else(|| {
            format!(
                "fk_{}_{}_{}_{}",
                input.from_table, input.from_column, input.to_table, input.to_column
            )
        });

        let relationship =
            Relationship::new(&rel_name, rel_type, &input.from_column, &input.to_column);

        match graph.create_relationship(from_idx, to_idx, relationship) {
            Ok(edge_idx) => ToolResponse::success_with_ops(
                format!(
                    "Relationship '{}' created: {}.{} -> {}.{}",
                    rel_name, input.from_table, input.from_column, input.to_table, input.to_column
                ),
                vec![GraphOperation::CreateRelationship {
                    edge_id: edge_idx.index() as u32,
                    from_node: from_idx.index() as u32,
                    to_node: to_idx.index() as u32,
                    relationship: RelationshipData {
                        name: rel_name,
                        relationship_type: input.relationship_type,
                        from_column: input.from_column,
                        to_column: input.to_column,
                    },
                }],
            ),
            Err(e) => ToolResponse::error(e),
        }
    }

    fn delete_relationship(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let from_table = match params.get("from_table").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: from_table"),
        };

        let from_column = match params.get("from_column").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: from_column"),
        };

        let to_table = match params.get("to_table").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: to_table"),
        };

        let to_column = match params.get("to_column").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResponse::error("Missing required parameter: to_column"),
        };

        let from_idx = match graph.find_table_by_name(from_table) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Source table '{}' not found", from_table)),
        };

        let to_idx = match graph.find_table_by_name(to_table) {
            Some(idx) => idx,
            None => return ToolResponse::error(format!("Target table '{}' not found", to_table)),
        };

        let edge_idx =
            match graph.find_relationship_by_columns(from_idx, to_idx, from_column, to_column) {
                Some(idx) => idx,
                None => {
                    return ToolResponse::error(format!(
                        "Relationship not found: {}.{} -> {}.{}",
                        from_table, from_column, to_table, to_column
                    ));
                }
            };

        let edge_id = edge_idx.index() as u32;
        match graph.delete_relationship(edge_idx) {
            Ok(_) => ToolResponse::success_with_ops(
                format!(
                    "Relationship deleted: {}.{} -> {}.{}",
                    from_table, from_column, to_table, to_column
                ),
                vec![GraphOperation::DeleteRelationship { edge_id }],
            ),
            Err(e) => ToolResponse::error(e),
        }
    }

    // ========================================================================
    // SQL-based operations
    // ========================================================================

    /// Apply SQL DDL statements to modify the schema
    /// This is a simplified parser that handles common DDL patterns
    fn apply_sql(graph: &mut SchemaGraph, params: &serde_json::Value) -> ToolResponse {
        let sql = match params.get("sql").and_then(|v| v.as_str()) {
            Some(sql) => sql,
            None => return ToolResponse::error("Missing required parameter: sql"),
        };

        let mut applied_statements = Vec::new();
        let mut errors = Vec::new();

        // Split by semicolons and process each statement
        for statement in sql.split(';') {
            let statement = statement.trim();
            if statement.is_empty() || statement.starts_with("--") {
                continue;
            }

            let upper = statement.to_uppercase();

            if upper.starts_with("CREATE TABLE") {
                match Self::parse_create_table(statement, graph) {
                    Ok(table_name) => {
                        applied_statements.push(format!("Created table: {}", table_name))
                    }
                    Err(e) => errors.push(format!("CREATE TABLE error: {}", e)),
                }
            } else if upper.starts_with("DROP TABLE") {
                match Self::parse_drop_table(statement, graph) {
                    Ok(table_name) => {
                        applied_statements.push(format!("Dropped table: {}", table_name))
                    }
                    Err(e) => errors.push(format!("DROP TABLE error: {}", e)),
                }
            } else if upper.starts_with("ALTER TABLE") {
                match Self::parse_alter_table(statement, graph) {
                    Ok(msg) => applied_statements.push(msg),
                    Err(e) => errors.push(format!("ALTER TABLE error: {}", e)),
                }
            }
        }

        if errors.is_empty() {
            ToolResponse::success_with_data(
                format!("Applied {} SQL statements", applied_statements.len()),
                serde_json::json!({
                    "applied": applied_statements,
                }),
            )
        } else {
            ToolResponse::error(format!(
                "SQL execution completed with errors. Applied: {:?}, Errors: {:?}",
                applied_statements, errors
            ))
        }
    }

    // ========================================================================
    // Validation operations
    // ========================================================================

    /// Validate SQL DDL statements without applying them
    fn validate_sql_tool(params: &serde_json::Value) -> ToolResponse {
        let sql = match params.get("sql").and_then(|v| v.as_str()) {
            Some(sql) => sql,
            None => return ToolResponse::error("Missing required parameter: sql"),
        };

        let dialect = params
            .get("dialect")
            .and_then(|v| v.as_str())
            .unwrap_or("mysql");

        let sql_dialect = match dialect.to_lowercase().as_str() {
            "mysql" => SqlDialect::MySQL,
            "postgresql" | "postgres" => SqlDialect::PostgreSQL,
            "sqlite" => SqlDialect::SQLite,
            _ => SqlDialect::MySQL,
        };

        let result = validate_sql(sql, sql_dialect);

        let message = if result.is_valid {
            format!(
                "SQL is valid. Found {} tables, {} relationships.",
                result.stats.table_count, result.stats.relationship_count
            )
        } else {
            format!(
                "SQL validation failed: {} errors, {} warnings.",
                result.stats.error_count, result.stats.warning_count
            )
        };

        ToolResponse::success_with_validation(message, result)
    }

    /// Check the current schema for validity
    fn check_schema(graph: &SchemaGraph) -> ToolResponse {
        let result = crate::core::check_schema_sql(graph, SqlDialect::MySQL);

        let message = if result.is_valid {
            format!(
                "Schema is valid. {} tables, {} relationships.",
                result.stats.table_count, result.stats.relationship_count
            )
        } else {
            format!(
                "Schema has issues: {} errors, {} warnings.",
                result.stats.error_count, result.stats.warning_count
            )
        };

        ToolResponse::success_with_validation(message, result)
    }

    /// Strip quote characters from identifier (backtick, double quote, single quote)
    fn strip_quotes(s: &str) -> &str {
        s.trim_matches(|c: char| c == '`' || c == '"' || c == '\'')
    }

    /// Extract table name from CREATE TABLE statement
    fn extract_create_table_name(statement: &str) -> Option<&str> {
        let upper = statement.to_uppercase();

        // Find position after "CREATE TABLE" or "CREATE TABLE IF NOT EXISTS"
        let start_pos = if let Some(pos) = upper.find("CREATE TABLE") {
            let after_create = pos + "CREATE TABLE".len();
            let rest = &upper[after_create..];

            // Check for IF NOT EXISTS
            let skip = if rest.trim_start().starts_with("IF NOT EXISTS") {
                rest.find("IF NOT EXISTS").unwrap() + "IF NOT EXISTS".len()
            } else {
                0
            };

            after_create + skip
        } else {
            return None;
        };

        // Now extract the table name from the original statement
        let rest = statement[start_pos..].trim_start();

        // Find the end of the table name (space or opening parenthesis)
        let end_pos = rest
            .find(|c: char| c.is_whitespace() || c == '(')
            .unwrap_or(rest.len());
        let name = &rest[..end_pos];

        Some(Self::strip_quotes(name))
    }

    /// Extract table name from DROP TABLE statement
    fn extract_drop_table_name(statement: &str) -> Option<&str> {
        let upper = statement.to_uppercase();

        let start_pos = if let Some(pos) = upper.find("DROP TABLE") {
            let after_drop = pos + "DROP TABLE".len();
            let rest = &upper[after_drop..];

            // Check for IF EXISTS
            let skip = if rest.trim_start().starts_with("IF EXISTS") {
                rest.find("IF EXISTS").unwrap() + "IF EXISTS".len()
            } else {
                0
            };

            after_drop + skip
        } else {
            return None;
        };

        let rest = statement[start_pos..].trim_start();
        let end_pos = rest
            .find(|c: char| c.is_whitespace() || c == ';')
            .unwrap_or(rest.len());
        let name = &rest[..end_pos];

        Some(Self::strip_quotes(name))
    }

    /// Extract table name from ALTER TABLE statement
    fn extract_alter_table_name(statement: &str) -> Option<&str> {
        let upper = statement.to_uppercase();

        let start_pos = upper.find("ALTER TABLE")? + "ALTER TABLE".len();
        let rest = statement[start_pos..].trim_start();

        let end_pos = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        let name = &rest[..end_pos];

        Some(Self::strip_quotes(name))
    }

    /// Parse CREATE TABLE statement (simplified, without regex)
    fn parse_create_table(statement: &str, graph: &mut SchemaGraph) -> Result<String, String> {
        // Extract table name
        let table_name = Self::extract_create_table_name(statement)
            .ok_or_else(|| "Could not parse table name".to_string())?;

        // Check if table exists
        if graph.table_exists(table_name) {
            return Err(format!("Table {} already exists", table_name));
        }

        // Extract column definitions (simplified parsing)
        let start = statement
            .find('(')
            .ok_or_else(|| "Missing opening parenthesis".to_string())?;
        let end = statement
            .rfind(')')
            .ok_or_else(|| "Missing closing parenthesis".to_string())?;
        let columns_str = &statement[start + 1..end];

        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();

        // Simple comma splitting (doesn't handle nested parentheses perfectly)
        for part in Self::split_column_definitions(columns_str) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let upper = part.to_uppercase();

            // Skip constraint definitions for now
            if upper.starts_with("PRIMARY KEY")
                || upper.starts_with("FOREIGN KEY")
                || upper.starts_with("UNIQUE")
                || upper.starts_with("CONSTRAINT")
                || upper.starts_with("INDEX")
                || upper.starts_with("KEY")
            {
                // Extract primary key columns
                if upper.starts_with("PRIMARY KEY")
                    && let Some(pk_start) = part.find('(')
                    && let Some(pk_end) = part.rfind(')')
                {
                    for col in part[pk_start + 1..pk_end].split(',') {
                        let col = Self::strip_quotes(col.trim());
                        primary_keys.push(col.to_string());
                    }
                }
                continue;
            }

            // Parse column definition
            let tokens: Vec<&str> = part.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let col_name = Self::strip_quotes(tokens[0]);
            let data_type = if tokens.len() > 1 {
                // Reconstruct data type (might have parentheses)
                let mut dt = tokens[1].to_string();
                for i in 2..tokens.len() {
                    let t = tokens[i].to_uppercase();
                    if t == "NOT"
                        || t == "NULL"
                        || t == "DEFAULT"
                        || t == "PRIMARY"
                        || t == "UNIQUE"
                        || t == "AUTO_INCREMENT"
                    {
                        break;
                    }
                    if !tokens[i - 1].ends_with(')') && !t.starts_with('(') {
                        break;
                    }
                    dt.push_str(tokens[i]);
                }
                dt
            } else {
                "VARCHAR(255)".to_string()
            };

            let is_primary_key = upper.contains("PRIMARY KEY");
            let is_not_null = upper.contains("NOT NULL") || is_primary_key;
            let is_unique = upper.contains("UNIQUE");

            let mut col = Column::new(col_name, &data_type);
            col.is_primary_key = is_primary_key;
            col.is_nullable = !is_not_null;
            col.is_unique = is_unique;
            columns.push(col);
        }

        // Apply primary key constraint to columns
        for col in &mut columns {
            if primary_keys.contains(&col.name) {
                col.is_primary_key = true;
                col.is_nullable = false;
            }
        }

        // Create the table
        let idx = graph.create_table(table_name, (100.0, 100.0))?;
        if let Some(table) = graph.node_weight_mut(idx) {
            table.columns = columns;
        }

        Ok(table_name.to_string())
    }

    /// Split column definitions respecting parentheses
    fn split_column_definitions(s: &str) -> Vec<&str> {
        let mut result = Vec::new();
        let mut depth = 0;
        let mut start = 0;

        for (i, c) in s.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    result.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }

        // Don't forget the last part
        if start < s.len() {
            result.push(&s[start..]);
        }

        result
    }

    /// Parse DROP TABLE statement (without regex)
    fn parse_drop_table(statement: &str, graph: &mut SchemaGraph) -> Result<String, String> {
        let table_name = Self::extract_drop_table_name(statement)
            .ok_or_else(|| "Could not parse table name".to_string())?;

        let idx = graph
            .find_table_by_name(table_name)
            .ok_or_else(|| format!("Table {} not found", table_name))?;

        graph.delete_table(idx)?;
        Ok(table_name.to_string())
    }

    /// Parse ALTER TABLE statement (simplified, without regex)
    fn parse_alter_table(statement: &str, graph: &mut SchemaGraph) -> Result<String, String> {
        let table_name = Self::extract_alter_table_name(statement)
            .ok_or_else(|| "Could not parse table name".to_string())?;

        let idx = graph
            .find_table_by_name(table_name)
            .ok_or_else(|| format!("Table {} not found", table_name))?;

        let upper = statement.to_uppercase();

        // ADD COLUMN
        if upper.contains("ADD COLUMN")
            || (upper.contains(" ADD ") && !upper.contains("ADD CONSTRAINT"))
        {
            let add_pos = upper
                .find("ADD COLUMN")
                .or_else(|| upper.find(" ADD ").map(|p| p + 1))
                .ok_or_else(|| "Could not find ADD".to_string())?;

            let after_add = if upper[add_pos..].starts_with("ADD COLUMN") {
                add_pos + "ADD COLUMN".len()
            } else {
                add_pos + "ADD".len()
            };

            let rest = statement[after_add..].trim();
            let tokens: Vec<&str> = rest.split_whitespace().collect();

            if tokens.len() >= 2 {
                let col_name = Self::strip_quotes(tokens[0]);
                let data_type = tokens[1];

                if let Some(table) = graph.node_weight_mut(idx) {
                    let col = Column::new(col_name, data_type);
                    table.columns.push(col);
                    return Ok(format!("Added column {} to table {}", col_name, table_name));
                }
            }
        }

        // DROP COLUMN
        if upper.contains("DROP COLUMN")
            || (upper.contains(" DROP ") && !upper.contains("DROP CONSTRAINT"))
        {
            let drop_pos = upper
                .find("DROP COLUMN")
                .or_else(|| upper.find(" DROP ").map(|p| p + 1))
                .ok_or_else(|| "Could not find DROP".to_string())?;

            let after_drop = if upper[drop_pos..].starts_with("DROP COLUMN") {
                drop_pos + "DROP COLUMN".len()
            } else {
                drop_pos + "DROP".len()
            };

            let rest = statement[after_drop..].trim();
            let col_name = rest
                .split_whitespace()
                .next()
                .map(Self::strip_quotes)
                .unwrap_or("");

            if let Some(table) = graph.node_weight_mut(idx)
                && let Some((col_idx, _)) = table.find_column(col_name)
            {
                table.delete_column(col_idx)?;
                return Ok(format!(
                    "Dropped column {} from table {}",
                    col_name, table_name
                ));
            }
        }

        // RENAME TO
        if upper.contains("RENAME TO") {
            let rename_pos = upper
                .find("RENAME TO")
                .ok_or_else(|| "Could not find RENAME TO".to_string())?;

            let rest = statement[rename_pos + "RENAME TO".len()..].trim();
            let new_name = rest
                .split_whitespace()
                .next()
                .map(Self::strip_quotes)
                .unwrap_or("");

            if !new_name.is_empty() {
                graph.rename_table(idx, new_name)?;
                return Ok(format!("Renamed table {} to {}", table_name, new_name));
            }
        }

        Err(format!(
            "Could not parse ALTER TABLE statement: {}",
            statement
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::create_demo_graph;

    #[test]
    fn test_list_tables() {
        let mut graph = create_demo_graph();
        let request = ToolRequest {
            tool_name: "list_tables".into(),
            parameters: serde_json::json!({}),
        };

        let response = ToolExecutor::execute(&mut graph, &request);
        assert!(response.success);
    }

    #[test]
    fn test_create_table() {
        let mut graph = SchemaGraph::new();
        let request = ToolRequest {
            tool_name: "create_table".into(),
            parameters: serde_json::json!({
                "name": "test_table",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INT",
                        "is_primary_key": true
                    },
                    {
                        "name": "name",
                        "data_type": "VARCHAR(255)"
                    }
                ]
            }),
        };

        let response = ToolExecutor::execute(&mut graph, &request);
        assert!(response.success);
        assert!(graph.table_exists("test_table"));
    }

    #[test]
    fn test_add_column() {
        let mut graph = create_demo_graph();
        let request = ToolRequest {
            tool_name: "add_column".into(),
            parameters: serde_json::json!({
                "table_name": "users",
                "column_name": "phone",
                "data_type": "VARCHAR(20)"
            }),
        };

        let response = ToolExecutor::execute(&mut graph, &request);
        assert!(response.success);
    }

    #[test]
    fn test_get_tool_definitions() {
        let tools = get_tool_definitions();
        assert!(!tools.is_empty());

        // Check that all essential tools are defined
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"create_table"));
        assert!(tool_names.contains(&"add_column"));
        assert!(tool_names.contains(&"create_relationship"));
        assert!(tool_names.contains(&"get_schema_sql"));
    }
}
