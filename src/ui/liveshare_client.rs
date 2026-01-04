//! LiveShare client for WebSocket communication
//!
//! This module provides the client-side WebSocket connection management
//! for real-time collaboration features.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Get current timestamp in milliseconds
/// For WASM, uses js_sys::Date::now()
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Get current timestamp in milliseconds
/// For non-WASM, uses SystemTime
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64
}

// Import auth context for getting username
use crate::ui::auth::use_auth_context;

// Re-export protocol types for convenience
pub use crate::core::liveshare::{
    ActivityStatus, AwarenessState, ClientMessage, ColumnData, GraphOperation, GraphStateSnapshot,
    RelationshipData, RelationshipSnapshot, RoomResponse, ServerMessage, TableSnapshot, UserInfo,
    WsErrorCode,
};

/// User ID type
pub type UserId = Uuid;

/// Connection state for the LiveShare client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

/// Synchronization status for tracking updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncStatus {
    #[default]
    Idle,
    Syncing,
    Synced,
    Error,
    Throttled,
}

/// Remote user with their awareness state
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteUser {
    pub user_id: UserId,
    pub username: String,
    pub color: String,
    pub cursor: Option<(f64, f64)>,
    pub selected_nodes: Vec<String>,
    pub is_active: bool,
    /// Table being dragged by this user (node_id, offset_x, offset_y)
    pub dragging_table: Option<(u32, f64, f64)>,
}

impl RemoteUser {
    pub fn new(user_id: UserId, username: String) -> Self {
        Self {
            user_id,
            username,
            color: generate_user_color(&user_id),
            cursor: None,
            selected_nodes: vec![],
            is_active: true,
            dragging_table: None,
        }
    }

    pub fn update_awareness(&mut self, state: &AwarenessState) {
        self.cursor = state.cursor;
        self.selected_nodes = state.selected_nodes.clone();
        self.is_active = state.is_active;
        if let Some(color) = &state.color {
            self.color = color.clone();
        }
        // Update username if provided in awareness state
        if let Some(username) = &state.username {
            self.username = username.clone();
        }
    }
}

/// Generate a consistent color for a user based on their ID
pub fn generate_user_color(user_id: &UserId) -> String {
    let colors = [
        "#ef4444", // red
        "#f97316", // orange
        "#eab308", // yellow
        "#22c55e", // green
        "#14b8a6", // teal
        "#3b82f6", // blue
        "#8b5cf6", // violet
        "#ec4899", // pink
        "#06b6d4", // cyan
        "#a855f7", // purple
    ];
    let index = (user_id.as_u128() % colors.len() as u128) as usize;
    colors[index].to_string()
}

/// Room information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub is_protected: bool,
    pub user_count: usize,
    pub max_users: usize,
    pub owner_id: UserId,
}

impl From<RoomResponse> for RoomInfo {
    fn from(resp: RoomResponse) -> Self {
        Self {
            id: resp.id.to_string(),
            name: resp.name,
            is_protected: resp.is_protected,
            user_count: resp.user_count,
            max_users: resp.max_users,
            owner_id: resp.owner_id,
        }
    }
}

