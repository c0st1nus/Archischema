//! LiveShare client for WebSocket communication
//!
//! This module provides the client-side WebSocket connection management
//! for real-time collaboration features.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export protocol types for convenience
pub use crate::core::liveshare::{
    AwarenessState, ClientMessage, ColumnData, GraphOperation, GraphStateSnapshot,
    RelationshipSnapshot, RoomResponse, ServerMessage, TableSnapshot, UserInfo, WsErrorCode,
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

/// Remote user with their awareness state
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteUser {
    pub user_id: UserId,
    pub username: String,
    pub color: String,
    pub cursor: Option<(f64, f64)>,
    pub selected_nodes: Vec<String>,
    pub is_active: bool,
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
        }
    }

    pub fn update_awareness(&mut self, state: &AwarenessState) {
        self.cursor = state.cursor;
        self.selected_nodes = state.selected_nodes.clone();
        self.is_active = state.is_active;
        if let Some(color) = &state.color {
            self.color = color.clone();
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
}

impl LiveShareContext {
    /// Create a new LiveShare context
    pub fn new() -> Self {
        let user_id = Uuid::new_v4();
        let username = format!("User_{}", &user_id.to_string()[..6]);

        Self {
            connection_state: RwSignal::new(ConnectionState::Disconnected),
            user_id: RwSignal::new(user_id),
            username: RwSignal::new(username),
            room_id: RwSignal::new(None),
            room_info: RwSignal::new(None),
            remote_users: RwSignal::new(vec![]),
            error: RwSignal::new(None),
            local_cursor: RwSignal::new(None),
        }
    }

    /// Check if connected to a room
    pub fn is_connected(&self) -> bool {
        self.connection_state.get() == ConnectionState::Connected
    }

    /// Get all users (including self) as DisplayUser
    pub fn get_all_users(&self) -> Vec<DisplayUser> {
        let mut users = vec![DisplayUser {
            user_id: self.user_id.get(),
            username: self.username.get(),
            color: generate_user_color(&self.user_id.get()),
            is_self: true,
        }];

        for remote in self.remote_users.get() {
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
            let auth_msg = ClientMessage::Auth {
                user_id: ctx.user_id.get_untracked(),
                username: ctx.username.get_untracked(),
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
    }

    /// Disconnect stub for SSR
    #[cfg(feature = "ssr")]
    pub fn disconnect(&self) {
        self.connection_state.set(ConnectionState::Disconnected);
        self.room_id.set(None);
        self.room_info.set(None);
        self.remote_users.set(vec![]);
        self.error.set(None);
    }

    /// Send awareness update (cursor position, etc.)
    #[cfg(not(feature = "ssr"))]
    pub fn send_awareness(&self, cursor: Option<(f64, f64)>, selected_nodes: Vec<String>) {
        self.local_cursor.set(cursor);

        if self.connection_state.get() != ConnectionState::Connected {
            return;
        }

        let msg = ClientMessage::Awareness {
            state: AwarenessState {
                cursor,
                selected_nodes,
                color: Some(generate_user_color(&self.user_id.get_untracked())),
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
        if self.connection_state.get() != ConnectionState::Connected {
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

    /// Send graph state response to a specific user
    #[cfg(not(feature = "ssr"))]
    pub fn send_graph_state_response(&self, target_user_id: UserId, state: GraphStateSnapshot) {
        if self.connection_state.get() != ConnectionState::Connected {
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
}

impl Default for LiveShareContext {
    fn default() -> Self {
        Self::new()
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
                        .filter(|u| u.user_id != ctx.user_id.get_untracked())
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
            if user_id != ctx.user_id.get_untracked() {
                ctx.remote_users.update(|users| {
                    // Don't add if already exists
                    if !users.iter().any(|u| u.user_id == user_id) {
                        users.push(RemoteUser::new(user_id, username));
                    }
                });
                // Update room info user count
                ctx.room_info.update(|info| {
                    if let Some(i) = info {
                        i.user_count = ctx.remote_users.get_untracked().len() + 1;
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
                    i.user_count = ctx.remote_users.get_untracked().len() + 1;
                }
            });
        }
        ServerMessage::Awareness { user_id, state } => {
            if user_id != ctx.user_id.get_untracked() {
                ctx.remote_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.user_id == user_id) {
                        user.update_awareness(&state);
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
            let my_id = ctx.user_id.get_untracked();
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
            let my_id = ctx.user_id.get_untracked();
            if target_user_id.is_some() && target_user_id != Some(my_id) {
                // This message is for another user, ignore it
                return;
            }

            leptos::logging::log!("Received GraphState: {} tables", state.tables.len());
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
    let ctx = LiveShareContext::new();
    provide_context(ctx);
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
