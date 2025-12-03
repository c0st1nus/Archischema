# UI Guide: Finding UI Elements in Diagramix

## 🎯 Where to Find Everything

### Creating Tables - 3 Ways

#### 1. **Sidebar Button** (Primary Method)
```
┌─────────────────────────────┐
│ Schema                      │
│ Database Explorer           │
├─────────────────────────────┤
│ 🔍 Search tables...         │
├─────────────────────────────┤
│   4        14        3      │
│ Tables  Columns  Relations  │
├─────────────────────────────┤
│ ┌─────────────────────────┐ │
│ │  ➕ New Table           │ │ ← HERE!
│ └─────────────────────────┘ │
├─────────────────────────────┤
│ 📋 users                    │
│ 📋 posts                    │
│ 📋 comments                 │
└─────────────────────────────┘
```

**Location**: Sidebar → Below statistics → Blue "New Table" button

**How to use**:
1. Open the left sidebar
2. Look below the statistics (Tables/Columns/Relations)
3. Click the blue "New Table" button
4. Table editor opens automatically

---

#### 2. **FAB Button** (Quick Access)
```
                        Canvas Area
┌────────────────────────────────────────┐
│                                        │
│    ┌────────┐      ┌────────┐        │
│    │ Table1 │      │ Table2 │        │
│    └────────┘      └────────┘        │
│                                        │
│                                        │
│                                        │
│                               ┌─────┐  │
│                               │  ➕  │ │ ← HERE!
│                               └─────┘  │
└────────────────────────────────────────┘
```

**Location**: Bottom-right corner of canvas

**How to use**:
1. Look at the bottom-right corner of the canvas
2. Find the large circular green button with "+"
3. Click it to instantly create a table

---

#### 3. **Empty State** (First Time)
```
┌────────────────────────────────────────┐
│                                        │
│            ┌───────────┐              │
│            │  📋       │              │
│            └───────────┘              │
│                                        │
│      Welcome to Diagramix             │
│                                        │
│   Start designing your database       │
│                                        │
│  ┌──────────────────────────────┐    │
│  │  ➕ Create Your First Table  │    │ ← HERE!
│  └──────────────────────────────┘    │
│                                        │
└────────────────────────────────────────┘
```

**Location**: Center of canvas (only when no tables exist)

**How to use**:
1. When you first open Diagramix (or delete all tables)
2. Click the large button in the center
3. Your first table is created!

---

### Editing Tables

#### Edit Table Name
```
Sidebar - Table List:
┌─────────────────────────────┐
│ 📋 users            ✏️  ➕  │ ← Click the ✏️ (edit icon)
│ 📋 posts            ✏️  ➕  │
│ 📋 comments         ✏️  ➕  │
└─────────────────────────────┘
```

**Location**: Next to each table name in sidebar

**How to use**:
1. Find your table in the sidebar list
2. Click the purple pencil icon (✏️)
3. Table editor opens

---

#### Add Columns
```
Sidebar - Table List:
┌─────────────────────────────┐
│ 📋 users            ✏️  ➕  │ ← Click the ➕ (plus icon)
│ 📋 posts            ✏️  ➕  │
│ 📋 comments         ✏️  ➕  │
└─────────────────────────────┘
```

**Location**: Next to each table name in sidebar

**How to use**:
1. Find your table in the sidebar list
2. Click the green plus icon (➕)
3. Column editor opens

---

### Current View Example

Based on your screenshot, here's what you should see:

```
┌─────────────────────────────────────────────────────────┐
│ Sidebar              │         Canvas                   │
├──────────────────────┼──────────────────────────────────┤
│ Schema               │                                   │
│ Database Explorer    │                                   │
│ ─────────────────    │                                   │
│ 🔍 Search...         │                                   │
│ ─────────────────    │                                   │
│   4      14      3   │                                   │
│ Tables Columns Rels  │                                   │
│ ─────────────────    │                                   │
│ ┌─────────────────┐  │                                   │
│ │ ➕ New Table    │  │ ← YOU SHOULD SEE THIS!           │
│ └─────────────────┘  │                                   │
│ ─────────────────    │                                   │
│ ▶ 📋 users      ✏️➕ │                                   │
│ ▶ 📋 posts      ✏️➕ │                                   │
│ ▶ 📋 comments   ✏️➕ │                                   │
│ ▼ 📋 new_table  ✏️➕ │                                   │
│   No columns yet     │                                   │
│   + Add first col    │                                   │
│ ─────────────────    │                                   │
│ [Expand All]         │                                   │
└──────────────────────┴──────────────────────────────────┘
```

