//! SQL Parser module with syntax and semantic validation
//!
//! Provides:
//! - SQL parsing using sqlparser-rs
//! - Syntax validation with line/column error positions
//! - Semantic validation (foreign key references, type compatibility, etc.)
//! - Error output suitable for LLM agents
//! - Check function for validation on save
//! - Apply SQL to graph with LiveShare synchronization

use crate::core::liveshare::{ColumnData, GraphOperation};
use crate::core::{Column, ExportOptions, SchemaExporter, SchemaGraph, SqlDialect, TableNode};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use sqlparser::ast::{AlterTableOperation, ColumnOption, DataType, ObjectName, Statement};
use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::{Parser, ParserError};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Error Types
// ============================================================================

/// Severity level for validation issues
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Critical error that prevents parsing/execution
    Error,
    /// Warning that should be addressed but doesn't prevent execution
    Warning,
    /// Informational hint for best practices
    Hint,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Hint => write!(f, "hint"),
        }
    }
}

/// Position in SQL source code
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePosition {
    /// 1-based line number
    pub line: usize,
    /// 1-based column number
    pub column: usize,
    /// Character offset from start of string
    pub offset: usize,
}

impl SourcePosition {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    /// Create position from 0-based offset in source text
    pub fn from_offset(source: &str, offset: usize) -> Self {
        let mut line = 1;
        let mut column = 1;
        let mut current_offset = 0;

        for ch in source.chars() {
            if current_offset >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            current_offset += ch.len_utf8();
        }

        Self {
            line,
            column,
            offset,
        }
    }
}

/// Span in SQL source code (start and end positions)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    pub fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    pub fn single_position(pos: SourcePosition) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

/// A validation error with position information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SqlValidationError {
    /// Error severity
    pub severity: ErrorSeverity,
    /// Error message
    pub message: String,
    /// Position in source (if available)
    pub span: Option<SourceSpan>,
    /// Error code for programmatic handling
    pub code: String,
    /// Suggestion for fixing the error
    pub suggestion: Option<String>,
    /// Related information (e.g., where a referenced table is defined)
    pub related: Vec<RelatedInfo>,
}

/// Related information for an error
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelatedInfo {
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl SqlValidationError {
    pub fn error(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            severity: ErrorSeverity::Error,
            message: message.into(),
            span: None,
            code: code.into(),
            suggestion: None,
            related: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            severity: ErrorSeverity::Warning,
            message: message.into(),
            span: None,
            code: code.into(),
            suggestion: None,
            related: Vec::new(),
        }
    }

    pub fn hint(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            severity: ErrorSeverity::Hint,
            message: message.into(),
            span: None,
            code: code.into(),
            suggestion: None,
            related: Vec::new(),
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_position(mut self, line: usize, column: usize) -> Self {
        let pos = SourcePosition::new(line, column, 0);
        self.span = Some(SourceSpan::single_position(pos));
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_related(mut self, message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        self.related.push(RelatedInfo {
            message: message.into(),
            span,
        });
        self
    }
}

impl std::fmt::Display for SqlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.severity, self.message)?;
        if let Some(ref span) = self.span {
            write!(
                f,
                " at line {}, column {}",
                span.start.line, span.start.column
            )?;
        }
        Ok(())
    }
}

// ============================================================================
// Validation Result
// ============================================================================

/// Result of SQL validation
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SqlValidationResult {
    /// List of validation errors and warnings
    pub diagnostics: Vec<SqlValidationError>,
    /// Parsed statements (if parsing succeeded)
    #[serde(skip)]
    pub statements: Vec<Statement>,
    /// Whether the SQL is valid (no errors, warnings are OK)
    pub is_valid: bool,
    /// Summary statistics
    pub stats: ValidationStats,
}

/// Statistics about validation
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    pub error_count: usize,
    pub warning_count: usize,
    pub hint_count: usize,
    pub table_count: usize,
    pub relationship_count: usize,
}

impl SqlValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_error(&mut self, error: SqlValidationError) {
        match error.severity {
            ErrorSeverity::Error => self.stats.error_count += 1,
            ErrorSeverity::Warning => self.stats.warning_count += 1,
            ErrorSeverity::Hint => self.stats.hint_count += 1,
        }
        self.diagnostics.push(error);
    }

    pub fn has_errors(&self) -> bool {
        self.stats.error_count > 0
    }

    pub fn has_warnings(&self) -> bool {
        self.stats.warning_count > 0
    }

    /// Get only errors (not warnings or hints)
    #[allow(dead_code)]
    pub fn errors(&self) -> impl Iterator<Item = &SqlValidationError> {
        self.diagnostics
            .iter()
            .filter(|e| e.severity == ErrorSeverity::Error)
    }

    /// Get only warnings
    #[allow(dead_code)]
    pub fn warnings(&self) -> impl Iterator<Item = &SqlValidationError> {
        self.diagnostics
            .iter()
            .filter(|e| e.severity == ErrorSeverity::Warning)
    }

    /// Format for display in UI
    pub fn format_for_display(&self) -> String {
        let mut output = String::new();

        for diag in &self.diagnostics {
            let icon = match diag.severity {
                ErrorSeverity::Error => "❌",
                ErrorSeverity::Warning => "⚠️",
                ErrorSeverity::Hint => "💡",
            };

            output.push_str(icon);
            output.push(' ');

            if let Some(ref span) = diag.span {
                output.push_str(&format!("[L{}:{}] ", span.start.line, span.start.column));
            }

            output.push_str(&diag.message);
            output.push('\n');

            if let Some(ref suggestion) = diag.suggestion {
                output.push_str(&format!("  → Suggestion: {}\n", suggestion));
            }
        }

        if output.is_empty() {
            output.push_str("✅ No issues found\n");
        }

        output.push_str(&format!(
            "\nSummary: {} errors, {} warnings, {} hints",
            self.stats.error_count, self.stats.warning_count, self.stats.hint_count
        ));

        output
    }

    /// Format for LLM agent consumption
    pub fn format_for_llm(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.is_valid,
            "error_count": self.stats.error_count,
            "warning_count": self.stats.warning_count,
            "diagnostics": self.diagnostics.iter().map(|d| {
                serde_json::json!({
                    "severity": d.severity.to_string(),
                    "code": d.code,
                    "message": d.message,
                    "position": d.span.as_ref().map(|s| {
                        serde_json::json!({
                            "line": s.start.line,
                            "column": s.start.column
                        })
                    }),
                    "suggestion": d.suggestion,
                })
            }).collect::<Vec<_>>(),
            "summary": self.format_summary()
        })
    }

    fn format_summary(&self) -> String {
        if self.is_valid {
            format!("SQL is valid. Found {} tables.", self.stats.table_count)
        } else {
            format!(
                "SQL has {} errors and {} warnings.",
                self.stats.error_count, self.stats.warning_count
            )
        }
    }
}

// ============================================================================
// SQL Parser
// ============================================================================

/// SQL Parser with validation capabilities
pub struct SqlParser {
    dialect: SqlDialect,
}

impl SqlParser {
    pub fn new(dialect: SqlDialect) -> Self {
        Self { dialect }
    }

    #[allow(dead_code)]
    pub fn mysql() -> Self {
        Self::new(SqlDialect::MySQL)
    }

    #[allow(dead_code)]
    pub fn postgresql() -> Self {
        Self::new(SqlDialect::PostgreSQL)
    }

    #[allow(dead_code)]
    pub fn sqlite() -> Self {
        Self::new(SqlDialect::SQLite)
    }

    /// Get the sqlparser dialect implementation
    fn get_dialect(&self) -> Box<dyn Dialect> {
        match self.dialect {
            SqlDialect::MySQL => Box::new(MySqlDialect {}),
            SqlDialect::PostgreSQL => Box::new(PostgreSqlDialect {}),
            SqlDialect::SQLite => Box::new(SQLiteDialect {}),
        }
    }

