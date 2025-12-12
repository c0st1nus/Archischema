# SQL Parser Integration

This document describes the SQL parser integration in Archischema, which provides syntax and semantic validation for SQL DDL statements, with LiveShare synchronization support.

## Overview

The SQL parser module (`src/core/sql_parser.rs`) provides:

1. **Syntax Validation** - Using `sqlparser-rs` to parse SQL and report syntax errors with line/column positions
2. **Semantic Validation** - Custom validation for:
   - Duplicate column names
   - Foreign key references to non-existent tables/columns
   - Invalid data types
   - ALTER TABLE operations on non-existent columns
3. **Error Underlines** - Position information for UI error highlighting
4. **LLM Agent Output** - Structured JSON output for AI agents
5. **Canvas Notifications** - Toast-style notifications for validation results
6. **Apply SQL to Graph** - Parse and apply SQL changes with LiveShare sync
7. **Save Workflow** - Validate before applying changes to ensure data integrity

## Usage

### Basic Validation

```rust
use archischema::core::{SqlDialect, validate_sql};

let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(255));";
let result = validate_sql(sql, SqlDialect::MySQL);

if result.is_valid {
    println!("SQL is valid!");
} else {
    for error in &result.diagnostics {
        println!("{}", error);
    }
}
```

### Validation with Existing Schema

```rust
use archischema::core::{validate_sql_with_graph, SqlDialect, SchemaGraph};

let graph: SchemaGraph = /* existing schema */;
let sql = "ALTER TABLE users ADD COLUMN email VARCHAR(255);";
let result = validate_sql_with_graph(sql, SqlDialect::MySQL, &graph);
```

### Schema Check (for saving)

```rust
use archischema::core::{check_schema_sql, SqlDialect, SchemaGraph};

let graph: SchemaGraph = /* your schema */;
let result = check_schema_sql(&graph, SqlDialect::MySQL);

if !result.is_valid {
    // Don't save - show errors to user
}
```

### LLM Agent Integration

The validation result can be formatted for LLM consumption:

```rust
let result = validate_sql(sql, SqlDialect::MySQL);
let llm_output = result.format_for_llm();
// Returns JSON with success, error_count, diagnostics, etc.
```

Example LLM output:

```json
{
  "success": false,
  "error_count": 1,
  "warning_count": 0,
  "diagnostics": [
    {
      "severity": "error",
      "code": "E003_UNKNOWN_TABLE",
      "message": "Foreign key references non-existent table 'users'",
      "position": { "line": 4, "column": 5 },
      "suggestion": "Create table 'users' before referencing it"
    }
  ],
  "summary": "SQL has 1 errors and 0 warnings."
}
```

### AI Tools Integration

Two new tools are available for AI agents:

1. **validate_sql** - Validate SQL before applying
   ```json
   {
     "tool_name": "validate_sql",
     "parameters": {
       "sql": "CREATE TABLE users (id INT PRIMARY KEY);",
       "dialect": "mysql"
     }
   }
   ```

2. **check_schema** - Validate current schema
   ```json
   {
     "tool_name": "check_schema",
     "parameters": {}
   }
   ```

## Error Codes

| Code | Description |
|------|-------------|
| E001_SYNTAX_ERROR | SQL syntax error |
| E002_DUPLICATE_COLUMN | Duplicate column name in table |
| E003_UNKNOWN_TABLE | Reference to non-existent table |
| E004_UNKNOWN_COLUMN | Reference to non-existent column |
| E005_FK_COLUMN_MISMATCH | Foreign key column count mismatch |
| E006_INVALID_DECIMAL | Invalid DECIMAL precision/scale |
| E999_EXPORT_ERROR | Schema export failed |
| W001_ZERO_LENGTH_VARCHAR | VARCHAR(0) warning |
| W002_DROP_NONEXISTENT | Dropping non-existent table |

## UI Integration

### Source Editor

The `SourceEditor` component now includes:

- **Save Button** - Click to validate and apply changes (appears when modified)
- **Reset Button** - Discard local changes and revert to graph state
- **Diagnostics Panel** - Shows errors/warnings with suggestions
- **Line Number Indicators** - Red/yellow dots for error lines
- **Error Underlines** - Visual underlines at error positions

### Save Workflow

When the user clicks "Save":

1. **Validation** - SQL is validated for syntax and semantic errors
2. **If Invalid** - Errors are displayed, changes are NOT applied
3. **If Valid** - SQL is parsed and applied to the schema graph
4. **LiveShare Sync** - Graph operations are sent to all connected users
5. **UI Update** - Modified flag is cleared, graph updates propagate to Visual mode

```rust
use archischema::ui::{SourceEditor, NotificationManager};

let notification_manager = NotificationManager::new();

view! {
    <SourceEditor
        graph=graph_signal
        on_notification=notification_manager.callback()
        on_validation=move |result| {
            // Handle validation result for LLM
        }
    />
}
```

### Applying SQL Programmatically

```rust
use archischema::core::{apply_sql_to_graph, SqlDialect};

let sql = r#"
    CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(255));
    ALTER TABLE users ADD COLUMN email VARCHAR(255);
"#;

let result = apply_sql_to_graph(sql, SqlDialect::MySQL, &mut graph);

if result.success {
    // Send operations through LiveShare
    for op in &result.graph_ops {
        liveshare_ctx.send_graph_op(op.clone());
    }
    println!("Applied: {:?}", result.applied_operations);
} else {
    println!("Errors: {:?}", result.errors);
}
```

### Canvas Notifications

```rust
use archischema::ui::{NotificationsContainer, NotificationManager};

let manager = NotificationManager::new();

view! {
    <div>
        <NotificationsContainer notifications=manager.notifications() />
        // ... rest of canvas
    </div>
}

// Show notifications
manager.success("Saved", "Schema saved successfully");
manager.error("Error", "Validation failed");
manager.warning("Warning", "Some issues found");
```

## Supported SQL Dialects

- MySQL (default)
- PostgreSQL
- SQLite

```rust
let result = validate_sql(sql, SqlDialect::PostgreSQL);
```

## Architecture

```
sql_parser.rs
├── SqlParser           - Syntax parsing with sqlparser-rs
├── SchemaValidator     - Semantic validation logic
├── SqlValidationResult - Validation result with diagnostics
├── SqlValidationError  - Error with position and suggestions
├── ApplySqlResult      - Result of applying SQL to graph
├── apply_sql_to_graph  - Apply validated SQL with GraphOperation generation
├── CanvasNotification  - UI notification type
└── UnderlineRange      - Error range for UI highlighting
```

## LiveShare Synchronization

When SQL is applied to the graph, the following `GraphOperation` types are generated:

- `CreateTable` - When a new table is created via CREATE TABLE
- `DeleteTable` - When a table is dropped via DROP TABLE
- `RenameTable` - When a table is renamed via ALTER TABLE RENAME
- `AddColumn` - When a column is added via ALTER TABLE ADD COLUMN
- `UpdateColumn` - When a column is modified (e.g., renamed)
- `DeleteColumn` - When a column is dropped via ALTER TABLE DROP COLUMN

These operations are sent through the LiveShare WebSocket connection to synchronize all connected users in real-time.

## Future Improvements

- [ ] Circular reference detection in foreign keys
- [ ] Type compatibility checking between FK and PK columns
- [ ] More precise source positions using sqlparser spans
- [ ] Auto-fix suggestions for common errors
- [ ] Real-time validation (debounced)
- [ ] Support for CREATE INDEX statements
- [ ] Support for FOREIGN KEY constraint application