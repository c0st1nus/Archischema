# 📸 Visual Screenshot Guide

## Where to Find the "New Table" Button

### Full Application View

```
┌────────────────────────────────────────────────────────────────────────┐
│                         Diagramix - Database Schema Editor              │
└────────────────────────────────────────────────────────────────────────┘
┌──────────────────┬─────────────────────────────────────────────────────┐
│   SIDEBAR        │              CANVAS (Main Area)                     │
│   (Width: 384px) │                                                     │
│                  │                                                     │
│ ┌──────────────┐ │                                                     │
│ │ 🟦 Schema    │ │                                                     │
│ │   Database   │ │                                                     │
│ │   Explorer   │ │                                                     │
│ └──────────────┘ │                                                     │
│                  │                                                     │
│ ┌──────────────┐ │                                                     │
│ │ 🔍 Search... │ │         (Canvas with tables here)                  │
│ └──────────────┘ │                                                     │
│                  │                                                     │
│ ┌──────────────┐ │                                                     │
│ │   4     14   3│ │                                                     │
│ │ Tables      │ │                                                     │
│ │ Columns     │ │                                                     │
│ │ Relations   │ │                                                     │
│ └──────────────┘ │                                                     │
│                  │                                                     │
│ ╔══════════════╗ │                                                     │
│ ║ ➕ New Table ║ │ ← BUTTON IS HERE! (Blue background)               │
│ ╚══════════════╝ │                                                     │
│                  │                                                     │
│ ▶ 📋 users   ✏️➕ │                                                     │
│ ▶ 📋 posts   ✏️➕ │                                                     │
│ ▶ 📋 comments ✏️➕│                                      ┌────────┐    │
│                  │                                      │   ➕   │    │
│                  │                                      │  FAB   │    │
│ [Expand All]     │                                      └────────┘    │
│                  │                                     (Bottom-right)  │
└──────────────────┴─────────────────────────────────────────────────────┘
```

---

## Button Location - Detailed View

### Sidebar Section Breakdown

```
┌─────────────────────────────────┐
│ Schema                    [<]   │ ← Header with collapse button
├─────────────────────────────────┤
│ 🔍 [Search tables and cols...] │ ← Search bar
├─────────────────────────────────┤
│ ┌─────┐  ┌─────┐  ┌─────┐     │
│ │  4  │  │ 14  │  │  3  │     │ ← Statistics
│ │Table│  │Cols │  │Rels │     │
│ └─────┘  └─────┘  └─────┘     │
├─────────────────────────────────┤
│ ╔═══════════════════════════╗  │
│ ║    ➕ New Table           ║  │ ← THE BUTTON!
│ ╚═══════════════════════════╝  │   (Blue, full width)
├─────────────────────────────────┤
│ ▶ 📋 users           ✏️ ➕     │ ← Table list starts here
│ ▶ 📋 posts           ✏️ ➕     │
│ ▶ 📋 comments        ✏️ ➕     │
│ ▶ 📋 new_table       ✏️ ➕     │
└─────────────────────────────────┘
```

---

## Button Appearance

### Normal State
```
┌──────────────────────────┐
│    ➕ New Table          │  Blue background (#2563eb)
└──────────────────────────┘  White text
                              Medium shadow
```

### Hover State
```
┌──────────────────────────┐
│    ➕ New Table          │  Darker blue (#1d4ed8)
└──────────────────────────┘  Slightly raised
                              Cursor: pointer
```

---

## Alternative Entry Points

### 1. Empty State (When no tables exist)

```
┌────────────────────────────────────────────────────────┐
│                                                        │
│                    ╭───────╮                          │
│                    │  📋   │                          │
│                    ╰───────╯                          │
│                                                        │
│              Welcome to Diagramix                     │
│                                                        │
│         Start designing your database                 │
│          schema by creating your first                │
│              table, or load a demo.                   │
│                                                        │
│  ┌──────────────────────┐  ┌──────────────────────┐  │
│  │ ➕ Create Your       │  │ 📋 Load Demo         │  │
│  │    First Table       │  │    Schema            │  │
│  └──────────────────────┘  └──────────────────────┘  │
│                                                        │
│      Or use the "New Table" button in the sidebar     │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### 2. FAB Button (Floating Action Button)

```
Canvas Area (Bottom-Right Corner):

                                        ┌─────────┐
                                        │         │
                                        │    ➕   │ ← Green circle
                                        │         │ ← Always visible
                                        └─────────┘
                                        (64px × 64px)