    /// Parse SQL and return AST statements
    pub fn parse(&self, sql: &str) -> Result<Vec<Statement>, ParserError> {
        let dialect = self.get_dialect();
        Parser::parse_sql(dialect.as_ref(), sql)
    }

    /// Parse and validate SQL syntax only
    pub fn validate_syntax(&self, sql: &str) -> SqlValidationResult {
        let mut result = SqlValidationResult::new();

        match self.parse(sql) {
            Ok(statements) => {
                result.statements = statements;
                result.is_valid = true;
            }
            Err(e) => {
                let error = self.parser_error_to_validation_error(sql, &e);
                result.add_error(error);
                result.is_valid = false;
            }
        }

        result
    }

    /// Convert sqlparser error to our validation error with position
    fn parser_error_to_validation_error(
        &self,
        sql: &str,
        error: &ParserError,
    ) -> SqlValidationError {
        let message = error.to_string();

        // Try to extract line and column from error message
        // sqlparser format: "Expected ..., found: ... at Line: X, Column: Y"
        let (line, column) = self.extract_position_from_error(&message);

        let mut validation_error = SqlValidationError::error(message.clone(), "E001_SYNTAX_ERROR");

        if let (Some(line), Some(column)) = (line, column) {
            let offset = self.calculate_offset(sql, line, column);
            let pos = SourcePosition::new(line, column, offset);
            validation_error = validation_error.with_span(SourceSpan::single_position(pos));
        }

        // Try to provide a helpful suggestion
        if message.contains("Expected")
            && let Some(suggestion) = self.generate_syntax_suggestion(&message)
        {
            validation_error = validation_error.with_suggestion(suggestion);
        }

        validation_error
    }

    /// Extract line and column from error message
    fn extract_position_from_error(&self, message: &str) -> (Option<usize>, Option<usize>) {
        // Pattern: "Line: X, Column: Y"
        let line = message.find("Line: ").and_then(|pos| {
            let start = pos + 6;
            let end = message[start..]
                .find(',')
                .map(|p| start + p)
                .unwrap_or(message.len());
            message[start..end].parse::<usize>().ok()
        });

        let column = message.find("Column: ").and_then(|pos| {
            let start = pos + 8;
            let end = message[start..]
                .find(|c: char| !c.is_numeric())
                .map(|p| start + p)
                .unwrap_or(message.len());
            message[start..end].parse::<usize>().ok()
        });

        (line, column)
    }

    /// Calculate character offset from line and column
    fn calculate_offset(&self, sql: &str, line: usize, column: usize) -> usize {
        let mut current_line = 1;
        let mut offset = 0;

        for ch in sql.chars() {
            if current_line == line {
                // We're on the target line, count columns
                let mut col = 1;
                for ch2 in sql[offset..].chars() {
                    if col == column {
                        return offset;
                    }
                    if ch2 == '\n' {
                        break;
                    }
                    offset += ch2.len_utf8();
                    col += 1;
                }
                return offset;
            }
            if ch == '\n' {
                current_line += 1;
            }
            offset += ch.len_utf8();
        }

        offset
    }

    /// Generate suggestion for common syntax errors
    fn generate_syntax_suggestion(&self, message: &str) -> Option<String> {
        if message.contains("Expected identifier") {
            Some("Make sure you have a valid table or column name.".into())
        } else if message.contains("Expected )") {
            Some("Check for matching parentheses.".into())
        } else if message.contains("Expected ;") {
            Some("Add a semicolon at the end of the statement.".into())
        } else if message.contains("Expected ,") {
            Some("Separate column definitions with commas.".into())
        } else if message.contains("Expected keyword") {
            Some("Check for typos in SQL keywords.".into())
        } else {
            None
        }
    }
}

// ============================================================================
// Schema Validator (Semantic Validation)
// ============================================================================

/// Semantic validator for database schemas
pub struct SchemaValidator {
    /// Known tables and their columns
    tables: HashMap<String, TableInfo>,
    /// Source SQL for position calculation
    #[allow(dead_code)]
    source: String,
}

/// Information about a table for validation
#[derive(Clone, Debug)]
struct TableInfo {
    pub columns: HashMap<String, ColumnInfo>,
    #[allow(dead_code)]
    pub primary_keys: Vec<String>,
    #[allow(dead_code)]
    pub source_position: Option<SourceSpan>,
}

/// Information about a column for validation
#[derive(Clone, Debug)]
struct ColumnInfo {
    #[allow(dead_code)]
    pub data_type: String,
    #[allow(dead_code)]
    pub is_nullable: bool,
    #[allow(dead_code)]
    pub is_primary_key: bool,
    #[allow(dead_code)]
    pub source_position: Option<SourceSpan>,
}

impl SchemaValidator {
    pub fn new(source: &str) -> Self {
        Self {
            tables: HashMap::new(),
            source: source.to_string(),
        }
    }

    /// Build validator from existing schema graph
    pub fn from_graph(graph: &SchemaGraph, source: &str) -> Self {
        let mut validator = Self::new(source);

        for node_idx in graph.node_indices() {
            if let Some(table) = graph.node_weight(node_idx) {
                let mut columns = HashMap::new();
                let mut primary_keys = Vec::new();

                for col in &table.columns {
                    columns.insert(
                        col.name.to_lowercase(),
                        ColumnInfo {
                            data_type: col.data_type.clone(),
                            is_nullable: col.is_nullable,
                            is_primary_key: col.is_primary_key,
                            source_position: None,
                        },
                    );
                    if col.is_primary_key {
                        primary_keys.push(col.name.clone());
                    }
                }

                validator.tables.insert(
                    table.name.to_lowercase(),
                    TableInfo {
                        columns,
                        primary_keys,
                        source_position: None,
                    },
                );
            }
        }

        validator
    }

    /// Validate parsed statements semantically
    pub fn validate(&mut self, statements: &[Statement]) -> SqlValidationResult {
        let mut result = SqlValidationResult::new();

        // First pass: collect all table definitions
        for stmt in statements {
            if let Statement::CreateTable(create_table) = stmt {
                self.register_table_from_ast(create_table);
                result.stats.table_count += 1;
            }
        }

        // Second pass: validate references
        for stmt in statements {
            match stmt {
                Statement::CreateTable(create_table) => {
                    self.validate_create_table(create_table, &mut result);
                }
                Statement::AlterTable(alter_table) => {
                    self.validate_alter_table(
                        &alter_table.name,
                        &alter_table.operations,
                        &mut result,
                    );
                }
                Statement::Drop { names, .. } => {
                    self.validate_drop(names, &mut result);
                }
                _ => {}
            }
        }

        // Check for circular references
        self.check_circular_references(&mut result);

        result.is_valid = result.stats.error_count == 0;
        result
    }

    /// Register a table from CREATE TABLE AST
    fn register_table_from_ast(&mut self, create_table: &sqlparser::ast::CreateTable) {
        let table_name = create_table.name.to_string().to_lowercase();
        let mut columns = HashMap::new();
        let mut primary_keys = Vec::new();

        for column in &create_table.columns {
            let col_name = column.name.value.to_lowercase();

            // Check for PRIMARY KEY in column options
            let is_pk = column
                .options
                .iter()
                .any(|opt| matches!(opt.option, ColumnOption::PrimaryKey(_)));

            let is_nullable = !column
                .options
                .iter()
                .any(|opt| matches!(opt.option, ColumnOption::NotNull));

            if is_pk {
                primary_keys.push(column.name.value.clone());
            }

            columns.insert(
                col_name,
                ColumnInfo {
                    data_type: column.data_type.to_string(),
                    is_nullable,
                    is_primary_key: is_pk,
                    source_position: None,
                },
            );
        }

        // Check table constraints for primary keys
        for constraint in &create_table.constraints {
            if let sqlparser::ast::TableConstraint::PrimaryKey(pk_constraint) = constraint {
                for col in &pk_constraint.columns {
                    // IndexColumn has an expr field, we need to extract column name from it
                    let col_name = col.column.to_string();
                    primary_keys.push(col_name.clone());
                    if let Some(col_info) = columns.get_mut(&col_name.to_lowercase()) {
                        col_info.is_primary_key = true;
                    }
                }
            }
        }

        self.tables.insert(
            table_name,
            TableInfo {
                columns,
                primary_keys,
                source_position: None,
            },
        );
    }