/// LiveShare context that can be provided to the component tree
#[derive(Clone, Copy)]
pub struct LiveShareContext {
    /// Current connection state
    pub connection_state: RwSignal<ConnectionState>,
    /// Current user's ID
    pub user_id: RwSignal<UserId>,
    /// Current user's name
    pub username: RwSignal<String>,
    /// Current room ID (if connected)
    pub room_id: RwSignal<Option<String>>,
    /// Current room info (if connected)
    pub room_info: RwSignal<Option<RoomInfo>>,
    /// List of remote users in the room
    pub remote_users: RwSignal<Vec<RemoteUser>>,
    /// Error message (if any)
    pub error: RwSignal<Option<String>>,
    /// Local cursor position (to send to others)
    pub local_cursor: RwSignal<Option<(f64, f64)>>,
    /// Pending room to join from URL (e.g., /?room=uuid)
    pub pending_join_room: RwSignal<Option<String>>,
    /// Synchronization status
    pub sync_status: RwSignal<SyncStatus>,
    /// Number of pending updates waiting to be sent
    pub pending_updates: RwSignal<usize>,
    /// Last successful sync time (timestamp in milliseconds)
    pub last_sync_time: RwSignal<Option<f64>>,
    /// Time when connection was lost (for reconnection UI, timestamp in milliseconds)
    pub connection_lost_since: RwSignal<Option<f64>>,
    /// Whether a snapshot is currently being saved
    pub snapshot_saving: RwSignal<bool>,
    /// Last successful snapshot save time (timestamp in milliseconds)
    pub last_snapshot_time: RwSignal<Option<f64>>,
    /// Current user's activity status (Active, Idle, or Away)
    pub activity_status: RwSignal<ActivityStatus>,
    /// Last time activity was detected locally (timestamp in milliseconds)
    pub last_activity_time: RwSignal<f64>,
    /// Whether initial sync from room has been completed
    pub initial_sync_done: RwSignal<bool>,
}

impl LiveShareContext {
    /// Create a new LiveShare context with optional username from auth
    pub fn new(username: Option<String>) -> Self {
        let user_id = Uuid::new_v4();
        let username = username.unwrap_or_else(|| format!("User_{}", &user_id.to_string()[..6]));

        // pending_join_room starts as None - will be set on client-side hydration
        Self {
            connection_state: RwSignal::new(ConnectionState::Disconnected),
            user_id: RwSignal::new(user_id),
            username: RwSignal::new(username),
            room_id: RwSignal::new(None),
            room_info: RwSignal::new(None),
            remote_users: RwSignal::new(vec![]),
            error: RwSignal::new(None),
            local_cursor: RwSignal::new(None),
            pending_join_room: RwSignal::new(None),
            sync_status: RwSignal::new(SyncStatus::Idle),
            pending_updates: RwSignal::new(0),
            last_sync_time: RwSignal::new(None),
            connection_lost_since: RwSignal::new(None),
            snapshot_saving: RwSignal::new(false),
            last_snapshot_time: RwSignal::new(None),
            activity_status: RwSignal::new(ActivityStatus::Active),
            last_activity_time: RwSignal::new(now_ms()),
            initial_sync_done: RwSignal::new(false),
        }
    }

    /// Update sync status
    pub fn set_sync_status(&self, status: SyncStatus) {
        self.sync_status.set(status);
        if status == SyncStatus::Synced {
            self.last_sync_time.set(Some(now_ms()));
            self.pending_updates.set(0);
        }
    }

    /// Increment pending updates counter
    pub fn add_pending_update(&self) {
        self.pending_updates.update(|count| *count += 1);
        if self.sync_status.get_untracked() != SyncStatus::Syncing {
            self.set_sync_status(SyncStatus::Syncing);
        }
    }

    /// Mark connection as lost
    pub fn mark_connection_lost(&self) {
        self.connection_lost_since.set(Some(now_ms()));
    }

    /// Clear connection lost time when reconnected
    pub fn mark_connection_restored(&self) {
        self.connection_lost_since.set(None);
    }

    /// Mark snapshot save started
    pub fn mark_snapshot_save_started(&self) {
        self.snapshot_saving.set(true);
    }

    /// Mark snapshot save completed
    pub fn mark_snapshot_save_completed(&self) {
        self.snapshot_saving.set(false);
        self.last_snapshot_time.set(Some(now_ms()));
    }

    /// Check URL for room parameter and set pending_join_room if found
    /// This should be called on client-side after hydration
    #[cfg(not(feature = "ssr"))]
    pub fn check_url_for_room(&self) {
        if let Some(room_id) = Self::get_room_from_url() {
            self.pending_join_room.set(Some(room_id));
        }
    }

    /// Check URL stub for SSR
    #[cfg(feature = "ssr")]
    pub fn check_url_for_room(&self) {
        // No-op on server
    }

