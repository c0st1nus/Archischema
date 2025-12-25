# Phase 11: Idle Detection

## Overview
This phase implements idle and away status detection for users in collaborative sessions. It tracks user activity through mouse and keyboard events, monitors page visibility, and broadcasts activity status to other users in the room.

## Architecture

### Activity States
The system tracks three activity states:

1. **Active**: User is actively working (within 30 seconds of last activity)
2. **Idle**: User has been inactive for 30+ seconds but page is visible
3. **Away**: User's tab is hidden or they've been inactive for 10+ minutes

### Components

#### IdleDetector (Core Module)
- **Location**: `src/core/liveshare/idle_detection.rs`
- **Purpose**: Tracks user activity status with configurable thresholds
- **Key Methods**:
  - `new()`: Creates detector with default timeouts
  - `record_activity()`: Updates last activity timestamp
  - `record_page_hidden()`: Sets status to Away
  - `record_page_visible()`: Returns to Active if was Away
  - `update()`: Evaluates elapsed time and updates status
  - `status()`: Returns current ActivityStatus

#### ActivityTracker (UI Component)
- **Location**: `src/ui/activity_tracker.rs`
- **Purpose**: Monitors user activity and page visibility in the browser
- **Events Tracked**:
  - `mousemove`: Records user activity
  - `keypress`: Records user activity
  - `visibilitychange`: Detects page visibility changes
- **Update Frequency**: Every 5 seconds

#### LiveShareContext Enhancement
- **Location**: `src/ui/liveshare_client.rs`
- **New Signals**:
  - `activity_status: RwSignal<ActivityStatus>`: Current activity state
  - `last_activity_time: RwSignal<Instant>`: Timestamp of last activity
- **New Methods**:
  - `record_activity()`: Called by event listeners
  - `update_activity_status()`: Periodic status check
  - `record_page_hidden()`: Called on visibility change
  - `record_page_visible()`: Called when tab becomes visible
  - `send_idle_status(is_active)`: Broadcasts status via WebSocket
  - `get_activity_status_display()`: Returns display string

### Data Flow

```
User Activity Events
     ↓
ActivityTracker (event listeners)
     ↓
record_activity() / record_page_hidden/visible()
     ↓
LiveShareContext signals updated
     ↓
update_activity_status() (periodic, 5s)
     ↓
Status change detected?
     ↓
send_idle_status() → ClientMessage::IdleStatus
     ↓
WebSocket → Server
     ↓
ServerMessage::IdleStatus broadcast to other users
     ↓
RemoteUser.is_active field updated
     ↓
Remote cursor display updated with visual feedback
```

## Implementation Details

### Task 1.51: Activity Detector Implementation

**Status**: ✅ Complete

**Changes**:
- Created `idle_detection.rs` module with `ActivityStatus` enum
- Implemented `IdleDetector` struct with state tracking
- Added configurable thresholds:
  - IDLE_THRESHOLD: 30 seconds
  - AWAY_THRESHOLD: 600 seconds (10 minutes)
- Unit tests for status transitions

**Testing**:
```rust
#[test]
fn test_initial_status() {
    let detector = IdleDetector::new();
    assert_eq!(detector.status(), ActivityStatus::Active);
}

#[test]
fn test_page_hidden_changes_to_away() {
    let mut detector = IdleDetector::new();
    detector.record_page_hidden();
    assert_eq!(detector.status(), ActivityStatus::Away);
}
```

### Task 1.52: Timeout Implementation

**Status**: ✅ Complete

**Implementation**:
- Idle timeout: 30 seconds of no mouse/keyboard activity
- Away timeout: Page visibility change OR 10 minutes of idle
- ActivityTracker updates status every 5 seconds

**Code**:
```rust
pub fn update_activity_status(&self) -> bool {
    let elapsed = self.last_activity_time.get_untracked().elapsed();
    let idle_threshold = std::time::Duration::from_secs(30);
    let away_threshold = std::time::Duration::from_secs(600);

    let new_status = if elapsed >= away_threshold {
        ActivityStatus::Away
    } else if elapsed >= idle_threshold {
        ActivityStatus::Idle
    } else {
        ActivityStatus::Active
    };
    
    // Send update if status changed
    ...
}
```

### Task 1.53: Idle Status Broadcasting

**Status**: ✅ Complete

**Implementation**:
- Automatically sends `ClientMessage::IdleStatus` when status changes
- Uses existing protocol message type for compatibility
- Only sends when connected to LiveShare session
- Server broadcasts to all users in room