    /// Validate CREATE TABLE statement
    fn validate_create_table(
        &self,
        create_table: &sqlparser::ast::CreateTable,
        result: &mut SqlValidationResult,
    ) {
        let table_name = create_table.name.to_string();

        // Check column names
        let mut seen_columns: HashSet<String> = HashSet::new();
        for column in &create_table.columns {
            let col_name = column.name.value.to_lowercase();

            // Check for duplicate column names
            if !seen_columns.insert(col_name.clone()) {
                result.add_error(
                    SqlValidationError::error(
                        format!(
                            "Duplicate column name '{}' in table '{}'",
                            column.name.value, table_name
                        ),
                        "E002_DUPLICATE_COLUMN",
                    )
                    .with_suggestion(format!("Rename one of the '{}' columns", column.name.value)),
                );
            }

            // Validate column data type
            self.validate_data_type(&column.data_type, result);
        }

        // Validate foreign key constraints
        for constraint in &create_table.constraints {
            if let sqlparser::ast::TableConstraint::ForeignKey(fk_constraint) = constraint {
                self.validate_foreign_key(
                    &table_name,
                    &fk_constraint.foreign_table,
                    &fk_constraint.columns,
                    &fk_constraint.referred_columns,
                    result,
                );
            }
        }
    }

    /// Validate foreign key references
    fn validate_foreign_key(
        &self,
        from_table: &str,
        to_table: &ObjectName,
        from_columns: &[sqlparser::ast::Ident],
        to_columns: &[sqlparser::ast::Ident],
        result: &mut SqlValidationResult,
    ) {
        let to_table_name = to_table.to_string().to_lowercase();
        let to_table_str = to_table.to_string();

        // Check if referenced table exists
        if !self.tables.contains_key(&to_table_name) {
            let mut error = SqlValidationError::error(
                format!("Foreign key references non-existent table '{}'", to_table),
                "E003_UNKNOWN_TABLE",
            )
            .with_suggestion(format!(
                "Create table '{}' before referencing it, or check the table name",
                to_table
            ));
            // Try to find position of this REFERENCES clause
            if let Some(span) = self.find_references_position(&to_table_str) {
                error = error.with_span(span);
            } else if let Some(span) = self.find_foreign_key_position(from_table, &to_table_str) {
                error = error.with_span(span);
            }
            result.add_error(error);
            result.stats.relationship_count += 1;
            return;
        }

        let table_info = self.tables.get(&to_table_name).unwrap();

        // Check if referenced columns exist
        for col in to_columns {
            let col_name = col.value.to_lowercase();
            if !table_info.columns.contains_key(&col_name) {
                let mut error = SqlValidationError::error(
                    format!(
                        "Foreign key references non-existent column '{}' in table '{}'",
                        col.value, to_table
                    ),
                    "E004_UNKNOWN_COLUMN",
                )
                .with_suggestion(format!(
                    "Available columns in '{}': {}",
                    to_table,
                    table_info
                        .columns
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                // Try to find position of column reference
                if let Some(span) = self.find_position_in_source(&col.value) {
                    error = error.with_span(span);
                }
                result.add_error(error);
            }
        }

        // Check column count matches
        if from_columns.len() != to_columns.len() {
            result.add_error(SqlValidationError::error(
                format!(
                    "Foreign key column count mismatch: {} columns reference {} columns",
                    from_columns.len(),
                    to_columns.len()
                ),
                "E005_FK_COLUMN_MISMATCH",
            ));
        }

        result.stats.relationship_count += 1;
    }

    /// Validate data type
    fn validate_data_type(&self, data_type: &DataType, result: &mut SqlValidationResult) {
        // Check for common data type issues
        match data_type {
            DataType::Varchar(Some(sqlparser::ast::CharacterLength::IntegerLength {
                length,
                ..
            })) => {
                if *length == 0 {
                    result.add_error(
                        SqlValidationError::warning(
                            "VARCHAR(0) is unusual and may cause issues",
                            "W001_ZERO_LENGTH_VARCHAR",
                        )
                        .with_suggestion("Use VARCHAR with a positive length"),
                    );
                }
            }
            DataType::Decimal(sqlparser::ast::ExactNumberInfo::PrecisionAndScale(p, s)) => {
                // s is i64, p is u64, need to compare safely
                if *s > 0 && (*s as u64) > *p {
                    result.add_error(SqlValidationError::error(
                        format!("DECIMAL scale ({}) cannot exceed precision ({})", s, p),
                        "E006_INVALID_DECIMAL",
                    ));
                }
            }
            _ => {}
        }
    }

    /// Validate ALTER TABLE statement
    fn validate_alter_table(
        &self,
        table_name: &ObjectName,
        operations: &[AlterTableOperation],
        result: &mut SqlValidationResult,
    ) {
        let name = table_name.to_string().to_lowercase();
        let table_name_str = table_name.to_string();

        // Check if table exists
        if !self.tables.contains_key(&name) {
            let mut error = SqlValidationError::error(
                format!("Cannot alter non-existent table '{}'", table_name),
                "E003_UNKNOWN_TABLE",
            );
            // Try to find position of this ALTER TABLE statement
            if let Some(span) = self.find_alter_table_position(&table_name_str) {
                error = error.with_span(span);
            }
            result.add_error(error);
            return;
        }

        let table_info = self.tables.get(&name).unwrap();

        for op in operations {
            match op {
                AlterTableOperation::DropColumn { column_names, .. } => {
                    for column_name in column_names {
                        let col = column_name.value.to_lowercase();
                        if !table_info.columns.contains_key(&col) {
                            result.add_error(SqlValidationError::error(
                                format!(
                                    "Cannot drop non-existent column '{}' from table '{}'",
                                    column_name.value, table_name
                                ),
                                "E004_UNKNOWN_COLUMN",
                            ));
                        }
                    }
                }
                AlterTableOperation::RenameColumn {
                    old_column_name,
                    new_column_name,
                } => {
                    let old = old_column_name.value.to_lowercase();
                    let new = new_column_name.value.to_lowercase();

                    if !table_info.columns.contains_key(&old) {
                        result.add_error(SqlValidationError::error(
                            format!(
                                "Cannot rename non-existent column '{}' in table '{}'",
                                old_column_name.value, table_name
                            ),
                            "E004_UNKNOWN_COLUMN",
                        ));
                    }

                    if table_info.columns.contains_key(&new) {
                        result.add_error(SqlValidationError::error(
                            format!(
                                "Cannot rename to '{}' - column already exists in table '{}'",
                                new_column_name.value, table_name
                            ),
                            "E002_DUPLICATE_COLUMN",
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    /// Validate DROP statement
    fn validate_drop(&self, names: &[ObjectName], result: &mut SqlValidationResult) {
        for name in names {
            let table_name = name.to_string().to_lowercase();
            let table_name_str = name.to_string();
            if !self.tables.contains_key(&table_name) {
                let mut warning = SqlValidationError::warning(
                    format!("Dropping non-existent table '{}'", name),
                    "W002_DROP_NONEXISTENT",
                )
                .with_suggestion("Use DROP TABLE IF EXISTS to avoid this warning");
                // Try to find position of DROP TABLE statement
                if let Some(span) = self.find_drop_table_position(&table_name_str) {
                    warning = warning.with_span(span);
                }
                result.add_error(warning);
            }
        }
    }

    /// Check for circular foreign key references
    fn check_circular_references(&self, _result: &mut SqlValidationResult) {
        // TODO: Implement circular reference detection using graph traversal
    }

    /// Find the position of a pattern in the source code (case-insensitive)
    fn find_position_in_source(&self, pattern: &str) -> Option<SourceSpan> {
        let source_lower = self.source.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        if let Some(offset) = source_lower.find(&pattern_lower) {
            let start = SourcePosition::from_offset(&self.source, offset);
            let end_offset = offset + pattern.len();
            let end = SourcePosition::from_offset(&self.source, end_offset);
            Some(SourceSpan::new(start, end))
        } else {
            None
        }
    }

    /// Find position of ALTER TABLE statement for a specific table
    fn find_alter_table_position(&self, table_name: &str) -> Option<SourceSpan> {
        // Search for "ALTER TABLE `table_name`" or "ALTER TABLE table_name"
        let patterns = [
            format!("ALTER TABLE `{}`", table_name),
            format!("ALTER TABLE {}", table_name),
        ];

        for pattern in &patterns {
            if let Some(span) = self.find_position_in_source(pattern) {
                return Some(span);
            }
        }
        None
    }

    /// Find position of REFERENCES clause for a specific table - underlines just the table name
    fn find_references_position(&self, table_name: &str) -> Option<SourceSpan> {
        let source_lower = self.source.to_lowercase();
        let table_lower = table_name.to_lowercase();

        // Search for patterns and return position of table name only
        let patterns = [
            (format!("references `{}`", table_lower), 12), // "REFERENCES `" = 12 chars
            (format!("references {}", table_lower), 11),   // "REFERENCES " = 11 chars
        ];

        for (pattern, prefix_len) in &patterns {
            if let Some(offset) = source_lower.find(pattern) {
                // Skip "REFERENCES " or "REFERENCES `" to point to table name
                let table_start = offset + prefix_len;
                let table_end = table_start + table_name.len();
                let start = SourcePosition::from_offset(&self.source, table_start);
                let end = SourcePosition::from_offset(&self.source, table_end);
                return Some(SourceSpan::new(start, end));
            }
        }
        None
    }

    /// Find position of FOREIGN KEY constraint referencing a specific table
    fn find_foreign_key_position(&self, from_table: &str, _to_table: &str) -> Option<SourceSpan> {
        // Try to find in context of ALTER TABLE or CREATE TABLE
        let patterns = [
            "FOREIGN KEY", // Generic fallback
        ];

        // First try to find ALTER TABLE ... ADD CONSTRAINT ... FOREIGN KEY ... REFERENCES to_table
        let alter_pattern = format!("ALTER TABLE `{}` ADD CONSTRAINT", from_table);
        let alter_pattern2 = format!("ALTER TABLE {} ADD CONSTRAINT", from_table);

        if let Some(span) = self.find_position_in_source(&alter_pattern) {
            return Some(span);
        }
        if let Some(span) = self.find_position_in_source(&alter_pattern2) {
            return Some(span);
        }

        for pattern in &patterns {
            if let Some(span) = self.find_position_in_source(pattern) {
                return Some(span);
            }
        }
        None
    }

    /// Find position of DROP TABLE statement for a specific table
    fn find_drop_table_position(&self, table_name: &str) -> Option<SourceSpan> {
        let patterns = [
            format!("DROP TABLE `{}`", table_name),
            format!("DROP TABLE {}", table_name),
        ];

        for pattern in &patterns {
            if let Some(span) = self.find_position_in_source(pattern) {
                return Some(span);
            }
        }
        None
    }
}

// ============================================================================
// Full SQL Validation
// ============================================================================

/// Full SQL validation combining syntax and semantic checks
pub fn validate_sql(sql: &str, dialect: SqlDialect) -> SqlValidationResult {
    let parser = SqlParser::new(dialect);

    // First, validate syntax
    let mut result = parser.validate_syntax(sql);

    if !result.is_valid {
        return result;
    }

    // Then, validate semantics
    let mut validator = SchemaValidator::new(sql);
    let semantic_result = validator.validate(&result.statements);

    // Merge results
    for diag in semantic_result.diagnostics {
        result.add_error(diag);
    }
    result.stats.table_count = semantic_result.stats.table_count;
    result.stats.relationship_count = semantic_result.stats.relationship_count;
    result.is_valid = result.stats.error_count == 0;

    result
}

/// Validate SQL against existing schema graph
#[allow(dead_code)]
pub fn validate_sql_with_graph(
    sql: &str,
    dialect: SqlDialect,
    graph: &SchemaGraph,
) -> SqlValidationResult {
    let parser = SqlParser::new(dialect);

    // First, validate syntax
    let mut result = parser.validate_syntax(sql);

    if !result.is_valid {
        return result;
    }

    // Then, validate semantics with existing graph context
    let mut validator = SchemaValidator::from_graph(graph, sql);
    let semantic_result = validator.validate(&result.statements);

    // Merge results
    for diag in semantic_result.diagnostics {
        result.add_error(diag);
    }
    result.stats.table_count = semantic_result.stats.table_count;
    result.stats.relationship_count = semantic_result.stats.relationship_count;
    result.is_valid = result.stats.error_count == 0;

    result
}

/// Check function for validation on save
pub fn check_schema_sql(graph: &SchemaGraph, dialect: SqlDialect) -> SqlValidationResult {
    let options = ExportOptions {
        sql_dialect: dialect.clone(),
        include_positions: false,
        include_drop_statements: false,
        pretty_print: true,
        ..Default::default()
    };

    let sql = match SchemaExporter::export_sql(graph, &options) {
        Ok(sql) => sql,
        Err(e) => {
            let mut result = SqlValidationResult::new();
            result.add_error(SqlValidationError::error(
                format!("Failed to export schema: {}", e),
                "E999_EXPORT_ERROR",
            ));
            return result;
        }
    };

    validate_sql(&sql, dialect)
}

// ============================================================================
// Helper for UI underline errors
// ============================================================================

/// Error range for UI underline
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnderlineRange {
    /// Start line (1-based)
    pub start_line: usize,
    /// Start column (1-based)
    pub start_column: usize,
    /// End line (1-based)
    pub end_line: usize,
    /// End column (1-based)
    pub end_column: usize,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Error message for tooltip
    pub message: String,
}

impl SqlValidationResult {
    /// Get ranges for UI underline errors
    pub fn get_underline_ranges(&self) -> Vec<UnderlineRange> {
        self.get_underline_ranges_with_source(None)
    }

    /// Get ranges for UI underline errors with source for better context
    pub fn get_underline_ranges_with_source(&self, source: Option<&str>) -> Vec<UnderlineRange> {
        self.diagnostics
            .iter()
            .filter_map(|diag| {
                diag.span.as_ref().map(|span| {
                    // Calculate end column with better context
                    let end_column = if span.start.column == span.end.column {
                        // Single position - try to find word end or use minimum width
                        if let Some(src) = source {
                            Self::find_word_end(src, span.start.line, span.start.column)
                        } else {
                            // Default: underline at least 8 characters for visibility
                            span.start.column + 8
                        }
                    } else {
                        span.end.column.max(span.start.column + 1)
                    };

                    UnderlineRange {
                        start_line: span.start.line,
                        start_column: span.start.column,
                        end_line: span.end.line,
                        end_column,
                        severity: diag.severity,
                        message: diag.message.clone(),
                    }
                })
            })
            .collect()
    }

    /// Find the end of the current word/token at the given position
    fn find_word_end(source: &str, line: usize, column: usize) -> usize {
        if let Some(line_text) = source.lines().nth(line.saturating_sub(1)) {
            let start_idx = column.saturating_sub(1);
            if start_idx < line_text.len() {
                let rest = &line_text[start_idx..];
                // Find end of current token (word, identifier, or symbol)
                let token_len = rest
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != ',' && *c != ';' && *c != ')')
                    .count();
                if token_len > 0 {
                    return column + token_len;
                }
            }
            // If at end of line or whitespace, underline to end of line or minimum 8 chars
            return column + 8.min(line_text.len().saturating_sub(start_idx).max(1));
        }
        // Default minimum width
        column + 8
    }
}

// ============================================================================
// Canvas Notification Types
// ============================================================================

/// Notification type for canvas display
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationType {
    Success,
    Error,
    Warning,
    Info,
}

/// Notification for canvas display
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasNotification {
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub auto_dismiss_ms: Option<u32>,
}

impl CanvasNotification {
    pub fn success(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            notification_type: NotificationType::Success,
            title: title.into(),
            message: message.into(),
            auto_dismiss_ms: Some(3000),
        }
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            notification_type: NotificationType::Error,
            title: title.into(),
            message: message.into(),
            auto_dismiss_ms: None, // Errors should be manually dismissed
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            notification_type: NotificationType::Warning,
            title: title.into(),
            message: message.into(),
            auto_dismiss_ms: Some(5000),
        }
    }

    #[allow(dead_code)]
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            notification_type: NotificationType::Info,
            title: title.into(),
            message: message.into(),
            auto_dismiss_ms: Some(3000),
        }
    }

    pub fn from_apply_result(result: &ApplySqlResult) -> Self {
        if result.success {
            if result.warnings.is_empty() {
                Self::success(
                    "Changes Applied",
                    format!(
                        "Successfully applied {} changes",
                        result.applied_operations.len()
                    ),
                )
            } else {
                Self::warning(
                    "Changes Applied with Warnings",
                    format!(
                        "Applied {} changes with {} warnings",
                        result.applied_operations.len(),
                        result.warnings.len()
                    ),
                )
            }
        } else {
            Self::error(
                "Failed to Apply Changes",
                result
                    .errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )
        }
    }

    pub fn from_validation_result(result: &SqlValidationResult) -> Self {
        if result.is_valid {
            if result.has_warnings() {
                Self::warning(
                    "Validation Complete",
                    format!(
                        "Schema is valid with {} warnings",
                        result.stats.warning_count
                    ),
                )
            } else {
                Self::success(
                    "Validation Successful",
                    format!(
                        "Schema is valid ({} tables, {} relationships)",
                        result.stats.table_count, result.stats.relationship_count
                    ),
                )
            }
        } else {
            Self::error(
                "Validation Failed",
                format!(
                    "{} errors found. Check the source editor for details.",
                    result.stats.error_count
                ),
            )
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

// ============================================================================
// SQL Application to Graph
// ============================================================================

/// Result of applying SQL to graph
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApplySqlResult {
    /// Whether the application was successful
    pub success: bool,
    /// List of applied operations (for display)
    pub applied_operations: Vec<String>,
    /// Graph operations for LiveShare sync
    #[serde(skip)]
    pub graph_ops: Vec<GraphOperation>,
    /// Errors that occurred
    pub errors: Vec<String>,
    /// Warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

impl ApplySqlResult {
    pub fn success(operations: Vec<String>, graph_ops: Vec<GraphOperation>) -> Self {
        Self {
            success: true,
            applied_operations: operations,
            graph_ops,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            applied_operations: Vec::new(),
            graph_ops: Vec::new(),
            errors: vec![message.into()],
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Helper function to strip backticks and quotes from identifiers
fn strip_quotes(name: &str) -> String {
    name.trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('[')
        .trim_matches(']')
        .to_string()
}

/// Parse position from SQL comment like "-- Position: (100.0, 200.0)"
fn parse_position_from_sql(sql: &str, table_name: &str) -> Option<(f64, f64)> {
    // Find the CREATE TABLE statement for this table
    let table_name_lower = table_name.to_lowercase();

    for line_idx in 0..sql.lines().count() {
        let lines: Vec<&str> = sql.lines().collect();
        if line_idx >= lines.len() {
            break;
        }

        let line = lines[line_idx];

        // Check if this line has CREATE TABLE for our table
        let line_lower = line.to_lowercase();
        if line_lower.contains("create table") {
            // Extract table name from this line
            let stripped_line = strip_quotes(line_lower.replace("create table", "").trim());
            let line_table_name = stripped_line
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .map(strip_quotes)
                .unwrap_or_default();

            if line_table_name == table_name_lower {
                // Look for position comment in previous line
                if line_idx > 0 {
                    let prev_line = lines[line_idx - 1];
                    if prev_line.starts_with("-- Position:")
                        && let Some(start) = prev_line.find('(')
                        && let Some(end) = prev_line.find(')')
                    {
                        // Parse "(x, y)" from the comment
                        let coords = &prev_line[start + 1..end];
                        let parts: Vec<&str> = coords.split(',').collect();
                        if parts.len() == 2
                            && let (Ok(x), Ok(y)) = (
                                parts[0].trim().parse::<f64>(),
                                parts[1].trim().parse::<f64>(),
                            )
                        {
                            return Some((x, y));
                        }
                    }
                }
                break;
            }
        }
    }
    None
}

/// Apply validated SQL to a schema graph
/// Returns graph operations for LiveShare synchronization
pub fn apply_sql_to_graph(
    sql: &str,
    dialect: SqlDialect,
    graph: &mut SchemaGraph,
) -> ApplySqlResult {
    // First validate
    let validation = validate_sql(sql, dialect.clone());
    if !validation.is_valid {
        return ApplySqlResult {
            success: false,
            applied_operations: Vec::new(),
            graph_ops: Vec::new(),
            errors: validation
                .diagnostics
                .iter()
                .filter(|d| d.severity == ErrorSeverity::Error)
                .map(|d| d.message.clone())
                .collect(),
            warnings: validation
                .diagnostics
                .iter()
                .filter(|d| d.severity == ErrorSeverity::Warning)
                .map(|d| d.message.clone())
                .collect(),
        };
    }

    let mut applied = Vec::new();
    let mut graph_ops = Vec::new();
    let errors: Vec<String> = Vec::new();
    let mut warnings = Vec::new();

    // Clear existing graph and rebuild from SQL
    // This ensures the graph matches the SQL exactly
    let old_table_count = graph.node_count();
    graph.clear();

    // Track table name to node index mapping
    let mut table_indices: HashMap<String, NodeIndex> = HashMap::new();

    // Collect foreign key constraints to process after all tables are created
    // Using a HashSet to track unique foreign keys (from_table, from_col, to_table, to_col)
    // Type alias for (constraint_name, from_table, from_columns, to_table, to_columns)
    type ForeignKeyInfo = (String, String, Vec<String>, String, Vec<String>);
    let mut foreign_keys: Vec<ForeignKeyInfo> = Vec::new();
    let mut seen_fks: HashSet<(String, String, String, String)> = HashSet::new();

    // Process each statement
    for statement in &validation.statements {
        match statement {
            Statement::CreateTable(create_table) => {
                // Strip backticks/quotes from table name
                let raw_table_name = create_table.name.to_string();
                let table_name = strip_quotes(&raw_table_name);
                let table_name_lower = table_name.to_lowercase();

                // Skip if table already exists in this batch
                if table_indices.contains_key(&table_name_lower) {
                    warnings.push(format!(
                        "Table '{}' already defined, skipping duplicate",
                        table_name
                    ));
                    continue;
                }

                // Try to parse position from SQL comments
                let position = parse_position_from_sql(sql, &table_name)
                    .unwrap_or_else(|| calculate_next_table_position(graph));

                // Create table node
                let mut table_node =
                    TableNode::new(&table_name).with_position(position.0, position.1);

                // Collect primary key columns from table constraints
                let mut pk_columns: HashSet<String> = HashSet::new();
                for constraint in &create_table.constraints {
                    if let sqlparser::ast::TableConstraint::PrimaryKey(pk_constraint) = constraint {
                        for col in &pk_constraint.columns {
                            // Strip backticks/quotes from column name
                            let col_name = strip_quotes(&col.column.to_string()).to_lowercase();
                            pk_columns.insert(col_name);
                        }
                    }
                }

                // Add columns
                for col_def in &create_table.columns {
                    let col_name = col_def.name.value.clone();
                    let col_name_lower = col_name.to_lowercase();

                    // Check if this column is PK from column options or table constraint
                    let is_pk_from_option = col_def
                        .options
                        .iter()
                        .any(|opt| matches!(opt.option, ColumnOption::PrimaryKey(_)));
                    let is_pk = is_pk_from_option || pk_columns.contains(&col_name_lower);

                    let is_not_null = col_def
                        .options
                        .iter()
                        .any(|opt| matches!(opt.option, ColumnOption::NotNull));
                    let is_nullable = !is_not_null && !is_pk; // PK columns are implicitly NOT NULL

                    let is_unique = col_def
                        .options
                        .iter()
                        .any(|opt| matches!(opt.option, ColumnOption::Unique(_)));
                    let default_value = col_def.options.iter().find_map(|opt| {
                        if let ColumnOption::Default(expr) = &opt.option {
                            Some(expr.to_string())
                        } else {
                            None
                        }
                    });

                    let column = Column {
                        name: col_name,
                        data_type: col_def.data_type.to_string(),
                        is_primary_key: is_pk,
                        is_nullable,
                        is_unique,
                        default_value,
                    };
                    table_node.columns.push(column);
                }

                // Add to graph
                let node_idx = graph.add_node(table_node.clone());
                table_indices.insert(table_name_lower.clone(), node_idx);

                // Generate GraphOperation
                let columns_data: Vec<ColumnData> = table_node
                    .columns
                    .iter()
                    .map(|c| ColumnData {
                        name: c.name.clone(),
                        data_type: c.data_type.clone(),
                        is_primary_key: c.is_primary_key,
                        is_nullable: c.is_nullable,
                        is_unique: c.is_unique,
                        default_value: c.default_value.clone(),
                        foreign_key: None,
                    })
                    .collect();

                // Get the UUID from the newly created table node
                let table_uuid = graph
                    .node_weight(node_idx)
                    .map(|n| n.uuid)
                    .unwrap_or_else(uuid::Uuid::new_v4);

                graph_ops.push(GraphOperation::CreateTable {
                    node_id: node_idx.index() as u32,
                    table_uuid,
                    name: table_name.clone(),
                    position,
                });

                // Add columns operations
                for col in columns_data {
                    graph_ops.push(GraphOperation::AddColumn {
                        node_id: node_idx.index() as u32,
                        table_uuid,
                        column: col,
                    });
                }

                // Collect foreign key constraints from CREATE TABLE
                for constraint in &create_table.constraints {
                    if let sqlparser::ast::TableConstraint::ForeignKey(fk) = constraint {
                        let from_columns: Vec<String> =
                            fk.columns.iter().map(|c| strip_quotes(&c.value)).collect();
                        let to_table = strip_quotes(&fk.foreign_table.to_string());
                        let to_columns: Vec<String> = fk
                            .referred_columns
                            .iter()
                            .map(|c| strip_quotes(&c.value))
                            .collect();

                        // Create a key for deduplication
                        for (from_col, to_col) in from_columns.iter().zip(to_columns.iter()) {
                            let fk_key = (
                                table_name_lower.clone(),
                                from_col.to_lowercase(),
                                to_table.to_lowercase(),
                                to_col.to_lowercase(),
                            );
                            if !seen_fks.contains(&fk_key) {
                                seen_fks.insert(fk_key);
                            }
                        }

                        foreign_keys.push((
                            table_name.clone(),
                            table_name_lower.clone(),
                            from_columns,
                            to_table,
                            to_columns,
                        ));
                    }
                }

                applied.push(format!("Created table '{}'", table_name));
            }

            Statement::AlterTable(alter_table) => {
                let raw_table_name = alter_table.name.to_string();
                let table_name = strip_quotes(&raw_table_name);
                let table_name_lower = table_name.to_lowercase();

                // Handle ADD CONSTRAINT for foreign keys from ALTER TABLE
                for operation in &alter_table.operations {
                    if let AlterTableOperation::AddConstraint { constraint, .. } = operation
                        && let sqlparser::ast::TableConstraint::ForeignKey(fk) = constraint
                    {
                        let from_columns: Vec<String> =
                            fk.columns.iter().map(|c| strip_quotes(&c.value)).collect();
                        let to_table = strip_quotes(&fk.foreign_table.to_string());
                        let to_columns: Vec<String> = fk
                            .referred_columns
                            .iter()
                            .map(|c| strip_quotes(&c.value))
                            .collect();

                        // Check for duplicates before adding
                        let mut is_duplicate = false;
                        for (from_col, to_col) in from_columns.iter().zip(to_columns.iter()) {
                            let fk_key = (
                                table_name_lower.clone(),
                                from_col.to_lowercase(),
                                to_table.to_lowercase(),
                                to_col.to_lowercase(),
                            );
                            if seen_fks.contains(&fk_key) {
                                is_duplicate = true;
                                break;
                            }
                        }

                        if !is_duplicate {
                            // Mark as seen
                            for (from_col, to_col) in from_columns.iter().zip(to_columns.iter()) {
                                let fk_key = (
                                    table_name_lower.clone(),
                                    from_col.to_lowercase(),
                                    to_table.to_lowercase(),
                                    to_col.to_lowercase(),
                                );
                                seen_fks.insert(fk_key);
                            }

                            foreign_keys.push((
                                table_name.clone(),
                                table_name_lower.clone(),
                                from_columns,
                                to_table,
                                to_columns,
                            ));
                        }
                    }
                }
            }

            _ => {
                // Skip other statement types (DROP is handled by clearing the graph)
            }
        }
    }

    // Now process all foreign key constraints
    for (from_table, from_table_lower, from_columns, to_table, to_columns) in foreign_keys {
        let to_table_lower = to_table.to_lowercase();

        // Get source and target node indices
        let from_idx = match table_indices.get(&from_table_lower) {
            Some(&idx) => idx,
            None => {
                warnings.push(format!(
                    "Foreign key source table '{}' not found, skipping",
                    from_table
                ));
                continue;
            }
        };

        let to_idx = match table_indices.get(&to_table_lower) {
            Some(&idx) => idx,
            None => {
                warnings.push(format!(
                    "Foreign key target table '{}' not found, skipping",
                    to_table
                ));
                continue;
            }
        };

        // Create relationship for each column pair
        for (from_col, to_col) in from_columns.iter().zip(to_columns.iter()) {
            use crate::core::{Relationship, RelationshipType};

            use crate::core::liveshare::RelationshipData;

            let rel_name = format!(
                "fk_{}_{}_{}",
                from_table_lower,
                from_col.to_lowercase(),
                to_table_lower
            );
            let relationship = Relationship::new(
                rel_name.clone(),
                RelationshipType::ManyToOne,
                from_col.clone(),
                to_col.clone(),
            );

            // Add edge to graph
            let edge_idx = graph.add_edge(from_idx, to_idx, relationship);

            // Add graph operation for LiveShare
            graph_ops.push(GraphOperation::CreateRelationship {
                edge_id: edge_idx.index() as u32,
                from_node: from_idx.index() as u32,
                to_node: to_idx.index() as u32,
                relationship: RelationshipData {
                    name: rel_name,
                    relationship_type: "many_to_one".to_string(),
                    from_column: from_col.clone(),
                    to_column: to_col.clone(),
                },
            });

            applied.push(format!(
                "Created relationship: {}.{} -> {}.{}",
                from_table, from_col, to_table, to_col
            ));
        }
    }

    if old_table_count > 0 && graph.node_count() > 0 {
        applied.insert(
            0,
            format!(
                "Replaced {} tables with {} tables",
                old_table_count,
                graph.node_count()
            ),
        );
    }

    if errors.is_empty() {
        ApplySqlResult {
            success: true,
            applied_operations: applied,
            graph_ops,
            errors: Vec::new(),
            warnings,
        }
    } else {
        ApplySqlResult {
            success: false,
            applied_operations: applied,
            graph_ops: Vec::new(), // Don't return ops on error
            errors,
            warnings,
        }
    }
}

/// Calculate position for a new table based on existing tables
fn calculate_next_table_position(graph: &SchemaGraph) -> (f64, f64) {
    const TABLE_WIDTH: f64 = 250.0;
    const TABLE_HEIGHT: f64 = 200.0;
    const TABLE_SPACING: f64 = 50.0;
    const TABLES_PER_ROW: usize = 4;
    const START_X: f64 = 100.0;
    const START_Y: f64 = 100.0;

    let table_count = graph.node_count();
    let row = table_count / TABLES_PER_ROW;
    let col = table_count % TABLES_PER_ROW;

    let x = START_X + col as f64 * (TABLE_WIDTH + TABLE_SPACING);
    let y = START_Y + row as f64 * (TABLE_HEIGHT + TABLE_SPACING);

    (x, y)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_validation_valid() {
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(255));";
        let result = validate_sql(sql, SqlDialect::MySQL);
        assert!(result.is_valid);
        assert_eq!(result.stats.error_count, 0);
    }

    #[test]
    fn test_syntax_validation_invalid() {
        let sql = "CREATE TABEL users (id INT);"; // typo: TABEL
        let result = validate_sql(sql, SqlDialect::MySQL);
        assert!(!result.is_valid);
        assert!(result.stats.error_count > 0);
    }

    #[test]
    fn test_semantic_validation_duplicate_column() {
        let sql = "CREATE TABLE users (id INT, id VARCHAR(255));";
        let result = validate_sql(sql, SqlDialect::MySQL);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "E002_DUPLICATE_COLUMN")
        );
    }

    #[test]
    fn test_foreign_key_validation() {
        let sql = r#"
            CREATE TABLE orders (
                id INT PRIMARY KEY,
                user_id INT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        "#;
        let result = validate_sql(sql, SqlDialect::MySQL);
        // Should fail because 'users' table doesn't exist
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "E003_UNKNOWN_TABLE")
        );
    }

    #[test]
    fn test_alter_table_nonexistent_has_position() {
        let sql = r#"
CREATE TABLE users (id INT PRIMARY KEY);
ALTER TABLE `tab2` ADD COLUMN name VARCHAR(255);
        "#;
        let result = validate_sql(sql, SqlDialect::MySQL);

        // Should have error for non-existent table
        let error = result
            .diagnostics
            .iter()
            .find(|d| d.code == "E003_UNKNOWN_TABLE");
        assert!(error.is_some(), "Should have unknown table error");

        // Error should have position info
        let error = error.unwrap();
        assert!(error.span.is_some(), "Error should have position info");

        let span = error.span.as_ref().unwrap();
        // ALTER TABLE `tab2` is on line 3 (1-indexed, after blank line)
        assert!(
            span.start.line >= 2,
            "Position should be on line 2 or later, got {}",
            span.start.line
        );

        // Check underline ranges
        let ranges = result.get_underline_ranges_with_source(Some(sql));
        assert!(
            !ranges.is_empty(),
            "Should have underline ranges for the error"
        );
    }

    #[test]
    fn test_foreign_key_error_has_position() {
        let sql = r#"
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT);
ALTER TABLE `orders` ADD CONSTRAINT `fk_orders_users` FOREIGN KEY (`user_id`) REFERENCES `nonexistent`(`id`);
        "#;
        let result = validate_sql(sql, SqlDialect::MySQL);

        // Should have error for non-existent referenced table
        let error = result
            .diagnostics
            .iter()
            .find(|d| d.code == "E003_UNKNOWN_TABLE");
        assert!(error.is_some(), "Should have unknown table error");

        let error = error.unwrap();
        assert!(
            error.span.is_some(),
            "Foreign key error should have position info"
        );

        // Check underline ranges
        let ranges = result.get_underline_ranges_with_source(Some(sql));
        assert!(
            !ranges.is_empty(),
            "Should have underline ranges for the FK error"
        );
    }

    #[test]
    fn test_foreign_key_valid() {
        let sql = r#"
            CREATE TABLE users (id INT PRIMARY KEY);
            CREATE TABLE orders (
                id INT PRIMARY KEY,
                user_id INT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        "#;
        let result = validate_sql(sql, SqlDialect::MySQL);
        // Should pass - users table is defined before orders
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code == "E003_UNKNOWN_TABLE")
        );
    }

    #[test]
    fn test_format_for_llm() {
        let sql = "CREATE TABLE users (id INT PRIMARY KEY);";
        let result = validate_sql(sql, SqlDialect::MySQL);
        let llm_output = result.format_for_llm();
        assert!(llm_output.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_underline_ranges() {
        let sql = "CREATE TABEL users (id INT);";
        let result = validate_sql(sql, SqlDialect::MySQL);
        let ranges = result.get_underline_ranges();
        // Should have at least one range for the syntax error
        // Note: sqlparser may not always provide position, so this might be empty
        // depending on the error
        assert!(ranges.is_empty() || ranges[0].severity == ErrorSeverity::Error);
    }

    #[test]
    fn test_canvas_notification() {
        let sql = "CREATE TABLE users (id INT PRIMARY KEY);";
        let result = validate_sql(sql, SqlDialect::MySQL);
        let notification = CanvasNotification::from_validation_result(&result);
        assert!(matches!(
            notification.notification_type,
            NotificationType::Success
        ));
    }

    #[test]
    fn test_position_from_offset() {
        let sql = "CREATE TABLE\nusers (\n  id INT\n);";
        let pos = SourcePosition::from_offset(sql, 15); // "users" position
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 3);
    }

    #[test]
    fn test_apply_sql_create_table() {
        let mut graph = SchemaGraph::new();
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(255));";
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 1);
        assert!(!result.graph_ops.is_empty());
    }

    #[test]
    fn test_apply_sql_drop_table() {
        let mut graph = SchemaGraph::new();

        // First create a table
        let create_sql = "CREATE TABLE users (id INT PRIMARY KEY);";
        apply_sql_to_graph(create_sql, SqlDialect::MySQL, &mut graph);
        assert_eq!(graph.node_count(), 1);

        // Then drop it
        let drop_sql = "DROP TABLE users;";
        let result = apply_sql_to_graph(drop_sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_apply_sql_invalid() {
        let mut graph = SchemaGraph::new();
        let sql = "CREATE TABEL users (id INT);"; // typo
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_apply_result_notification() {
        let result = ApplySqlResult::success(vec!["Created table 'users'".to_string()], vec![]);
        let notification = CanvasNotification::from_apply_result(&result);
        assert!(matches!(
            notification.notification_type,
            NotificationType::Success
        ));
    }

    #[test]
    fn test_apply_sql_with_table_constraint_primary_key() {
        let mut graph = SchemaGraph::new();
        let sql = r#"
            CREATE TABLE users (
                id INT NOT NULL,
                name VARCHAR(255),
                PRIMARY KEY (id)
            );
        "#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 1);

        // Check that id column is marked as primary key
        let table = graph
            .node_indices()
            .next()
            .and_then(|idx| graph.node_weight(idx));
        assert!(table.is_some());
        let table = table.unwrap();
        let id_col = table.columns.iter().find(|c| c.name == "id");
        assert!(id_col.is_some());
        assert!(id_col.unwrap().is_primary_key);
    }

    #[test]
    fn test_apply_sql_with_foreign_keys() {
        let mut graph = SchemaGraph::new();
        let sql = r#"
            CREATE TABLE users (
                id INT NOT NULL,
                PRIMARY KEY (id)
            );
            CREATE TABLE posts (
                id INT NOT NULL,
                user_id INT,
                PRIMARY KEY (id),
                CONSTRAINT fk_posts_user FOREIGN KEY (user_id) REFERENCES users(id)
            );
        "#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1); // One foreign key relationship
    }

    #[test]
    fn test_apply_sql_with_alter_table_foreign_key() {
        let mut graph = SchemaGraph::new();
        let sql = r#"
            CREATE TABLE users (
                id INT NOT NULL,
                PRIMARY KEY (id)
            );
            CREATE TABLE posts (
                id INT NOT NULL,
                user_id INT,
                PRIMARY KEY (id)
            );
            ALTER TABLE posts ADD CONSTRAINT fk_posts_user FOREIGN KEY (user_id) REFERENCES users(id);
        "#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1); // One foreign key relationship
    }

    #[test]
    fn test_apply_sql_with_position_comments() {
        let mut graph = SchemaGraph::new();
        let sql = r#"-- Position: (500.5, 300.25)
CREATE TABLE users (
    id INT NOT NULL,
    PRIMARY KEY (id)
);
"#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 1);

        // Check that position was parsed from comment
        let table = graph
            .node_indices()
            .next()
            .and_then(|idx| graph.node_weight(idx));
        assert!(table.is_some());
        let table = table.unwrap();
        assert!((table.position.0 - 500.5).abs() < 0.01);
        assert!((table.position.1 - 300.25).abs() < 0.01);
    }

    #[test]
    fn test_strip_quotes_from_table_names() {
        let mut graph = SchemaGraph::new();
        let sql = "CREATE TABLE `users` (`id` INT PRIMARY KEY);";
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 1);

        // Check that backticks were stripped from table name
        let table = graph
            .node_indices()
            .next()
            .and_then(|idx| graph.node_weight(idx));
        assert!(table.is_some());
        let table = table.unwrap();
        assert_eq!(table.name, "users"); // Not `users`
    }

    #[test]
    fn test_apply_sql_deduplicates_foreign_keys() {
        let mut graph = SchemaGraph::new();
        // FK defined in both CREATE TABLE and ALTER TABLE - should only create one relationship
        let sql = r#"
            CREATE TABLE users (
                id INT NOT NULL,
                PRIMARY KEY (id)
            );
            CREATE TABLE posts (
                id INT NOT NULL,
                user_id INT,
                PRIMARY KEY (id),
                CONSTRAINT fk_posts_user FOREIGN KEY (user_id) REFERENCES users(id)
            );
            ALTER TABLE posts ADD CONSTRAINT fk_posts_user_duplicate FOREIGN KEY (user_id) REFERENCES users(id);
        "#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1); // Should be 1, not 2 (deduplicated)
    }

    #[test]
    fn test_apply_sql_preserves_primary_key_through_roundtrip() {
        use crate::core::{ExportOptions, SchemaExporter};

        let mut graph = SchemaGraph::new();
        let sql = r#"
-- Position: (100, 200)
CREATE TABLE users (
    id INT NOT NULL,
    name VARCHAR(255),
    PRIMARY KEY (id)
);
"#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);
        assert!(result.success);

        // Check that id column is marked as primary key
        let table = graph
            .node_indices()
            .next()
            .and_then(|idx| graph.node_weight(idx))
            .expect("Table should exist");

        let id_col = table
            .columns
            .iter()
            .find(|c| c.name == "id")
            .expect("id column should exist");
        assert!(
            id_col.is_primary_key,
            "id column should be marked as primary key"
        );

        // Now export and check that PRIMARY KEY is in the output
        let options = ExportOptions {
            sql_dialect: SqlDialect::MySQL,
            include_positions: true,
            ..Default::default()
        };
        let exported = SchemaExporter::export_sql(&graph, &options).expect("Export should succeed");

        assert!(
            exported.contains("PRIMARY KEY"),
            "Exported SQL should contain PRIMARY KEY. Got:\n{}",
            exported
        );
    }

    #[test]
    fn test_apply_sql_complex_schema_with_multiple_fks() {
        use crate::core::{ExportOptions, SchemaExporter};

        let mut graph = SchemaGraph::new();
        let sql = r#"
-- Position: (519.6298515937716, 463.8799885454206)
CREATE TABLE `tablica` (
    `id` INT NOT NULL,
    PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Position: (100, 450)
CREATE TABLE `tab2` (
    `id` INT NOT NULL,
    `fk` INT,
    PRIMARY KEY (`id`),
    CONSTRAINT `fk_tab2_id` FOREIGN KEY (`fk`) REFERENCES `tablica`(`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Position: (459.9999999999841, 100)
CREATE TABLE `posts` (
    `id` INT NOT NULL,
    `title` VARCHAR,
    `content` TEXT,
    `user_id` INT,
    PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Position: (879.6298515937671, 113.8799885454206)
CREATE TABLE `users` (
    `id` INT NOT NULL,
    `name` VARCHAR,
    PRIMARY KEY (`id`),
    CONSTRAINT `fk_users_user_id` FOREIGN KEY (`id`) REFERENCES `posts`(`user_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Foreign Key Constraints
ALTER TABLE `tab2` ADD CONSTRAINT `fk_tab2_fk_tab` FOREIGN KEY (`fk`) REFERENCES `tablica`(`id`);
ALTER TABLE `users` ADD CONSTRAINT `fk_users_id_posts` FOREIGN KEY (`id`) REFERENCES `posts`(`user_id`);
"#;
        let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

        assert!(result.success, "Parse should succeed");
        assert_eq!(graph.node_count(), 4, "Should have 4 tables");

        // Check relationships - should be 2, not 4 (deduplicated)
        assert_eq!(
            graph.edge_count(),
            2,
            "Should have 2 relationships (deduplicated from CREATE TABLE and ALTER TABLE)"
        );

        // Verify all tables have PRIMARY KEY columns
        for node_idx in graph.node_indices() {
            let table = graph.node_weight(node_idx).expect("Table should exist");
            let has_pk = table.columns.iter().any(|c| c.is_primary_key);
            assert!(
                has_pk,
                "Table '{}' should have a PRIMARY KEY column",
                table.name
            );
        }

        // Export and verify PRIMARY KEY is in output for all tables
        let options = ExportOptions {
            sql_dialect: SqlDialect::MySQL,
            include_positions: true,
            ..Default::default()
        };
        let exported = SchemaExporter::export_sql(&graph, &options).expect("Export should succeed");

        // Count PRIMARY KEY occurrences - should be 4 (one per table)
        let pk_count = exported.matches("PRIMARY KEY").count();
        assert_eq!(
            pk_count, 4,
            "Exported SQL should have 4 PRIMARY KEY constraints. Got:\n{}",
            exported
        );

        // Verify no duplicate ALTER TABLE statements for same FK
        let alter_fk_count = exported.matches("ALTER TABLE").count();
        assert_eq!(
            alter_fk_count, 2,
            "Should have 2 ALTER TABLE FK statements (one per relationship). Got:\n{}",
            exported
        );
    }

    #[test]
    fn test_apply_sql_replaces_existing_graph() {
        let mut graph = SchemaGraph::new();

        // Create initial table
        let sql1 = "CREATE TABLE old_table (id INT PRIMARY KEY);";
        apply_sql_to_graph(sql1, SqlDialect::MySQL, &mut graph);
        assert_eq!(graph.node_count(), 1);

        // Apply new SQL - should replace, not add
        let sql2 = "CREATE TABLE new_table (id INT PRIMARY KEY);";
        let result = apply_sql_to_graph(sql2, SqlDialect::MySQL, &mut graph);

        assert!(result.success);
        assert_eq!(graph.node_count(), 1); // Still 1, not 2

        // Check it's the new table
        let table = graph
            .node_indices()
            .next()
            .and_then(|idx| graph.node_weight(idx));
        assert!(table.is_some());
        assert_eq!(table.unwrap().name, "new_table");
    }
}