    /// Extract room ID from URL query parameter (?room=uuid)
    #[cfg(not(feature = "ssr"))]
    fn get_room_from_url() -> Option<String> {
        use leptos::web_sys;

        let window = web_sys::window()?;
        let location = window.location();
        let search = location.search().ok()?;

        // Parse query string: ?room=uuid
        if search.starts_with('?') {
            for param in search[1..].split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "room" && !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    }

    /// Check if connected to a room
    pub fn is_connected(&self) -> bool {
        self.connection_state.get_untracked() == ConnectionState::Connected
    }

    /// Get all users (including self) as DisplayUser
    pub fn get_all_users(&self) -> Vec<DisplayUser> {
        let mut users = vec![DisplayUser {
            user_id: self.user_id.get_untracked(),
            username: self.username.get_untracked(),
            color: generate_user_color(&self.user_id.get_untracked()),
            is_self: true,
        }];

        for remote in self.remote_users.get_untracked() {
            users.push(DisplayUser {
                user_id: remote.user_id,
                username: remote.username.clone(),
                color: remote.color.clone(),
                is_self: false,
            });
        }

        users
    }

    /// Connect to a room
    #[cfg(not(feature = "ssr"))]
    pub fn connect(&self, room_id: String, password: Option<String>) {
        use leptos::wasm_bindgen::{JsCast, closure::Closure};
        use leptos::web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

        // Sync username from auth context before connecting
        let auth_ctx = use_auth_context();
        if let Some(user) = auth_ctx.user() {
            let current_username = self.username.with_untracked(|v| v.clone());
            if current_username != user.username {
                leptos::logging::log!(
                    "Syncing username before connect: '{}' -> '{}'",
                    current_username,
                    user.username
                );
                self.username.set(user.username);
            }
        }

        self.connection_state.set(ConnectionState::Connecting);
        self.error.set(None);

        // Build WebSocket URL
        let window = leptos::web_sys::window().expect("no window");
        let location = window.location();
        let protocol = if location.protocol().unwrap_or_default() == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        let host = location
            .host()
            .unwrap_or_else(|_| "localhost:3000".to_string());
        let ws_url = format!("{}//{}/room/{}", protocol, host, room_id);

        // Create WebSocket
        let ws = match WebSocket::new(&ws_url) {
            Ok(ws) => ws,
            Err(e) => {
                self.connection_state.set(ConnectionState::Error);
                self.error
                    .set(Some(format!("Failed to create WebSocket: {:?}", e)));
                return;
            }
        };

        ws.set_binary_type(leptos::web_sys::BinaryType::Arraybuffer);

        // Store context values for closures
        let ctx = *self;
        let password_clone = password.clone();

        // onopen handler
        let ws_clone = ws.clone();
        let onopen = Closure::wrap(Box::new(move |_: leptos::web_sys::Event| {
            // Send auth message
            let username = ctx.username.with_untracked(|v| v.clone());
            leptos::logging::log!("Sending auth with username: {}", username);

            let auth_msg = ClientMessage::Auth {
                user_id: ctx.user_id.with_untracked(|v| *v),
                username,
                password: password_clone.clone(),
            };

            if let Ok(json) = serde_json::to_string(&auth_msg) {
                let _ = ws_clone.send_with_str(&json);
            }
        }) as Box<dyn FnMut(leptos::web_sys::Event)>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // onmessage handler
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(text) = e.data().as_string() {
                handle_message(&ctx, &text);
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // onclose handler
        let onclose = Closure::wrap(Box::new(move |_: CloseEvent| {
            ctx.connection_state.set(ConnectionState::Disconnected);
            ctx.room_id.set(None);
            ctx.room_info.set(None);
            ctx.remote_users.set(vec![]);
            ctx.initial_sync_done.set(false);
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        // onerror handler
        let onerror = Closure::wrap(Box::new(move |_: ErrorEvent| {
            ctx.connection_state.set(ConnectionState::Error);
            ctx.error.set(Some("WebSocket error".to_string()));
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        self.room_id.set(Some(room_id));

        // Store WebSocket in a global variable for sending messages
        store_websocket(ws);
    }

    /// Connect stub for SSR
    #[cfg(feature = "ssr")]
    pub fn connect(&self, _room_id: String, _password: Option<String>) {
        // No-op on server
    }

    /// Disconnect from the room
    #[cfg(not(feature = "ssr"))]
    pub fn disconnect(&self) {
        close_websocket();
        self.connection_state.set(ConnectionState::Disconnected);
        self.room_id.set(None);
        self.room_info.set(None);
        self.remote_users.set(vec![]);
        self.error.set(None);
        self.initial_sync_done.set(false);
    }

    /// Disconnect stub for SSR
    #[cfg(feature = "ssr")]
    pub fn disconnect(&self) {
        self.connection_state.set(ConnectionState::Disconnected);
        self.room_id.set(None);
        self.room_info.set(None);
        self.remote_users.set(vec![]);
        self.initial_sync_done.set(false);
    }

    /// Send awareness update (cursor position, etc.)
    #[cfg(not(feature = "ssr"))]
    pub fn send_awareness(&self, cursor: Option<(f64, f64)>, selected_nodes: Vec<String>) {
        self.local_cursor.set(cursor);

        if self.connection_state.get_untracked() != ConnectionState::Connected {
            return;
        }

        let msg = ClientMessage::Awareness {
            state: AwarenessState {
                username: Some(self.username.with_untracked(|v| v.clone())),
                cursor,
                selected_nodes,
                color: Some(generate_user_color(&self.user_id.with_untracked(|v| *v))),
                is_active: true,
            },
        };

        send_message(&msg);
    }

    /// Send awareness stub for SSR
    #[cfg(feature = "ssr")]
    pub fn send_awareness(&self, cursor: Option<(f64, f64)>, _selected_nodes: Vec<String>) {
        self.local_cursor.set(cursor);
    }

    /// Send a Yjs update to the server
    #[allow(dead_code)]
    pub fn send_update(&self, update: Vec<u8>) {
        let msg = ClientMessage::Update { update };
        send_message(&msg);
    }

    /// Send a graph operation for synchronization
    #[cfg(not(feature = "ssr"))]
    pub fn send_graph_op(&self, op: GraphOperation) {
        if self.connection_state.get_untracked() != ConnectionState::Connected {
            return;
        }
        let msg = ClientMessage::GraphOp { op };
        send_message(&msg);
    }

    /// Send graph operation stub for SSR
    #[cfg(feature = "ssr")]
    pub fn send_graph_op(&self, _op: GraphOperation) {
        // No-op on server
    }

    /// Send table drag start notification
    #[cfg(not(feature = "ssr"))]
    pub fn send_table_drag_start(&self, node_id: u32, offset: (f64, f64)) {
        if self.connection_state.get_untracked() != ConnectionState::Connected {
            return;
        }
        let msg = ClientMessage::TableDragStart { node_id, offset };
        send_message(&msg);
    }

    /// Send table drag start stub for SSR
    #[cfg(feature = "ssr")]
    pub fn send_table_drag_start(&self, _node_id: u32, _offset: (f64, f64)) {
        // No-op on server
    }

    /// Send table drag end notification
    #[cfg(not(feature = "ssr"))]
    pub fn send_table_drag_end(&self, node_id: u32, position: (f64, f64)) {
        if self.connection_state.get_untracked() != ConnectionState::Connected {
            return;
        }
        let msg = ClientMessage::TableDragEnd { node_id, position };
        send_message(&msg);
    }

    /// Send table drag end stub for SSR
    #[cfg(feature = "ssr")]
    pub fn send_table_drag_end(&self, _node_id: u32, _position: (f64, f64)) {
        // No-op on server
    }

    /// Send graph state response to a specific user
    #[cfg(not(feature = "ssr"))]
    pub fn send_graph_state_response(&self, target_user_id: UserId, state: GraphStateSnapshot) {
        if self.connection_state.get_untracked() != ConnectionState::Connected {
            return;
        }
        let msg = ClientMessage::GraphStateResponse {
            target_user_id,
            state,
        };
        send_message(&msg);
    }

    /// Send graph state response stub for SSR
    #[cfg(feature = "ssr")]
    pub fn send_graph_state_response(&self, _target_user_id: UserId, _state: GraphStateSnapshot) {
        // No-op on server
    }

    /// Record user activity and update activity status
    #[cfg(not(feature = "ssr"))]
    pub fn record_activity(&self) {
        self.last_activity_time.set(now_ms());

        // If we were in idle/away state, transition back to active
        if self.activity_status.get_untracked() != ActivityStatus::Active {
            self.activity_status.set(ActivityStatus::Active);
            self.send_idle_status(true);
        }
    }

    /// Record user activity stub for SSR
    #[cfg(feature = "ssr")]
    pub fn record_activity(&self) {
        self.last_activity_time.set(now_ms());
    }

    /// Update activity status based on elapsed time
    /// Returns true if status changed
    #[cfg(not(feature = "ssr"))]
    pub fn update_activity_status(&self) -> bool {
        let now = now_ms();
        let last_activity = self.last_activity_time.with_untracked(|v| *v);
        let elapsed_ms = now - last_activity;

        let idle_threshold_ms = 30_000.0; // 30 seconds
        let away_threshold_ms = 600_000.0; // 10 minutes

        let new_status = if elapsed_ms >= away_threshold_ms {
            ActivityStatus::Away
        } else if elapsed_ms >= idle_threshold_ms {
            ActivityStatus::Idle
        } else {
            ActivityStatus::Active
        };

        let current_status = self.activity_status.with_untracked(|v| *v);
        if new_status != current_status {
            self.activity_status.set(new_status);
            self.send_idle_status(new_status == ActivityStatus::Active);
            return true;
        }

        false
    }

    /// Update activity status stub for SSR
    #[cfg(feature = "ssr")]
    pub fn update_activity_status(&self) -> bool {
        false
    }

    /// Record that page/tab is now hidden
    #[cfg(not(feature = "ssr"))]
    pub fn record_page_hidden(&self) {
        if self.activity_status.get_untracked() != ActivityStatus::Away {
            self.activity_status.set(ActivityStatus::Away);
            self.send_idle_status(false);
        }
    }

    /// Record that page/tab is now hidden stub for SSR
    #[cfg(feature = "ssr")]
    pub fn record_page_hidden(&self) {
        // No-op on server
    }

    /// Record that page/tab is now visible again
    #[cfg(not(feature = "ssr"))]
    pub fn record_page_visible(&self) {
        if self.activity_status.get_untracked() == ActivityStatus::Away {
            self.activity_status.set(ActivityStatus::Active);
            self.last_activity_time.set(now_ms());
            self.send_idle_status(true);
        }
    }

    /// Record that page/tab is now visible again stub for SSR
    #[cfg(feature = "ssr")]
    pub fn record_page_visible(&self) {
        // No-op on server
    }

    /// Send idle status to server
    #[cfg(not(feature = "ssr"))]
    #[allow(dead_code)]
    fn send_idle_status(&self, is_active: bool) {
        if self.connection_state.get_untracked() != ConnectionState::Connected {
            return;
        }

        let msg = ClientMessage::IdleStatus { is_active };
        send_message(&msg);
    }

    /// Send idle status stub for SSR
    #[cfg(feature = "ssr")]
    #[allow(dead_code)]
    fn send_idle_status(&self, _is_active: bool) {
        // No-op on server
    }

    /// Get current activity status display name
    pub fn get_activity_status_display(&self) -> &'static str {
        self.activity_status.get_untracked().display_name()
    }
}

impl Default for LiveShareContext {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Handle incoming WebSocket message
#[cfg(not(feature = "ssr"))]
fn handle_message(ctx: &LiveShareContext, text: &str) {
    let msg: ServerMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            leptos::logging::warn!("Failed to parse message: {}", e);
            return;
        }
    };

    match msg {
        ServerMessage::AuthResult {
            success,
            error,
            room_info,
        } => {
            if success {
                ctx.connection_state.set(ConnectionState::Connected);
                if let Some(info) = room_info {
                    // Add existing users from room info
                    let remote_users: Vec<RemoteUser> = info
                        .users
                        .iter()
                        .filter(|u| u.user_id != ctx.user_id.with_untracked(|v| *v))
                        .map(|u| RemoteUser::new(u.user_id, u.username.clone()))
                        .collect();
                    ctx.remote_users.set(remote_users.clone());
                    ctx.room_info.set(Some(info.into()));

                    // If there are other users, request graph state from them
                    if !remote_users.is_empty() {
                        let msg = ClientMessage::RequestGraphState;
                        send_message(&msg);
                    }
                }
            } else {
                ctx.connection_state.set(ConnectionState::Error);
                ctx.error.set(error);
                ctx.disconnect();
            }
        }
        ServerMessage::UserJoined { user_id, username } => {
            if user_id != ctx.user_id.with_untracked(|v| *v) {
                ctx.remote_users.update(|users| {
                    // Don't add if already exists
                    if !users.iter().any(|u| u.user_id == user_id) {
                        users.push(RemoteUser::new(user_id, username));
                    }
                });
                // Update room info user count
                ctx.room_info.update(|info| {
                    if let Some(i) = info {
                        i.user_count = ctx.remote_users.with_untracked(|v| v.len()) + 1;
                    }
                });
            }
        }
        ServerMessage::UserLeft { user_id } => {
            ctx.remote_users.update(|users| {
                users.retain(|u| u.user_id != user_id);
            });
            ctx.room_info.update(|info| {
                if let Some(i) = info {
                    i.user_count = ctx.remote_users.with_untracked(|v| v.len()) + 1;
                }
            });
        }
        ServerMessage::Awareness { user_id, state } => {
            if user_id != ctx.user_id.with_untracked(|v| *v) {
                ctx.remote_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
                        user.update_awareness(&state);
                    } else {
                        // User not found - they might have joined before we got the UserJoined message
                        // Add them now with username from awareness state or a generic name
                        let username = state
                            .username
                            .clone()
                            .unwrap_or_else(|| format!("User_{}", &user_id.to_string()[..6]));
                        leptos::logging::log!(
                            "Adding unknown user from awareness: {:?} ({})",
                            user_id,
                            username
                        );
                        let mut new_user = RemoteUser::new(user_id, username);
                        new_user.update_awareness(&state);
                        users.push(new_user);
                    }
                });
            }
        }
        ServerMessage::Update { update } => {
            // TODO: Apply Yjs update to local document
            leptos::logging::log!("Received Yjs update: {} bytes", update.len());
        }
        ServerMessage::GraphOp { user_id, op } => {
            // Ignore our own operations - we already applied them locally
            let my_id = ctx.user_id.with_untracked(|v| *v);
            if user_id == my_id {
                return;
            }

            leptos::logging::log!("Received GraphOp from {:?}: {:?}", user_id, op);
            // The actual graph update will be handled by a callback set by the canvas
            // We dispatch a custom event that the canvas can listen to
            #[cfg(not(feature = "ssr"))]
            {
                use leptos::wasm_bindgen::JsValue;
                if let Some(window) = web_sys::window() {
                    let init = web_sys::CustomEventInit::new();
                    init.set_detail(&JsValue::from_str(
                        &serde_json::to_string(&op).unwrap_or_default(),
                    ));
                    if let Ok(event) =
                        web_sys::CustomEvent::new_with_event_init_dict("liveshare-graph-op", &init)
                    {
                        let _ = window.dispatch_event(&event);
                    }
                }
            }
        }
        ServerMessage::GraphState {
            state,
            target_user_id,
        } => {
            // Only process if this message is for us (or for everyone if target is None)
            let my_id = ctx.user_id.with_untracked(|v| *v);
            if target_user_id.is_some() && target_user_id != Some(my_id) {
                // This message is for another user, ignore it
                return;
            }

            // Only apply GraphState if we haven't done initial sync yet
            // This prevents constant re-syncing between clients
            if ctx.initial_sync_done.get_untracked() {
                leptos::logging::log!(
                    "Ignoring GraphState (already synced): {} tables",
                    state.tables.len()
                );
                return;
            }

            leptos::logging::log!("Received GraphState: {} tables", state.tables.len());
            ctx.initial_sync_done.set(true);

            // Dispatch custom event for initial state sync
            #[cfg(not(feature = "ssr"))]
            {
                use leptos::wasm_bindgen::JsValue;
                if let Some(window) = web_sys::window() {
                    let init = web_sys::CustomEventInit::new();
                    init.set_detail(&JsValue::from_str(
                        &serde_json::to_string(&state).unwrap_or_default(),
                    ));
                    if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(
                        "liveshare-graph-state",
                        &init,
                    ) {
                        let _ = window.dispatch_event(&event);
                    }
                }
            }
        }
        ServerMessage::RequestGraphState { requester_id } => {
            // Another user is requesting the graph state - we should send our state
            // Dispatch custom event so canvas can respond with current state
            leptos::logging::log!("Received RequestGraphState from {:?}", requester_id);
            #[cfg(not(feature = "ssr"))]
            {
                use leptos::wasm_bindgen::JsValue;
                if let Some(window) = web_sys::window() {
                    let init = web_sys::CustomEventInit::new();
                    init.set_detail(&JsValue::from_str(&requester_id.to_string()));
                    if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(
                        "liveshare-request-graph-state",
                        &init,
                    ) {
                        let _ = window.dispatch_event(&event);
                    }
                }
            }
        }
        ServerMessage::SyncStep1 { state_vector } => {
            // TODO: Handle sync step 1
            leptos::logging::log!("Received SyncStep1: {} bytes", state_vector.len());
        }
        ServerMessage::SyncStep2 { update } => {
            // TODO: Handle sync step 2
            leptos::logging::log!("Received SyncStep2: {} bytes", update.len());
        }
        ServerMessage::Error { code, message } => {
            leptos::logging::error!("Server error: {:?} - {}", code, message);
            ctx.error.set(Some(message));
        }
        ServerMessage::Pong => {
            // Keepalive response, ignore
        }
        ServerMessage::CursorMove { user_id, position } => {
            // Update remote user cursor position
            if user_id != ctx.user_id.with_untracked(|v| *v) {
                ctx.remote_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
                        user.cursor = Some(position);
                    }
                });
            }
        }
        ServerMessage::IdleStatus { user_id, is_active } => {
            // Update remote user activity status
            if user_id != ctx.user_id.with_untracked(|v| *v) {
                ctx.remote_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
                        user.is_active = is_active;
                    }
                });
            }
        }
        ServerMessage::UserViewport {
            user_id: _,
            center: _,
            zoom: _,
        } => {
            // TODO: Handle user viewport updates for optimization
            // This would be used to only send updates for visible elements
        }
        ServerMessage::TableDragStart {
            user_id,
            node_id,
            offset,
        } => {
            // Update remote user's dragging state
            if user_id != ctx.user_id.with_untracked(|v| *v) {
                ctx.remote_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
                        user.dragging_table = Some((node_id, offset.0, offset.1));
                    }
                });

                // Dispatch event for canvas to handle visual feedback
                #[cfg(not(feature = "ssr"))]
                {
                    use leptos::wasm_bindgen::JsValue;
                    if let Some(window) = web_sys::window() {
                        let init = web_sys::CustomEventInit::new();
                        let detail = serde_json::json!({
                            "user_id": user_id.to_string(),
                            "node_id": node_id,
                            "offset": offset,
                        });
                        init.set_detail(&JsValue::from_str(&detail.to_string()));
                        if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(
                            "liveshare-table-drag-start",
                            &init,
                        ) {
                            let _ = window.dispatch_event(&event);
                        }
                    }
                }
            }
        }
        ServerMessage::TableDragEnd {
            user_id,
            node_id,
            position,
        } => {
            // Clear remote user's dragging state
            if user_id != ctx.user_id.with_untracked(|v| *v) {
                ctx.remote_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
                        user.dragging_table = None;
                    }
                });

                // Dispatch event for canvas to update final position
                #[cfg(not(feature = "ssr"))]
                {
                    use leptos::wasm_bindgen::JsValue;
                    if let Some(window) = web_sys::window() {
                        let init = web_sys::CustomEventInit::new();
                        let detail = serde_json::json!({
                            "user_id": user_id.to_string(),
                            "node_id": node_id,
                            "position": position,
                        });
                        init.set_detail(&JsValue::from_str(&detail.to_string()));
                        if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(
                            "liveshare-table-drag-end",
                            &init,
                        ) {
                            let _ = window.dispatch_event(&event);
                        }
                    }
                }
            }
        }
        ServerMessage::SnapshotRecovery {
            snapshot_id,
            snapshot_data,
            element_count,
            created_at: _,
        } => {
            // TODO: Handle snapshot recovery
            leptos::logging::log!(
                "Received snapshot recovery: id={}, {} elements, {} bytes",
                snapshot_id,
                element_count,
                snapshot_data.len()
            );
        }
    }
}

