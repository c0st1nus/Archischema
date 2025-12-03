# API Examples: Table Operations

This document provides practical examples of using the `TableOps` trait in Diagramix.

## Table of Contents

- [Basic Operations](#basic-operations)
- [Advanced Usage](#advanced-usage)
- [UI Integration](#ui-integration)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)

---

## Basic Operations

### Creating a Table with Specific Name

```rust
use crate::core::{SchemaGraph, TableOps};
use petgraph::stable_graph::StableGraph;

let mut graph: SchemaGraph = StableGraph::new();

// Create a table at position (100, 200)
match graph.create_table("users", (100.0, 200.0)) {
    Ok(node_idx) => {
        println!("Created table 'users' with index: {:?}", node_idx);
        // Use node_idx to add columns, create relationships, etc.
    }
    Err(error) => {
        eprintln!("Failed to create table: {}", error);
    }
}
```

### Creating a Table with Auto-Generated Name

```rust
// Creates table with name "new_table", "new_table_2", etc.
let node_idx = graph.create_table_auto((300.0, 150.0));

// Get the generated name
if let Some(table) = graph.node_weight(node_idx) {
    println!("Created table: {}", table.name);
}
```

### Renaming a Table

```rust
use petgraph::graph::NodeIndex;

let node_idx: NodeIndex = /* ... */;

match graph.rename_table(node_idx, "customers") {
    Ok(()) => {
        println!("Table renamed successfully!");
    }
    Err(error) => {
        eprintln!("Rename failed: {}", error);
    }
}
```

### Deleting a Table

```rust
match graph.delete_table(node_idx) {
    Ok(deleted_table) => {
        println!("Deleted table: {}", deleted_table.name);
        println!("It had {} columns", deleted_table.columns.len());
    }
    Err(error) => {
        eprintln!("Delete failed: {}", error);
    }
}
```

### Checking if Table Exists

```rust
if graph.table_exists("users") {
    println!("Table 'users' exists");
} else {
    println!("Table 'users' does not exist");
}
```

### Finding a Table by Name

```rust
match graph.find_table_by_name("products") {
    Some(node_idx) => {
        println!("Found table at index: {:?}", node_idx);
        // Access the table
        if let Some(table) = graph.node_weight(node_idx) {
            println!("Position: {:?}", table.position);
        }
    }
    None => {
        println!("Table 'products' not found");
    }
}
```

### Generating Unique Names

```rust
// Get a unique name based on "table"
let unique_name = graph.generate_unique_table_name("table");
println!("Unique name: {}", unique_name);

// Create the table with that name
let result = graph.create_table(&unique_name, (0.0, 0.0));
```

---

## Advanced Usage

### Creating Multiple Tables

```rust
let tables = vec![
    ("users", (100.0, 100.0)),
    ("posts", (400.0, 100.0)),
    ("comments", (400.0, 400.0)),
];

let mut table_indices = Vec::new();

for (name, position) in tables {
    match graph.create_table(name, position) {
        Ok(idx) => {
            table_indices.push(idx);
            println!("✓ Created {}", name);
        }
        Err(e) => {
            eprintln!("✗ Failed to create {}: {}", name, e);
        }
    }
}
```

### Batch Creation with Auto-Naming

```rust
let positions = vec![(100.0, 100.0), (300.0, 100.0), (500.0, 100.0)];

let table_indices: Vec<NodeIndex> = positions
    .into_iter()
    .map(|pos| graph.create_table_auto(pos))
    .collect();

println!("Created {} tables", table_indices.len());
```

### Creating Table with Columns

```rust
use crate::core::Column;

// Create table
let users_idx = graph.create_table("users", (100.0, 100.0))?;

// Add columns
if let Some(table) = graph.node_weight_mut(users_idx) {
    table.create_column(Column::new("id", "INTEGER").primary_key());
    table.create_column(Column::new("username", "VARCHAR(255)").not_null().unique());
    table.create_column(Column::new("email", "VARCHAR(255)").not_null());
    table.create_column(Column::new("created_at", "TIMESTAMP"));
}
```

### Safe Rename with Fallback

```rust
fn safe_rename_table(
    graph: &mut SchemaGraph,
    node_idx: NodeIndex,
    new_name: &str,
) -> String {
    match graph.rename_table(node_idx, new_name) {
        Ok(()) => new_name.to_string(),
        Err(_) => {
            // If rename fails, generate a unique name
            let unique_name = graph.generate_unique_table_name(new_name);
            graph.rename_table(node_idx, &unique_name).unwrap();
            unique_name
        }
    }
}

// Usage
let final_name = safe_rename_table(&mut graph, node_idx, "users");
println!("Table renamed to: {}", final_name);
```

### Conditional Table Creation

```rust
fn get_or_create_table(
    graph: &mut SchemaGraph,
    name: &str,
    position: (f64, f64),
) -> NodeIndex {
    match graph.find_table_by_name(name) {
        Some(idx) => {
            println!("Table '{}' already exists", name);
            idx
        }
        None => {
            println!("Creating new table '{}'", name);
            graph.create_table(name, position).unwrap()
        }
    }
}

// Usage
let users_idx = get_or_create_table(&mut graph, "users", (100.0, 100.0));
```

---

## UI Integration

### Leptos Component Example

```rust
use leptos::prelude::*;
use crate::core::{SchemaGraph, TableOps};

#[component]
pub fn CreateTableButton(graph: RwSignal<SchemaGraph>) -> impl IntoView {
    let (error, set_error) = signal::<Option<String>>(None);
    let (table_name, set_table_name) = signal(String::new());

    let create_table = move || {
        let name = table_name.get();
        
        // Validate
        if name.trim().is_empty() {
            set_error.set(Some("Name cannot be empty".to_string()));
            return;
        }

        // Create table
        match graph.write().create_table(&name, (300.0, 300.0)) {
            Ok(node_idx) => {
                set_error.set(None);
                set_table_name.set(String::new());
                leptos::logging::log!("Created table at {:?}", node_idx);
            }
            Err(e) => {
                set_error.set(Some(e));
            }
        }
    };

    view! {
        <div>
            <input
                type="text"
                placeholder="Table name"
                prop:value=move || table_name.get()
                on:input=move |ev| set_table_name.set(event_target_value(&ev))
            />
            <button on:click=move |_| create_table()>
                "Create Table"
            </button>
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
        </div>
    }
}
```

### Quick Create with Auto-Name

```rust
#[component]
pub fn QuickCreateButton(graph: RwSignal<SchemaGraph>) -> impl IntoView {
    let create_table = move || {
        let node_idx = graph.write().create_table_auto((400.0, 300.0));
        leptos::logging::log!("Created table with auto-generated name");
    };

    view! {
        <button 
            on:click=move |_| create_table()
            class="fab-button"
        >
            "+"
        </button>
    }
}
```

### Table List with Delete

```rust
#[component]
pub fn TableList(graph: RwSignal<SchemaGraph>) -> impl IntoView {
    let delete_table = move |node_idx: NodeIndex| {
        graph.update(|g| {
            if let Ok(deleted) = g.delete_table(node_idx) {
                leptos::logging::log!("Deleted table: {}", deleted.name);
            }
        });
    };

    view! {
        <ul>
            {move || {
                graph.with(|g| {
                    g.node_indices()
                        .map(|idx| {
                            let table = g.node_weight(idx).unwrap();
                            let name = table.name.clone();
                            view! {
                                <li>
                                    <span>{name}</span>
                                    <button on:click=move |_| delete_table(idx)>
                                        "Delete"
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()
                })
            }}
        </ul>
    }
}
```

---

## Error Handling

### Comprehensive Error Handling

```rust
use crate::core::{SchemaGraph, TableOps};

fn create_table_safely(
    graph: &mut SchemaGraph,
    name: &str,
    position: (f64, f64),
) -> Result<NodeIndex, String> {
    // Pre-validation
    if name.trim().is_empty() {
        return Err("Table name is required".to_string());
    }

    if name.len() > 64 {
        return Err("Table name too long (max 64 characters)".to_string());
    }

    // Check for special characters (optional)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Table name can only contain letters, numbers, and underscores".to_string());
    }

    // Create table
    graph.create_table(name, position)
}

// Usage
match create_table_safely(&mut graph, "my-table", (0.0, 0.0)) {
    Ok(idx) => println!("Success!"),
    Err(e) => eprintln!("Error: {}", e),
}
```

### User-Friendly Error Messages

```rust
fn get_friendly_error_message(error: &str) -> &str {
    if error.contains("already exists") {
        "A table with this name already exists. Please choose a different name."
    } else if error.contains("cannot be empty") {
        "Please enter a table name."
    } else {
        "An unexpected error occurred. Please try again."
    }
}

// Usage in UI
match graph.write().create_table(&name, pos) {
    Ok(_) => { /* success */ }
    Err(e) => {
        let friendly_msg = get_friendly_error_message(&e);
        show_error_to_user(friendly_msg);
    }
}
```

---

## Best Practices

### 1. Always Validate Before Creating

```rust
// ❌ Bad
let idx = graph.create_table(&user_input, pos).unwrap();

// ✅ Good
if let Ok(idx) = graph.create_table(&user_input, pos) {
    // Handle success
} else {
    // Handle error
}
```

### 2. Use Auto-Naming for Temporary Tables

```rust
// ✅ Good for quick prototyping or temporary tables
let temp_idx = graph.create_table_auto((0.0, 0.0));

// Rename later when ready
graph.rename_table(temp_idx, "final_name")?;
```

### 3. Check Existence Before Operations

```rust
// ✅ Good
if !graph.table_exists("users") {
    graph.create_table("users", (100.0, 100.0))?;
}

// Or use find_table_by_name for more control
if graph.find_table_by_name("users").is_none() {
    graph.create_table("users", (100.0, 100.0))?;
}
```

### 4. Batch Operations for Performance

```rust
// ✅ Good - Single write lock
graph.update(|g| {
    g.create_table("users", (100.0, 100.0))?;
    g.create_table("posts", (300.0, 100.0))?;
    g.create_table("comments", (500.0, 100.0))?;
    Ok::<(), String>(())
});

// ❌ Less efficient - Multiple write locks
graph.write().create_table("users", (100.0, 100.0))?;
graph.write().create_table("posts", (300.0, 100.0))?;
graph.write().create_table("comments", (500.0, 100.0))?;
```

### 5. Handle Cascading Deletes

```rust
// ✅ Good - Aware that relationships are deleted
let deleted_table = graph.delete_table(node_idx)?;
println!("Deleted {} and its {} relationships", 
    deleted_table.name,
    // Count relationships before deletion if needed
);
```

### 6. Use Type Safety

```rust
use petgraph::graph::NodeIndex;

// ✅ Good - Type-safe
fn process_table(graph: &SchemaGraph, idx: NodeIndex) {
    if let Some(table) = graph.node_weight(idx) {
        println!("Processing: {}", table.name);
    }
}

// ❌ Avoid - No type safety
fn process_table_bad(graph: &SchemaGraph, idx: usize) {
    // Less safe, requires conversion
}
```

---

## Common Patterns

### Pattern 1: Create, Validate, Commit

```rust
// Create with auto-name
let temp_idx = graph.create_table_auto((300.0, 300.0));

// Let user edit
let user_name = get_user_input();

// Validate and commit
match graph.rename_table(temp_idx, &user_name) {
    Ok(()) => { /* committed */ }
    Err(_) => {
        // Rollback
        graph.delete_table(temp_idx)?;
    }
}
```

### Pattern 2: Factory Method

```rust
fn create_standard_table(
    graph: &mut SchemaGraph,
    name: &str,
    position: (f64, f64),
) -> Result<NodeIndex, String> {
    let idx = graph.create_table(name, position)?;
    
    // Add standard columns
    if let Some(table) = graph.node_weight_mut(idx) {
        table.create_column(Column::new("id", "INTEGER").primary_key());
        table.create_column(Column::new("created_at", "TIMESTAMP"));
        table.create_column(Column::new("updated_at", "TIMESTAMP"));
    }
    
    Ok(idx)
}
```

### Pattern 3: Template System

```rust
enum TableTemplate {
    Users,
    Posts,
    Products,
}

fn create_from_template(
    graph: &mut SchemaGraph,
    template: TableTemplate,
    position: (f64, f64),
) -> Result<NodeIndex, String> {
    let (name, columns) = match template {
        TableTemplate::Users => (
            "users",
            vec![
                Column::new("id", "INTEGER").primary_key(),
                Column::new("username", "VARCHAR(255)"),
                Column::new("email", "VARCHAR(255)"),
            ],
        ),
        TableTemplate::Posts => (
            "posts",
            vec![
                Column::new("id", "INTEGER").primary_key(),
                Column::new("title", "VARCHAR(255)"),
                Column::new("content", "TEXT"),
            ],
        ),
        TableTemplate::Products => (
            "products",
            vec![
                Column::new("id", "INTEGER").primary_key(),
                Column::new("name", "VARCHAR(255)"),
                Column::new("price", "DECIMAL(10,2)"),
            ],
        ),
    };

    let idx = graph.create_table(name, position)?;
    
    if let Some(table) = graph.node_weight_mut(idx) {
        for column in columns {
            table.create_column(column);
        }
    }
    
    Ok(idx)
}
```

---

## Troubleshooting

### Issue: "Table already exists" error

```rust
// Solution: Check before creating
if graph.table_exists("users") {
    println!("Table exists, using existing one");
} else {
    graph.create_table("users", pos)?;
}
```

### Issue: Lost NodeIndex reference

```rust
// Solution: Store indices or use name lookup
let user_table = graph.find_table_by_name("users")
    .expect("Users table should exist");
```

### Issue: Table created but not visible

```rust
// Check position
if let Some(table) = graph.node_weight(idx) {
    println!("Table position: {:?}", table.position);
    // Adjust if off-screen
}
```

---

**For more information, see:**
- [Table Creation Documentation](TABLE_CREATION.md)
- [Implementation Summary](IMPLEMENTATION_SUMMARY.md)
- [Quick Start Guide](QUICK_START.md)