---

## 🚨 Troubleshooting

### "I don't see the New Table button!"

**Possible reasons**:

1. **Old version** - Make sure you've pulled the latest code
   ```bash
   git pull
   cargo clean
   cargo leptos watch
   ```

2. **Cache issue** - Hard refresh your browser
   - Chrome/Edge: `Ctrl + Shift + R` (Windows) or `Cmd + Shift + R` (Mac)
   - Firefox: `Ctrl + F5` (Windows) or `Cmd + Shift + R` (Mac)

3. **Server not restarted** - Restart the dev server
   ```bash
   # Stop: Ctrl + C
   # Start again:
   cargo leptos watch
   ```

4. **Wrong branch** - Make sure you're on the correct branch with the new feature

---

### "The button is there but nothing happens!"

**Check**:
1. Open browser DevTools (F12)
2. Look at the Console tab for errors
3. Try clicking the FAB button instead (bottom-right corner)

---

### "I created a table but can't see it!"

**Solutions**:
1. **Scroll the canvas** - Table might be off-screen
2. **Check the sidebar** - Table appears in the list
3. **Pan the canvas** - Middle-click and drag to find it

---

## ⌨️ Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Save (in editor) | `Enter` |
| Cancel (in editor) | `Esc` |
| Pan canvas | Middle mouse + drag |
| Zoom | `Ctrl + Scroll` |

---

## 🎨 Visual Reference

### Button Colors

- **🔵 Blue** - Primary actions (New Table button in sidebar)
- **🟢 Green** - Create actions (FAB button, Add Column)
- **🟣 Purple** - Edit actions (Edit icon)
- **🔴 Red** - Delete actions (Delete button)

### Button States

- **Normal**: Solid color
- **Hover**: Darker shade
- **Active**: Pressed appearance
- **Disabled**: Grayed out

---

## 📱 Layout Breakdown

```
Full Application Layout:

┌────────────────────────────────────────────────────────────┐
│                     Top Bar (if any)                       │
├──────────────┬─────────────────────────────────────────────┤
│              │                                             │
│   SIDEBAR    │              CANVAS                         │
│   (Left)     │              (Main Area)                    │
│              │                                             │
│  ┌────────┐  │  ┌─────┐    ┌─────┐    ┌─────┐            │
│  │Search  │  │  │Table│    │Table│    │Table│            │
│  └────────┘  │  └─────┘    └─────┘    └─────┘            │
│              │                                             │
│  ┌────────┐  │                                             │
│  │ Stats  │  │                                             │
│  └────────┘  │                                             │
│              │                                  ┌────┐     │
│  ┌────────┐  │                                  │FAB │     │
│  │➕ New  │  │                                  └────┘     │
│  └────────┘  │                                             │
│              │                                             │
│  Table List  │                                             │
│  • users     │                                             │
│  • posts     │                                             │
│              │                                             │
└──────────────┴─────────────────────────────────────────────┘
```

---

## 🔄 Workflow Example

### Creating a Table: Step by Step

1. **Find the button** (any of the 3 ways above)
2. **Click it** → Table is created with name "new_table"
3. **Editor opens automatically** with name field focused
4. **Type your table name** (e.g., "products")
5. **Press Enter** → Table is saved
6. **Add columns** by clicking the ➕ icon
7. **Done!** Your table is ready

---

## 💡 Pro Tips

- Use **Sidebar button** when you want to name the table immediately
- Use **FAB button** for quick creation while designing
- **Expand tables** in sidebar to see their columns
- **Click table name** in sidebar to focus it on canvas
- **Right icons** (✏️➕) for quick actions

---

Need more help? Check:
- [Quick Start Guide](QUICK_START.md)
- [Table Creation Documentation](TABLE_CREATION.md)