// Global WebSocket storage using thread_local
#[cfg(not(feature = "ssr"))]
thread_local! {
    static WEBSOCKET: std::cell::RefCell<Option<leptos::web_sys::WebSocket>> = std::cell::RefCell::new(None);
}

#[cfg(not(feature = "ssr"))]
fn store_websocket(ws: leptos::web_sys::WebSocket) {
    WEBSOCKET.with(|cell| {
        *cell.borrow_mut() = Some(ws);
    });
}

#[cfg(not(feature = "ssr"))]
fn close_websocket() {
    WEBSOCKET.with(|cell| {
        if let Some(ws) = cell.borrow_mut().take() {
            let _ = ws.close();
        }
    });
}

#[cfg(not(feature = "ssr"))]
fn send_message(msg: &ClientMessage) {
    WEBSOCKET.with(|cell| {
        if let Some(ref ws) = *cell.borrow() {
            if ws.ready_state() == leptos::web_sys::WebSocket::OPEN {
                if let Ok(json) = serde_json::to_string(msg) {
                    let _ = ws.send_with_str(&json);
                }
            }
        }
    });
}

#[cfg(feature = "ssr")]
fn send_message(_msg: &ClientMessage) {
    // No-op on server
}

/// User for display purposes
#[derive(Debug, Clone)]
pub struct DisplayUser {
    pub user_id: UserId,
    pub username: String,
    pub color: String,
    pub is_self: bool,
}

