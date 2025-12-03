# Quick Start: Creating Tables in Diagramix

## 🚀 Getting Started

Diagramix makes it easy to create and manage database tables visually. Here are three simple ways to add tables to your schema.

---

## Method 1: Sidebar Button (Recommended)

**Perfect for**: Adding tables with immediate editing

1. Look at the left sidebar
2. Find the green **"+ New Table"** button (below the statistics)
3. Click it
4. The table editor opens automatically
5. Type your table name and press `Enter`
6. Done! Now add columns using the **"+"** button next to the table name

**Tip**: The new table appears on the canvas at (300, 300)

---

## Method 2: Floating Action Button

**Perfect for**: Quick table creation while designing

1. Look at the bottom-right corner of the canvas
2. Click the large green circular **"+"** button
3. A new table appears on the canvas immediately
4. Edit it from the sidebar when ready

**Keyboard shortcut**: `Ctrl+N` (coming soon!)

---

## Method 3: Empty State Welcome

**Perfect for**: Your very first table

1. When you first open Diagramix, you'll see a welcome screen
2. Click **"Create Your First Table"**
3. Start building your schema!

---

## ✏️ Editing a Table

Once you've created a table, you can edit it:

1. Find your table in the sidebar
2. Click the **purple pencil icon** next to the table name
3. Change the table name
4. Press `Enter` to save or `Esc` to cancel

**Table Editor Features**:
- ✅ Real-time validation (unique names, non-empty)
- ✅ Shows column count and relationship count
- ✅ Delete table option (removes all relationships too)
- ✅ Keyboard shortcuts for quick editing

---

## ➕ Adding Columns

After creating a table:

1. Click the **"+"** button next to the table name in sidebar
2. Or expand the table and click **"+ Add first column"**
3. Fill in column details (name, type, constraints)
4. Save and repeat!

---

## 🎯 Pro Tips

- **Naming**: Use descriptive names like `users`, `orders`, `products`
- **Organization**: Rename tables right after creation for better workflow
- **Positioning**: Drag tables around the canvas to organize your layout
- **Navigation**: Use the sidebar to quickly find and edit tables
- **Relationships**: Add columns first, then create relationships between tables

---

## ⌨️ Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Save changes | `Enter` |
| Cancel editing | `Esc` |
| Pan canvas | Middle mouse button + drag |
| Zoom in/out | `Ctrl + Scroll` or `Ctrl + +/-` |

---

## 🎨 Visual Guide

```
┌─────────────────────────────────────────────────┐
│  Sidebar                    │     Canvas        │
│  ┌──────────────────┐       │                   │
│  │   Statistics     │       │   ┌─────────┐     │
│  └──────────────────┘       │   │  Table  │     │
│  ┌──────────────────┐       │   └─────────┘     │
│  │ + New Table  ◄───┼───────┼─── Method 1       │
│  └──────────────────┘       │                   │
│  ┌──────────────────┐       │                   │
│  │ 📋 users       ✏️│       │                   │
│  │   └ id          │       │                   │
│  │   └ name        │       │        ┌─────┐    │
│  └──────────────────┘       │        │  +  │◄── Method 2
│                              │        └─────┘    │
└─────────────────────────────────────────────────┘
```

---

## ❓ Troubleshooting

**Problem**: Table name turns red  
**Solution**: The name is either empty or already exists. Choose a unique name.

**Problem**: Can't save table name  
**Solution**: Make sure the name isn't blank or just whitespace.

**Problem**: Don't see my new table  
**Solution**: Check the sidebar - it's there! You might need to scroll the canvas to find it.

**Problem**: Deleted table by accident  
**Solution**: Unfortunately, there's no undo yet. You'll need to recreate it.

---

## 🎓 Next Steps

1. ✅ Create your first table
2. ✅ Add columns with data types
3. ✅ Create more tables
4. ✅ Add relationships between tables
5. ✅ Organize your canvas layout
6. ✅ Export your schema (coming soon!)

---

**Need more help?** Check out the full documentation in `TABLE_CREATION.md`

Happy schema designing! 🎉