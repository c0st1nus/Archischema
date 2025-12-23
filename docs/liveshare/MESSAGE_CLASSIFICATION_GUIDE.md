# LiveShare Message Classification Guide

## Overview

The LiveShare system uses a message classification framework to prioritize WebSocket messages based on their importance and characteristics. This guide explains how to use the message type system effectively.

## Message Types

### WsMessageType Enum

All WebSocket messages are classified into one of five types:

```rust
pub enum WsMessageType {
    Init,           // Full schema initialization
    Update,         // Incremental schema changes
    CursorMove,     // Cursor position updates
    IdleStatus,     // User activity status
    UserViewport,   // Viewport bounds
}
```

### Priority Levels

Each message type has an associated priority:

```rust
pub enum MessagePriority {
    Volatile,   // Lowest - can be dropped
    Low,        // Can be throttled
    Normal,     // Standard priority
    Critical,   // Highest - must deliver
}
```

## Message Classification

### Critical Messages
**Must be delivered reliably and in order**

- `Init` - Initial schema synchronization
- `Update` - Schema changes (add/delete tables, columns, relationships)
- `Auth` - Authentication
- `GraphOp` - All graph operations
- `SyncStep1`, `SyncStep2` - Yjs synchronization
- `UserJoined`, `UserLeft` - Presence events

**Characteristics:**
- `requires_ordering()` returns `true`
- `is_droppable()` returns `false`
- Should use reliable delivery channel
- Order must be preserved

### Volatile Messages
**Can be dropped under network pressure**

- `CursorMove` - Real-time cursor position

**Characteristics:**
- High frequency updates (60+ per second)
- Latest value is most important
- `is_droppable()` returns `true`
- Can skip frames if network is congested

### Low Priority Messages
**Can be throttled or batched**

- `IdleStatus` - User active/idle state
- `UserViewport` - Viewport center and zoom

**Characteristics:**
- Infrequent updates
- Not time-critical
- Can be batched with other low-priority messages
- Can be rate-limited

## Client-Side Usage

### Sending Messages

```rust
use crate::core::liveshare::protocol::{ClientMessage, WsMessageType, MessagePriority};

// Send cursor update (volatile)
let cursor_msg = ClientMessage::CursorMove {
    position: (mouse_x, mouse_y),
};

// Check if we should send (e.g., under network pressure)
if !network_congested || !cursor_msg.is_droppable() {
    ws_send(cursor_msg);
}

// Send critical schema change
let graph_msg = ClientMessage::GraphOp {
    op: GraphOperation::CreateTable { /* ... */ },
};

// Always send critical messages
assert_eq!(graph_msg.priority(), MessagePriority::Critical);
ws_send_reliable(graph_msg);
```

### Receiving Messages

```rust
// Handle incoming server message
match server_msg {
    ServerMessage::CursorMove { user_id, position } => {
        // Update cursor position in UI
        // This is volatile - if we're busy, we can skip rendering
        if !ui_busy {
            update_remote_cursor(user_id, position);
        }
    }
    
    ServerMessage::GraphOp { user_id, op } => {
        // Critical - must process
        apply_graph_operation(op);
    }
    
    ServerMessage::IdleStatus { user_id, is_active } => {
        // Low priority - can batch UI updates
        batch_ui_update(|| {
            update_user_status(user_id, is_active);
        });
    }
    
    _ => { /* ... */ }
}
```

## Server-Side Usage

### Broadcasting Messages

```rust
use crate::core::liveshare::protocol::ServerMessage;

// Broadcast cursor movement (volatile)
let cursor_msg = ServerMessage::CursorMove {
    user_id,
    position: (x, y),
};

// Can use fire-and-forget broadcast
room.broadcast(cursor_msg);

// Broadcast schema change (critical)
let graph_msg = ServerMessage::GraphOp {
    user_id,
    op,
};

// Ensure reliable delivery
room.broadcast_reliable(graph_msg);
```

### Rate Limiting