/// Provide LiveShare context to the component tree
pub fn provide_liveshare_context() -> LiveShareContext {
    let auth_ctx = use_auth_context();

    // Try to get username from auth context if user is authenticated
    let username = auth_ctx.user().map(|user| {
        leptos::logging::log!(
            "Initializing LiveShare with authenticated user: {}",
            user.username
        );
        user.username
    });

    if username.is_none() {
        leptos::logging::log!(
            "Initializing LiveShare without authenticated user (will use generated ID)"
        );
    }

    let ctx = LiveShareContext::new(username);
    provide_context(ctx);

    // Reactively update username when auth state changes
    let ctx_for_effect = ctx;
    Effect::new(move |_| {
        let new_username = auth_ctx
            .user()
            .map(|user| user.username)
            .unwrap_or_else(|| {
                let user_id = ctx_for_effect.user_id.with_untracked(|v| *v);
                format!("User_{}", &user_id.to_string()[..6])
            });

        // Only update if username actually changed
        let old_username = ctx_for_effect.username.with_untracked(|v| v.clone());
        if old_username != new_username {
            leptos::logging::log!(
                "Updating username from '{}' to '{}'",
                old_username,
                new_username
            );
            ctx_for_effect.username.set(new_username);
        }
    });

    // Check URL for room parameter on client-side (handles invite links)
    #[cfg(not(feature = "ssr"))]
    {
        ctx.check_url_for_room();
    }

    ctx
}

/// Use the LiveShare context from the component tree
pub fn use_liveshare_context() -> LiveShareContext {
    expect_context::<LiveShareContext>()
}

/// Try to get LiveShare context (returns None if not provided)
#[allow(dead_code)]
pub fn try_use_liveshare_context() -> Option<LiveShareContext> {
    use_context::<LiveShareContext>()
}