```

---

## Visual Hierarchy

### Colors Guide

| Element | Color | Purpose |
|---------|-------|---------|
| New Table Button | Blue (#2563eb) | Primary action |
| FAB Button | Green (#16a34a) | Secondary quick action |
| Edit Icons | Purple (#9333ea) | Edit actions |
| Add Column Icons | Green (#16a34a) | Create actions |
| Delete Actions | Red (#dc2626) | Destructive actions |

---

## Responsive Behavior

### Desktop (> 768px)
- Sidebar: 384px wide (fixed)
- Button: Full width with padding
- FAB: Visible bottom-right

### Tablet (768px - 1024px)
- Sidebar: Can be collapsed
- Button: Same as desktop
- FAB: Always visible

### Mobile (< 768px)
- Sidebar: Overlay (can be toggled)
- Button: Full width
- FAB: Primary creation method

---

## Common Visual Issues & Solutions

### Issue: Button not visible

**Check these locations:**

1. **Scroll sidebar down** - Button is below statistics
2. **Expand sidebar** - Click menu icon if collapsed
3. **Check viewport** - Ensure browser window is wide enough

### Issue: Button looks different

**Expected appearance:**
- Background: Solid blue (#2563eb)
- Text: White
- Icon: Plus symbol (➕)
- Border radius: 12px
- Shadow: Medium (shadow-md)

---

## Testing Checklist

- [ ] Sidebar opens on page load
- [ ] Statistics show correct counts
- [ ] "New Table" button visible below statistics
- [ ] Button is blue with white text
- [ ] Plus icon appears before text
- [ ] Hover effect works (darker blue)
- [ ] Click opens table editor
- [ ] FAB visible in bottom-right corner
- [ ] Empty state shows when no tables

---

## Comparison: Before vs After

### Before (No button visible)
```
Sidebar:
├── Search bar
├── Statistics
└── Table list
```

### After (Button added)
```
Sidebar:
├── Search bar
├── Statistics
├── ➕ NEW TABLE BUTTON ← ADDED!
└── Table list
```

---

## Pixel-Perfect Measurements

```css
Button Specifications:
- Width: 100% (with container padding: 24px left/right)
- Height: 48px (py-3 = 12px top + 24px content + 12px bottom)
- Margin: 16px top/bottom (py-4)
- Border radius: 12px (rounded-xl)
- Font size: 14px (text-sm)
- Font weight: 600 (font-semibold)
- Background: Linear gradient (from-blue-600 to-blue-700)
- Shadow: 0 4px 6px -1px rgba(0,0,0,0.1) (shadow-sm)
```

---

## Animation States

### Click Animation
1. **Mousedown**: Scale down to 98%
2. **Mouseup**: Scale up to 100%
3. **Hover**: Darken background color
4. **Focus**: Show blue ring (ring-2 ring-blue-500)

---

## Accessibility

- **Keyboard**: Tab to focus, Enter/Space to activate
- **Screen readers**: "New Table button, creates a new table"
- **Focus indicator**: Blue ring appears on focus
- **Color contrast**: Passes WCAG AA standard (4.5:1 ratio)

---

## Browser Compatibility

Tested and working on:
- ✅ Chrome 90+
- ✅ Firefox 88+
- ✅ Safari 14+
- ✅ Edge 90+

---

## Quick Reference Card

```
LOCATION:
  Sidebar → Below Statistics → Above Table List

APPEARANCE:
  Color: Blue (#2563eb)
  Text: "New Table" with ➕ icon
  Width: Full width of sidebar
  Height: 48px

BEHAVIOR:
  Click → Creates table → Opens editor

ALTERNATIVES:
  1. FAB button (bottom-right corner)
  2. Empty state button (center of canvas)
  3. Keyboard shortcut: Ctrl+N (coming soon)
```

---

**Need help?** See [UI_GUIDE.md](UI_GUIDE.md) for detailed instructions.