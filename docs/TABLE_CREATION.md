# Table Creation Feature

## Overview

The table creation feature allows users to easily add new tables to their database schema through multiple intuitive entry points.

## Features

### 1. Create Table via Sidebar

The primary method for creating tables is through the sidebar:

- **Location**: Top of the sidebar, below statistics section
- **Button**: Green "**+ New Table**" button
- **Behavior**:
  - Creates a new table with auto-generated name (e.g., `new_table`, `new_table_1`, etc.)
  - Automatically opens the table editor for renaming
  - Table appears at position (300, 300) on the canvas
  - The new table is automatically expanded in the sidebar list

### 2. Floating Action Button (FAB)

Quick access button always visible on the canvas:

- **Location**: Bottom-right corner of the canvas
- **Appearance**: Large green circular button with "+" icon
- **Keyboard Shortcut**: Ctrl+N (tooltip hint)
- **Behavior**:
  - Creates table at viewport center (400, 300)
  - Table appears immediately on canvas
  - Auto-generated unique name

### 3. Empty State

When no tables exist, a welcome screen is displayed:

- **Location**: Center of canvas
- **Content**:
  - Welcome message
  - Large call-to-action button: "Create Your First Table"
  - Helper text mentioning the sidebar button
- **Purpose**: Guides new users on how to get started

## Table Editor

After creating a table, users can edit its properties:

### Accessible via:
- Clicking the edit icon (purple pencil) next to any table name in sidebar
- Automatically opens after creating a new table

### Features:
- **Table Name**: Required field with validation
  - Cannot be empty
  - Must be unique across the schema
  - Real-time validation with error messages
- **Statistics Display**:
  - Number of columns
  - Number of relationships
- **Actions**:
  - **Save** (Enter key): Saves changes and returns to table list
  - **Cancel** (Esc key): Discards changes and returns to table list
  - **Delete Table**: Removes table and all its relationships
- **Keyboard Shortcuts**:
  - `Enter` - Save changes
  - `Esc` - Cancel editing

## Technical Implementation

### Core Components

#### 1. `TableOps` Trait (`src/core/schema.rs`)
Provides table management operations:

```rust
pub trait TableOps {
    fn create_table(&mut self, name: impl Into<String>, position: (f64, f64)) -> Result<NodeIndex, String>;
    fn create_table_auto(&mut self, position: (f64, f64)) -> NodeIndex;
    fn rename_table(&mut self, node_idx: NodeIndex, new_name: impl Into<String>) -> Result<(), String>;
    fn delete_table(&mut self, node_idx: NodeIndex) -> Result<TableNode, String>;
    fn table_exists(&self, name: &str) -> bool;
    fn find_table_by_name(&self, name: &str) -> Option<NodeIndex>;
    fn generate_unique_table_name(&self, base_name: &str) -> String;
}
```

#### 2. `TableEditor` Component (`src/ui/table_editor.rs`)
Provides inline editing UI for table properties:
- Auto-focus on table name input
- Real-time validation
- Keyboard navigation
- Statistics display
- Delete confirmation

#### 3. `EditingMode` Enum (`src/ui/sidebar.rs`)
Manages sidebar state:
```rust
enum EditingMode {
    None,                                    // Viewing table list
    EditingColumn(NodeIndex, Option<usize>), // Editing a column
    EditingTable(NodeIndex),                 // Editing a table
}
```

### Validation Rules

#### Table Names:
1. **Non-empty**: Name cannot be blank or whitespace-only
2. **Unique**: No two tables can have the same name
3. **Case-sensitive**: `Users` and `users` are different tables

#### Auto-naming:
- Base name: `new_table`
- If exists, appends counter: `new_table_1`, `new_table_2`, etc.
- Counter increments until unique name is found

### User Experience Flow

```
User clicks "+ New Table" button
        ↓
System generates unique name (e.g., "new_table_1")
        ↓
Table created at default position
        ↓
Table Editor opens automatically
        ↓
User enters desired table name
        ↓
User presses Enter or clicks "Save"
        ↓
System validates name (not empty, unique)
        ↓
If valid: Save and return to table list
If invalid: Show error message
```

## Icons Used

- `plus` - Create new table buttons
- `edit` - Edit table button
- `trash` - Delete table button
- `check` - Save button
- `alert-circle` - Error messages
- `loader` - Saving state indicator
- `chevron-left` - Back navigation
- `table` - Empty state icon

## Best Practices

### For Users:
1. Use descriptive table names (e.g., `users`, `orders`, `products`)
2. Rename tables immediately after creation for better organization
3. Use the FAB button for quick table creation while designing
4. Use the sidebar button when you want to immediately edit table properties

### For Developers:
1. Always use `TableOps` trait methods for table operations
2. Validate table names before operations
3. Handle errors gracefully with user-friendly messages
4. Use `write()` instead of `update()` when you need return values from mutations
5. Maintain unique table names at all times

## Future Enhancements

Potential improvements for future versions:

- [ ] Drag and drop table templates
- [ ] Duplicate table functionality
- [ ] Table name suggestions based on common patterns
- [ ] Batch table creation from CSV/JSON
- [ ] Undo/redo for table operations
- [ ] Context menu (right-click) on canvas for table creation
- [ ] Double-click on empty canvas to create table at that position
- [ ] Keyboard shortcut (Ctrl+N) implementation
- [ ] Table grouping/folders for large schemas
- [ ] Import tables from existing database connections