```rust
use std::time::{Duration, Instant};

struct CursorThrottle {
    last_send: Instant,
    min_interval: Duration,
}

impl CursorThrottle {
    fn should_send(&mut self, msg: &ClientMessage) -> bool {
        if msg.is_droppable() {
            let now = Instant::now();
            if now.duration_since(self.last_send) >= self.min_interval {
                self.last_send = now;
                true
            } else {
                false
            }
        } else {
            true // Always send critical messages
        }
    }
}
```

## Best Practices

### 1. Cursor Updates

```rust
// ✅ GOOD: Throttle cursor updates
let mut last_cursor_send = Instant::now();
const CURSOR_THROTTLE_MS: u64 = 16; // ~60fps

if last_cursor_send.elapsed().as_millis() >= CURSOR_THROTTLE_MS {
    send(ClientMessage::CursorMove { position });
    last_cursor_send = Instant::now();
}

// ❌ BAD: Send every mouse move event
on_mouse_move(|pos| {
    send(ClientMessage::CursorMove { position: pos });
});
```

### 2. Idle Status

```rust
// ✅ GOOD: Send only on state change
let mut is_active = true;

fn on_activity() {
    if !is_active {
        is_active = true;
        send(ClientMessage::IdleStatus { is_active: true });
    }
}

// ❌ BAD: Send continuously
setInterval(|| {
    send(ClientMessage::IdleStatus { is_active: check_active() });
}, 1000);
```

### 3. Schema Changes

```rust
// ✅ GOOD: Always use GraphOp for schema
send(ClientMessage::GraphOp {
    op: GraphOperation::CreateTable { /* ... */ },
});

// ❌ BAD: Never drop schema changes
if !busy {  // DON'T DO THIS
    send(graph_operation);
}
```

## Message Frequency Guidelines

| Message Type | Recommended Max Frequency | Notes |
|-------------|---------------------------|-------|
| CursorMove | 60 Hz (16ms) | Can be lower on slow networks |
| IdleStatus | On change only | Typically < 1/minute |
| UserViewport | 10 Hz (100ms) | Only during panning/zooming |
| GraphOp | On user action | Typically < 1/second |
| Auth | Once per connection | One-time |

## Network Adaptation

```rust
struct AdaptiveMessaging {
    rtt: Duration,
    packet_loss: f32,
}

impl AdaptiveMessaging {
    fn cursor_throttle_interval(&self) -> Duration {
        match (self.rtt.as_millis(), self.packet_loss) {
            (0..=50, 0.0..=0.01) => Duration::from_millis(16),   // 60fps
            (51..=100, _) => Duration::from_millis(33),          // 30fps
            (101..=200, _) => Duration::from_millis(50),         // 20fps
            _ => Duration::from_millis(100),                      // 10fps
        }
    }
    
    fn should_send_viewport(&self) -> bool {
        self.packet_loss < 0.05  // Skip if >5% loss
    }
}
```

## Testing

```rust
#[test]
fn test_message_priorities() {
    let cursor = ClientMessage::CursorMove { position: (0.0, 0.0) };
    assert!(cursor.is_droppable());
    assert_eq!(cursor.priority(), MessagePriority::Volatile);
    
    let graph_op = ClientMessage::GraphOp { 
        op: GraphOperation::CreateTable { /* ... */ } 
    };
    assert!(!graph_op.is_droppable());
    assert_eq!(graph_op.priority(), MessagePriority::Critical);
}
```

## Future Enhancements

The message classification system enables future optimizations:

1. **Automatic QoS**: Route messages through different channels based on priority
2. **Adaptive Throttling**: Adjust rates based on network conditions
3. **Message Batching**: Combine low-priority messages
4. **Conflict Resolution**: Use priority to resolve concurrent edits
5. **Bandwidth Management**: Shed volatile traffic under congestion

## See Also

- [PHASE_3_MESSAGE_TYPES.md](./PHASE_3_MESSAGE_TYPES.md) - Implementation details
- [protocol.rs](../../src/core/liveshare/protocol.rs) - Message definitions
- [websocket.rs](../../src/core/liveshare/websocket.rs) - Handler implementations