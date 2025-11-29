# Canvas Zoom & Pan Implementation Notes

## Overview

This document describes the technical implementation of zoom and pan functionality in the Diagramix canvas component.

## Architecture

### State Management

The canvas transformation is managed through Leptos reactive signals:

```rust
let (zoom, set_zoom) = signal(1.0_f64);      // Zoom level (0.1 to 5.0)
let (pan_x, set_pan_x) = signal(0.0_f64);    // X offset in pixels
let (pan_y, set_pan_y) = signal(0.0_f64);    // Y offset in pixels
let (panning, set_panning) = signal::<Option<(f64, f64)>>(None); // Pan state
```

### Event Handling Strategy

#### Why `node_ref` Instead of Document-Level Events?

Initially, event listeners were attached to `document`, but this approach had issues:

1. **Browser Default Behavior**: The browser's native Ctrl+Scroll zoom couldn't be prevented
2. **Passive Events**: Document-level wheel events are passive by default for performance
3. **Context Issues**: Events needed to know about the canvas boundaries

**Solution**: Use `NodeRef` to attach listeners directly to the canvas element with `passive: false`:

```rust
let canvas_ref = NodeRef::<html::Div>::new();

// In Effect block:
let mut options = web_sys::AddEventListenerOptions::new();
options.passive(false);

canvas_element.add_event_listener_with_callback_and_add_event_listener_options(
    "wheel",
    wheel_handler.as_ref().unchecked_ref(),
    &options,
).unwrap();
```

### Zoom Implementation

#### Mouse Wheel Zoom

- Detects `Ctrl + Scroll` combination
- Zoom factor: 1.1x for zoom in, 0.9x for zoom out
- Clamped between 0.1 (10%) and 5.0 (500%)
- Prevents default browser zoom behavior

```rust
if ev.ctrl_key() {
    ev.prevent_default();
    ev.stop_propagation();
    
    let zoom_factor = if delta < 0.0 { 1.1 } else { 0.9 };
    set_zoom.update(|z| *z = (*z * zoom_factor).clamp(0.1, 5.0));
}
```

#### Keyboard Zoom

- Listens for `Ctrl + +` and `Ctrl + -` keys
- Same zoom factors and clamping as wheel zoom
- Attached to document level (appropriate for keyboard events)

### Pan Implementation

#### Middle Mouse Button Pan

Uses a two-phase approach:

1. **Initiation**: On `mousedown` with button 1 (middle), store initial position
2. **Movement**: Track mouse delta and update pan offsets
3. **Completion**: On `mouseup`, clear panning state

```rust
// Store initial position
set_panning.set(Some((ev.client_x() as f64, ev.client_y() as f64)));

// In move handler:
let dx = ev.client_x() as f64 - start_x;
let dy = ev.client_y() as f64 - start_y;
set_pan_x.set(initial_pan_x + dx);
set_pan_y.set(initial_pan_y + dy);
```

### Transform Application

The transformation is applied via CSS transform on a container div:

```rust
style:transform=move || format!(
    "translate({}px, {}px) scale({})",
    pan_x.get(),
    pan_y.get(),
    zoom.get()
)
style:transform-origin="0 0"
```

**Key Points**:
- `transform-origin: 0 0` ensures scaling from top-left corner
- `transition: none` prevents animation lag during interaction
- Applied to a wrapper div containing tables and SVG edges

### Coordinate Space Transformation

When dragging tables, coordinates must be transformed from screen space to canvas space:

```rust
// Screen to canvas space
let canvas_x = (screen_x - pan_x) / zoom;
let canvas_y = (screen_y - pan_y) / zoom;

// Canvas to screen space (for drag offset calculation)
let screen_x = canvas_x * zoom + pan_x;
let screen_y = canvas_y * zoom + pan_y;
```

This ensures tables are positioned correctly regardless of zoom/pan state.

## Performance Considerations

### Batched Updates

Uses Leptos `batch()` for grouping multiple signal updates:

```rust
batch(move || {
    graph.update(|g| {
        if let Some(node) = g.node_weight_mut(node_idx) {
            node.position = (new_x, new_y);
        }
    });
});
```

### Hardware Acceleration

- CSS transforms leverage GPU acceleration
- No JavaScript-based animation loops
- Smooth 60fps performance even on complex schemas

### Closure Management

Closures are carefully managed to prevent memory leaks:

1. Old closures removed before adding new ones
2. `forget()` used for stable, long-lived handlers
3. `Rc<RefCell<>>` pattern for mutable closure storage

## Browser Compatibility

### Tested Features

- ✅ Chrome/Edge: Full support
- ✅ Firefox: Full support  
- ✅ Safari: Full support (WebKit)

### Known Issues

None currently reported.

## Future Enhancements

Potential improvements for consideration:

1. **Zoom to Point**: Zoom towards mouse cursor position instead of origin
2. **Touch Gestures**: Pinch-to-zoom on mobile/tablet devices
3. **Minimap**: Small overview map for large schemas
4. **Zoom Presets**: Quick buttons for 50%, 100%, 200% zoom
5. **Fit to Screen**: Automatically adjust zoom/pan to show all tables
6. **Animated Transitions**: Smooth transitions when focusing on tables

## Dependencies

- `leptos`: Reactive framework and DOM manipulation
- `web_sys`: Browser API bindings (MouseEvent, WheelEvent, KeyboardEvent)
- `wasm_bindgen`: Closure management for event handlers

## Testing Notes

When testing these features:

1. Test at various zoom levels (10%, 100%, 500%)
2. Test panning at different zoom levels
3. Test drag-and-drop while zoomed/panned
4. Verify no memory leaks during extended use
5. Check that keyboard shortcuts don't conflict with browser defaults
6. Verify middle mouse button doesn't trigger browser autoscroll

## Code Location

- Main implementation: `src/ui/canvas.rs`
- Component: `SchemaCanvas`
- Lines: ~10-280 (event handlers and state)
- Lines: ~280-480 (view rendering)