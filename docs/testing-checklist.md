# Testing Checklist: Canvas Zoom & Pan Features

## Pre-Testing Setup

- [ ] Run `cargo leptos watch` to start the development server
- [ ] Open browser at `http://127.0.0.1:3000`
- [ ] Open browser DevTools Console to check for JavaScript errors
- [ ] Test on multiple browsers (Chrome, Firefox, Safari if available)

## Zoom Functionality Tests

### Mouse Wheel Zoom
- [ ] **Basic Zoom In**: Hold `Ctrl` and scroll **up** → Canvas should zoom in
- [ ] **Basic Zoom Out**: Hold `Ctrl` and scroll **down** → Canvas should zoom out
- [ ] **Zoom Range**: Test that zoom stops at 10% (min) and 500% (max)
- [ ] **Browser Zoom Prevention**: Verify that browser page zoom does NOT occur when using Ctrl+Scroll on canvas
- [ ] **Zoom Display**: Check that zoom percentage updates in Quick Help panel
- [ ] **Table Scaling**: Verify that tables scale correctly with zoom
- [ ] **SVG Edge Scaling**: Verify that connection lines scale correctly with zoom

### Keyboard Zoom
- [ ] **Zoom In (Plus)**: Press `Ctrl` + `+` → Canvas should zoom in
- [ ] **Zoom In (Equals)**: Press `Ctrl` + `=` → Canvas should zoom in
- [ ] **Zoom Out**: Press `Ctrl` + `-` → Canvas should zoom out
- [ ] **Rapid Zoom**: Hold `Ctrl` and press `+` multiple times quickly → Should zoom smoothly
- [ ] **Zoom Range**: Verify keyboard zoom also respects 10%-500% limits

### Zoom Edge Cases
- [ ] **No Ctrl Key**: Scroll without Ctrl → Should NOT zoom canvas
- [ ] **Outside Canvas**: Ctrl+Scroll outside canvas area (on sidebar) → Canvas should NOT zoom
- [ ] **Zoom at 10%**: Try to zoom out further → Should stay at 10%
- [ ] **Zoom at 500%**: Try to zoom in further → Should stay at 500%

## Pan Functionality Tests

### Middle Mouse Button Pan
- [ ] **Basic Pan**: Hold middle mouse button and drag → Canvas should move
- [ ] **Pan Up**: Pan canvas upward → Tables should move up
- [ ] **Pan Down**: Pan canvas downward → Tables should move down
- [ ] **Pan Left**: Pan canvas left → Tables should move left
- [ ] **Pan Right**: Pan canvas right → Tables should move right
- [ ] **Smooth Movement**: Verify panning is smooth without jitter
- [ ] **Release Button**: Release middle button → Panning should stop immediately

### Pan Edge Cases
- [ ] **Pan While Zoomed In**: Zoom to 200%, then pan → Should work correctly
- [ ] **Pan While Zoomed Out**: Zoom to 50%, then pan → Should work correctly
- [ ] **Quick Pan**: Pan rapidly in different directions → Should follow cursor smoothly
- [ ] **Context Menu**: Right-click during pan → Should not open context menu

## Combined Zoom & Pan Tests

- [ ] **Zoom Then Pan**: Zoom in, then pan around → Both should work together
- [ ] **Pan Then Zoom**: Pan to a location, then zoom → Should zoom from that position
- [ ] **Zoom-Pan-Zoom**: Chain multiple zoom and pan operations → Should work smoothly

## Table Drag & Drop Integration

### Drag While Zoomed
- [ ] **Drag at 100%**: Drag table at normal zoom → Should work as before
- [ ] **Drag at 200%**: Zoom to 200%, drag table → Table should follow cursor correctly
- [ ] **Drag at 50%**: Zoom to 50%, drag table → Table should follow cursor correctly
- [ ] **Drag at 500%**: Zoom to 500%, drag table → Should work (even if viewport is small)

### Drag While Panned
- [ ] **Drag After Pan**: Pan canvas, then drag table → Table should move correctly
- [ ] **Drag With Offset**: Pan significantly, then drag → Coordinates should be correct

### Combined Tests
- [ ] **Drag at Zoom+Pan**: Zoom to 150% and pan, then drag table → Should work correctly
- [ ] **Drop Position**: Verify dropped table position is correct in canvas coordinates
- [ ] **SVG Lines Update**: After dragging, connection lines should update correctly

## Performance Tests

- [ ] **Rapid Zoom**: Zoom in and out rapidly → Should remain smooth (60fps)
- [ ] **Continuous Pan**: Pan continuously for 10 seconds → No lag or memory issues
- [ ] **Large Schema**: Test with many tables (10+) → Performance should remain good
- [ ] **Browser Memory**: Check DevTools Memory tab → No memory leaks during use

## UI/UX Tests

### Visual Feedback
- [ ] **Zoom Indicator**: Zoom percentage displays correctly (e.g., "Zoom: 150%")
- [ ] **Cursor Changes**: Cursor changes appropriately during pan (if applicable)
- [ ] **Grid Background**: Background grid scales with zoom
- [ ] **Text Readability**: Table text remains readable at various zoom levels

### Help Panel
- [ ] **Instructions Visible**: Quick Help panel shows correct instructions
- [ ] **Help Text**: Verify all shortcuts are listed correctly
- [ ] **Zoom Display**: Real-time zoom percentage updates as you zoom

## Cross-Browser Testing

### Chrome/Edge (Chromium)
- [ ] All zoom tests pass
- [ ] All pan tests pass
- [ ] No console errors

### Firefox
- [ ] All zoom tests pass
- [ ] All pan tests pass
- [ ] No console errors
- [ ] Middle mouse button doesn't trigger Firefox autoscroll

### Safari (if available)
- [ ] All zoom tests pass
- [ ] All pan tests pass
- [ ] No console errors

## Regression Tests

- [ ] **Sidebar Functionality**: Sidebar still works correctly
- [ ] **Column Editing**: Can still edit columns in sidebar
- [ ] **Table Creation**: Can still create new tables (if applicable)
- [ ] **Relationship Lines**: Lines still render between tables
- [ ] **Existing Drag & Drop**: Original drag functionality still works

## Known Issues to Watch For

- [ ] Middle mouse button triggering browser autoscroll (should be prevented)
- [ ] Ctrl+Scroll zooming the browser page instead of canvas (should be prevented)
- [ ] Table positions being incorrect after zoom/pan
- [ ] SVG lines not scaling properly
- [ ] Memory leaks from event listeners
- [ ] Zoom indicator not updating

## Bug Reporting

If you find any issues, please report:
1. **What you did**: Exact steps to reproduce
2. **What happened**: Actual behavior
3. **What should happen**: Expected behavior
4. **Browser**: Browser name and version
5. **Console errors**: Any errors in browser DevTools Console
6. **Zoom level**: Current zoom percentage when bug occurred
7. **Pan offset**: Whether canvas was panned when bug occurred

## Sign-Off

- [ ] All critical tests passed
- [ ] No blocking bugs found
- [ ] Performance is acceptable
- [ ] Ready for deployment

**Tester Name**: _______________
**Date**: _______________
**Browser(s) Tested**: _______________
**Notes**: 
