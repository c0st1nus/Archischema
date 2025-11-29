# Canvas Navigation Guide

This guide explains how to navigate and interact with the Diagramix canvas.

## 🔍 Zoom Controls

### Mouse Wheel Zoom
- **Zoom In**: Hold `Ctrl` and scroll **up** with the mouse wheel
- **Zoom Out**: Hold `Ctrl` and scroll **down** with the mouse wheel
- **Zoom Range**: 10% to 500%

### Keyboard Zoom
- **Zoom In**: Press `Ctrl` + `+` (or `Ctrl` + `=`)
- **Zoom Out**: Press `Ctrl` + `-`

The current zoom level is displayed in the Quick Help panel in the top-right corner.

## 🖱️ Pan/Move Canvas

### Middle Mouse Button
- **Click and hold** the middle mouse button (scroll wheel button)
- **Drag** to move the canvas around
- **Release** to stop panning

This allows you to navigate large schemas without losing your current zoom level.

## ✋ Drag Tables

### Moving Tables
- **Click and hold** the left mouse button on any table
- **Drag** to reposition the table
- **Release** to place the table

The dragging system is zoom-aware, so tables will move correctly regardless of your current zoom level.

## 📊 Quick Tips

1. **Zoom to Focus**: Use zoom to focus on specific areas of complex schemas
2. **Pan for Overview**: Pan around to see different parts of your schema
3. **Combine Actions**: Zoom in on a table, then drag it to fine-tune positioning
4. **Quick Reset**: If you get lost, you can always zoom out to see the full schema

## 🎯 Best Practices

- **Start Wide**: Begin with a zoomed-out view to see the overall structure
- **Zoom In**: Use zoom when you need to read column details or make precise adjustments
- **Use Middle Button**: The middle mouse button provides the smoothest panning experience
- **Keyboard Shortcuts**: Use `Ctrl+/Ctrl-` for quick zoom adjustments while editing

## 🛠️ Technical Details

The canvas uses CSS transforms for smooth, hardware-accelerated zooming and panning. All transformations are applied with `transform-origin: 0 0` to ensure consistent behavior across different zoom levels.