**Code**:
```rust
fn send_idle_status(&self, is_active: bool) {
    if self.connection_state.get() != ConnectionState::Connected {
        return;
    }

    let msg = ClientMessage::IdleStatus { is_active };
    send_message(&msg);
}
```

### Task 1.54: Visual Indicators

**Status**: ✅ Complete

**Remote Cursor Display**:
- Active users: Bright cursor + pulsing white dot indicator
- Idle users: Dimmed cursor (0.5 opacity) + static white/50% dot
- Hover tooltip shows status: "User is editing" or "User is idle"

**Remote User Avatars**:
- Active users: Full opacity (1.0)
- Idle users: Reduced opacity (0.5)
- Title attribute shows detailed status

**Code Changes**:
```rust
// In remote_cursors.rs
let opacity = if is_active { "1" } else { "0.5" };
let label_style = format!(
    "background-color: {}; opacity: {}; ...",
    color,
    opacity
);

// Activity indicator dot
<span class={move || {
    if is_active {
        "w-1.5 h-1.5 rounded-full bg-white animate-pulse"
    } else {
        "w-1.5 h-1.5 rounded-full bg-white/50"
    }
}}></span>
```

### Task 1.55: Auto-Disconnect (Optional)

**Status**: ⏳ Deferred

Can be implemented in Phase 12 by:
- Adding disconnect logic to ActivityTracker when Away > 10 minutes
- Showing warning notification before auto-disconnect
- Allowing user to cancel disconnect with activity

## Integration Points

### With Protocol
- Uses existing `ClientMessage::IdleStatus { is_active: bool }`
- Uses existing `ServerMessage::IdleStatus { user_id, is_active }`
- No protocol changes required

### With RemoteUser
- Updates `is_active` field from awareness state
- Cursor display automatically reflects status
- User avatars show visual distinction

### With CursorTracker
- Sends is_active in awareness state
- Coordinates with activity status updates

## Event Listeners Setup

```
App Component
    ↓
ActivityTracker mounted
    ↓
Effect::new() triggered on ConnectionState::Connected
    ↓
Register listeners:
    - document.addEventListener("mousemove", ...)
    - document.addEventListener("keypress", ...)
    - document.addEventListener("visibilitychange", ...)
    - setInterval(update_activity_status, 5000)
```

## Testing Checklist

- [x] Activity detector tracks time correctly
- [x] Status transitions work (Active → Idle → Away)
- [x] Page visibility changes affect status
- [x] Mousemove/keypress reset to Active
- [x] Idle status broadcasts to other users
- [x] Remote cursors show activity indicators
- [x] User avatars show different opacity
- [x] Tooltips display correct status text
- [x] No memory leaks from event listeners
- [x] Code compiles without warnings

## Performance Notes

1. **Event Throttling**: Activity updates limited to 5-second intervals
2. **Event Listener Cleanup**: Closures kept alive but properly scoped
3. **Memory**: Minimal overhead (2 signals per user)
4. **Network**: Only sends when status actually changes

## Browser Compatibility

- `mousemove`: All modern browsers ✓
- `keypress`: All modern browsers ✓
- `visibilitychange`: IE10+, all modern browsers ✓
- `document.hidden`: IE10+, all modern browsers ✓

## Future Enhancements

1. **Configurable Timeouts**: Allow users to adjust idle/away thresholds
2. **Inactivity Warnings**: Notify users before transitioning to Away
3. **Auto-Disconnect**: Automatically leave room after extended Away time
4. **Activity Log**: Track user activity history for analytics
5. **Idle Notifications**: Show who just went idle in chat
6. **Focus Detection**: React to window focus changes
7. **Activity Leaderboard**: Show most active users

## Debugging

### Check Current Status
```javascript
// In browser console
archischema_context.activity_status.get() // Returns ActivityStatus
archischema_context.last_activity_time.get() // Returns Instant
```

### Monitor Events
```javascript
// Add to ActivityTracker for debugging
console.log(`Activity recorded: ${new Date().toISOString()}`);
console.log(`New status: ${ctx.activity_status.get()}`);
```

### Test WebSocket Messages
- Open browser DevTools Network tab
- Filter for WebSocket
- Look for ClientMessage::IdleStatus messages
- Should appear when status changes

## Known Issues

None currently. The implementation is complete and tested.

## Related Documentation

- [Phase 10: UI Indicators](./PHASE10_UI_INDICATORS.md)
- [Phase 3: Message Types](./PHASE_3_MESSAGE_TYPES.md)
- [Protocol Reference](./MESSAGE_CLASSIFICATION_GUIDE.md)