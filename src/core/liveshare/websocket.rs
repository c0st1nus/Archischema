//! WebSocket handler for LiveShare real-time synchronization
//!
//! This module provides WebSocket endpoint handlers for:
//! - Real-time Yjs document synchronization
//! - User awareness (cursors, selections)
//! - Room presence (join/leave notifications)
//!
//! WebSocket URL: ws(s)://{host}/room/{room_id}

use std::sync::Arc;

use axum::{
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::api::LiveshareState;
use super::cursor_broadcaster::CursorBroadcaster;
use super::protocol::*;
use super::rate_limiter::MessageRateLimiter;
use super::room::Room;
use super::throttling::{AwarenessBatcher, SchemaThrottler};

// ============================================================================
// Constants
// ============================================================================

/// Channel buffer size for outgoing messages
const OUTGOING_BUFFER_SIZE: usize = 64;

/// Timeout for authentication (in seconds)
#[allow(dead_code)]
const AUTH_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// WebSocket Handler
// ============================================================================

/// WebSocket upgrade handler
///
/// Upgrades HTTP connection to WebSocket for the specified room.
/// The room must exist before connecting.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<Uuid>,
    State(state): State<LiveshareState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, room_id, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, room_id: Uuid, state: LiveshareState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(OUTGOING_BUFFER_SIZE);

    // Spawn task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if ws_sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Connection state
    let mut session = ConnectionSession::new(room_id, tx.clone());

    // Create periodic timer for batching and throttling checks
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(50));

    // Process incoming messages and periodic tasks
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            result = ws_receiver.next() => {
                let Some(result) = result else {
                    // Connection closed
                    break;
                };
        match result {
            Ok(Message::Text(text)) => {
                let text_str: &str = &text;
                match serde_json::from_str::<ClientMessage>(text_str) {
                    Ok(client_msg) => {
                        if let Err(e) = session.handle_message(client_msg, &state).await {
                            tracing::error!("Error handling message: {}", e);
                            let _ = tx
                                .send(ServerMessage::error(WsErrorCode::InternalError, e))
                                .await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Invalid message format: {}", e);
                        let _ = tx
                            .send(ServerMessage::error(
                                WsErrorCode::InvalidMessage,
                                format!("Invalid message format: {}", e),
                            ))
                            .await;
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                // Handle binary Yjs updates directly (more efficient than JSON)
                if session.is_authenticated()
                    && let Some(ref room) = session.room
                {
                    if let Err(e) = room.apply_update(&data).await {
                        tracing::error!("Failed to apply binary update: {}", e);
                    } else {
                        // Broadcast to other clients
                        room.broadcast(ServerMessage::Update {
                            update: data.to_vec(),
                        });
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                // Axum handles Pong automatically, but we can log it
                tracing::trace!("Received ping: {:?}", data);
            }
            Ok(Message::Pong(_)) => {
                // Pong received
                tracing::trace!("Received pong");
            }
            Ok(Message::Close(_)) => {
                tracing::info!(
                    room_id = %room_id,
                    user_id = ?session.user_id,
                    "Client closed connection"
                );
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
                }
            }
            // Handle periodic tasks (batching, throttling)
            _ = interval.tick() => {
                if let Err(e) = session.check_pending_updates().await {
                    tracing::debug!("Error checking pending updates: {}", e);
                }
            }
        }
    }

    // Cleanup: remove user from room
    session.cleanup();

    // Abort send task
    send_task.abort();

    tracing::info!(
        room_id = %room_id,
        user_id = ?session.user_id,
        "WebSocket connection closed"
    );
}

// ============================================================================
// Connection Session
// ============================================================================

/// State for a single WebSocket connection
struct ConnectionSession {
    /// Room ID this connection is for
    room_id: Uuid,
    /// User ID (set after authentication)
    user_id: Option<UserId>,
    /// Username (set after authentication)
    username: Option<String>,
    /// Whether the user has authenticated
    authenticated: bool,
    /// Reference to the room (set after authentication)
    room: Option<Arc<Room>>,
    /// Channel for sending messages to this client
    tx: mpsc::Sender<ServerMessage>,
    /// Broadcast receiver task handle
    broadcast_task: Option<tokio::task::JoinHandle<()>>,
    /// Rate limiter for incoming messages
    rate_limiter: MessageRateLimiter,
    /// Cursor broadcaster with throttling
    cursor_broadcaster: CursorBroadcaster,
    /// Schema update throttler
    schema_throttler: SchemaThrottler,
    /// Awareness batcher
    awareness_batcher: AwarenessBatcher,
}

impl ConnectionSession {
    fn new(room_id: Uuid, tx: mpsc::Sender<ServerMessage>) -> Self {
        Self {
            room_id,
            user_id: None,
            username: None,
            authenticated: false,
            room: None,
            tx,
            broadcast_task: None,
            rate_limiter: MessageRateLimiter::new(),
            cursor_broadcaster: CursorBroadcaster::new(),
            schema_throttler: SchemaThrottler::new(),
            awareness_batcher: AwarenessBatcher::new(),
        }
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Handle an incoming client message
    async fn handle_message(
        &mut self,
        msg: ClientMessage,
        state: &LiveshareState,
    ) -> Result<(), String> {
        // Rate limit check based on message priority
        let priority = msg.priority();
        let rate_limit_ok = match priority {
            MessagePriority::Volatile => self.rate_limiter.check_volatile(),
            MessagePriority::Low => self.rate_limiter.check_normal(),
            MessagePriority::Normal => self.rate_limiter.check_normal(),
            MessagePriority::Critical => self.rate_limiter.check_critical(),
        };

        if !rate_limit_ok {
            return Err("Rate limit exceeded".to_string());
        }

        match msg {
            ClientMessage::Auth {
                user_id,
                username,
                password,
            } => self.handle_auth(user_id, username, password, state).await,

            ClientMessage::SyncStep1 { state_vector } => {
                self.require_auth()?;
                self.handle_sync_step1(state_vector).await
            }

            ClientMessage::SyncStep2 { update } => {
                self.require_auth()?;
                self.handle_sync_step2(update).await
            }

            ClientMessage::Update { update } => {
                self.require_auth()?;
                self.handle_update(update).await
            }

            ClientMessage::Awareness { state } => {
                self.require_auth()?;
                self.handle_awareness(state).await
            }

            ClientMessage::GraphOp { op } => {
                self.require_auth()?;
                self.handle_graph_op(op).await
            }

            ClientMessage::CursorMove { position } => {
                self.require_auth()?;
                self.handle_cursor_move(position).await
            }

            ClientMessage::IdleStatus { is_active } => {
                self.require_auth()?;
                self.handle_idle_status(is_active).await
            }

            ClientMessage::UserViewport { center, zoom } => {
                self.require_auth()?;
                self.handle_user_viewport(center, zoom).await
            }

            ClientMessage::RequestGraphState => {
                self.require_auth()?;
                self.handle_request_graph_state().await
            }

            ClientMessage::GraphStateResponse {
                target_user_id,
                state,
            } => {
                self.require_auth()?;
                self.handle_graph_state_response(target_user_id, state)
                    .await
            }

            ClientMessage::Ping => {
                let _ = self.tx.send(ServerMessage::Pong).await;
                Ok(())
            }
        }
    }

    /// Require authentication before processing a message
    fn require_auth(&self) -> Result<(), String> {
        if !self.authenticated {
            Err("Not authenticated".to_string())
        } else {
            Ok(())
        }
    }

    /// Handle graph operation - broadcast to all other users in room
    async fn handle_graph_op(&mut self, op: super::protocol::GraphOperation) -> Result<(), String> {
        if let Some(ref room) = self.room
            && let Some(user_id) = self.user_id
        {
            // Apply schema throttling for graph operations
            if self.schema_throttler.should_send() {
                room.broadcast(ServerMessage::GraphOp { user_id, op });
                self.schema_throttler.mark_sent();
            }
            // If throttled, the update is silently dropped
            // This is acceptable for high-frequency operations
        }
        Ok(())
    }

    /// Handle request for full graph state
    /// Broadcasts a request to all other users in the room - they should respond with their graph state
    async fn handle_request_graph_state(&self) -> Result<(), String> {
        if let Some(ref room) = self.room
            && let Some(user_id) = self.user_id
        {
            // First, send what the server currently knows
            let current_state = room.get_state().await;
            if !current_state.tables.is_empty() {
                let _ = self
                    .tx
                    .send(ServerMessage::GraphState {
                        state: current_state,
                        target_user_id: Some(user_id),
                    })
                    .await;
            }

            // Also broadcast request to all other users to get the most up-to-date state
            room.broadcast(ServerMessage::RequestGraphState {
                requester_id: user_id,
            });
        }
        Ok(())
    }

    /// Handle cursor move - broadcast to all other users (volatile)
    async fn handle_cursor_move(&mut self, position: (f64, f64)) -> Result<(), String> {
        if let Some(ref room) = self.room
            && let Some(user_id) = self.user_id
        {
            // Apply cursor throttling
            if let Some(throttled_position) = self
                .cursor_broadcaster
                .update_position(position.0, position.1)
            {
                // Broadcast cursor position to all other clients
                room.broadcast(ServerMessage::CursorMove {
                    user_id,
                    position: throttled_position,
                });
            }
        }
        Ok(())
    }

    /// Handle idle status update - broadcast to all other users
    async fn handle_idle_status(&self, is_active: bool) -> Result<(), String> {
        if let Some(ref room) = self.room
            && let Some(user_id) = self.user_id
        {
            // Broadcast idle status to all other clients
            room.broadcast(ServerMessage::IdleStatus { user_id, is_active });
        }
        Ok(())
    }

    /// Handle user viewport update - broadcast to all other users
    async fn handle_user_viewport(&self, center: (f64, f64), zoom: f64) -> Result<(), String> {
        if let Some(ref room) = self.room
            && let Some(user_id) = self.user_id
        {
            // Broadcast viewport to all other clients
            room.broadcast(ServerMessage::UserViewport {
                user_id,
                center,
                zoom,
            });
        }
        Ok(())
    }

    /// Handle graph state response - broadcast with target_user_id so only that user processes it
    async fn handle_graph_state_response(
        &self,
        target_user_id: super::protocol::UserId,
        state: super::protocol::GraphStateSnapshot,
    ) -> Result<(), String> {
        if let Some(ref room) = self.room {
            // Broadcast with target_user_id - only that user should process this message
            room.broadcast(ServerMessage::GraphState {
                state,
                target_user_id: Some(target_user_id),
            });
        }
        Ok(())
    }

    /// Handle authentication message
    async fn handle_auth(
        &mut self,
        user_id: UserId,
        username: String,
        password: Option<String>,
        state: &LiveshareState,
    ) -> Result<(), String> {
        // Get the room
        let room = match state.room_manager.get_room(&self.room_id) {
            Some(r) => r,
            None => {
                let _ = self
                    .tx
                    .send(ServerMessage::auth_failed("Room not found"))
                    .await;
                return Ok(());
            }
        };

        // Check if room is full
        if room.is_full() {
            let _ = self
                .tx
                .send(ServerMessage::auth_failed("Room is full"))
                .await;
            return Ok(());
        }

        // Verify password
        if !room.verify_password(password.as_deref()) {
            let _ = self
                .tx
                .send(ServerMessage::auth_failed("Invalid password"))
                .await;
            return Ok(());
        }

        // Add user to room
        if let Err(e) = room.add_user(user_id, username.clone()) {
            let _ = self
                .tx
                .send(ServerMessage::auth_failed(format!("{:?}", e)))
                .await;
            return Ok(());
        }

        // Update session state
        self.user_id = Some(user_id);
        self.username = Some(username);
        self.authenticated = true;
        self.room = Some(room.clone());

        // Subscribe to room broadcasts
        let mut broadcast_rx = room.subscribe();
        let tx = self.tx.clone();
        let my_user_id = user_id;

        self.broadcast_task = Some(tokio::spawn(async move {
            while let Ok(msg) = broadcast_rx.recv().await {
                // Don't send user's own messages back to them (for some message types)
                let should_send = match &msg {
                    ServerMessage::Awareness { user_id, .. } => *user_id != my_user_id,
                    _ => true,
                };

                if should_send && tx.send(msg).await.is_err() {
                    break;
                }
            }
        }));

        // Send success response with room info
        let room_info = room.to_response(state.room_manager.host(), state.room_manager.is_secure());
        let _ = self.tx.send(ServerMessage::auth_success(room_info)).await;

        tracing::info!(
            room_id = %self.room_id,
            user_id = %user_id,
            "User authenticated and joined room"
        );

        Ok(())
    }

    /// Handle Yjs sync step 1 (client sends state vector)
    async fn handle_sync_step1(&self, client_state_vector: Vec<u8>) -> Result<(), String> {
        let room = self.room.as_ref().ok_or("No room")?;

        // Send server's state vector to client
        let server_sv = room.get_state_vector().await;
        let _ = self
            .tx
            .send(ServerMessage::SyncStep1 {
                state_vector: server_sv,
            })
            .await;

        // Send update to client based on their state vector
        if let Some(update) = room.get_update_from_sv(&client_state_vector).await {
            let _ = self.tx.send(ServerMessage::SyncStep2 { update }).await;
        }

        Ok(())
    }

    /// Handle Yjs sync step 2 (client sends update)
    async fn handle_sync_step2(&self, update: Vec<u8>) -> Result<(), String> {
        let room = self.room.as_ref().ok_or("No room")?;

        // Apply update to document
        room.apply_update(&update).await?;

        // Broadcast update to other clients
        room.broadcast(ServerMessage::Update { update });

        Ok(())
    }

    /// Handle incremental Yjs update
    async fn handle_update(&self, update: Vec<u8>) -> Result<(), String> {
        let room = self.room.as_ref().ok_or("No room")?;

        // Apply update to document
        room.apply_update(&update).await?;

        // Broadcast update to other clients
        room.broadcast(ServerMessage::Update { update });

        Ok(())
    }

    /// Handle awareness update (cursor, selection, etc.)
    async fn handle_awareness(&mut self, state: AwarenessState) -> Result<(), String> {
        let user_id = self.user_id.ok_or("No user ID")?;

        // Add to batcher instead of sending immediately
        let state_json = serde_json::to_value(&state).map_err(|e| e.to_string())?;
        self.awareness_batcher.add(user_id.to_string(), state_json);

        // Update room awareness state
        if let Some(ref room) = self.room {
            room.update_awareness(&user_id, state);
        }

        Ok(())
    }

    /// Check and flush pending updates (called periodically)
    async fn check_pending_updates(&mut self) -> Result<(), String> {
        // Check cursor pending
        if let Some(position) = self.cursor_broadcaster.check_pending()
            && let (Some(room), Some(user_id)) = (&self.room, self.user_id)
        {
            room.broadcast(ServerMessage::CursorMove { user_id, position });
        }

        // Check awareness batch
        if self.awareness_batcher.should_flush() {
            let batch = self.awareness_batcher.flush();
            if let Some(ref room) = self.room {
                for (user_id_str, state_json) in batch {
                    if let Ok(state) = serde_json::from_value::<AwarenessState>(state_json) {
                        // Convert user_id back to Uuid
                        if let Ok(user_id) = uuid::Uuid::parse_str(&user_id_str) {
                            room.broadcast(ServerMessage::Awareness { user_id, state });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Cleanup when connection closes
    fn cleanup(&mut self) {
        // Remove user from room
        if let (Some(room), Some(user_id)) = (&self.room, &self.user_id) {
            room.remove_user(user_id);
        }

        // Abort broadcast task
        if let Some(task) = self.broadcast_task.take() {
            task.abort();
        }
    }
}

impl Drop for ConnectionSession {
    fn drop(&mut self) {
        // Ensure cleanup happens on drop
        self.cleanup();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ConnectionSession Tests
    // ========================================================================

    #[test]
    fn test_connection_session_initial_state() {
        let (tx, _rx) = mpsc::channel(16);
        let session = ConnectionSession::new(Uuid::new_v4(), tx);

        assert!(!session.is_authenticated());
        assert!(session.user_id.is_none());
        assert!(session.room.is_none());
    }

    #[test]
    fn test_require_auth_fails_when_not_authenticated() {
        let (tx, _rx) = mpsc::channel(16);
        let session = ConnectionSession::new(Uuid::new_v4(), tx);

        assert!(session.require_auth().is_err());
    }

    #[test]
    fn test_connection_session_room_id_stored() {
        let (tx, _rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();
        let session = ConnectionSession::new(room_id, tx);

        assert_eq!(session.room_id, room_id);
    }

    #[test]
    fn test_connection_session_username_initially_none() {
        let (tx, _rx) = mpsc::channel(16);
        let session = ConnectionSession::new(Uuid::new_v4(), tx);

        assert!(session.username.is_none());
    }

    #[test]
    fn test_connection_session_broadcast_task_initially_none() {
        let (tx, _rx) = mpsc::channel(16);
        let session = ConnectionSession::new(Uuid::new_v4(), tx);

        assert!(session.broadcast_task.is_none());
    }

    #[test]
    fn test_require_auth_error_message() {
        let (tx, _rx) = mpsc::channel(16);
        let session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session.require_auth();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not authenticated");
    }

    #[test]
    fn test_is_authenticated_false_by_default() {
        let (tx, _rx) = mpsc::channel(16);
        let session = ConnectionSession::new(Uuid::new_v4(), tx);

        assert!(!session.is_authenticated());
        assert!(!session.authenticated);
    }

    // ========================================================================
    // Constants Tests
    // ========================================================================

    #[test]
    fn test_outgoing_buffer_size() {
        assert_eq!(OUTGOING_BUFFER_SIZE, 64);
    }

    #[test]
    fn test_auth_timeout_secs() {
        assert_eq!(AUTH_TIMEOUT_SECS, 30);
    }

    // ========================================================================
    // Message Handling Tests (unit tests for helper methods)
    // ========================================================================

    #[tokio::test]
    async fn test_handle_ping_sends_pong() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        // Set authenticated to bypass auth check for Ping
        // Note: Ping doesn't require auth in the actual implementation
        let result = session
            .handle_message(ClientMessage::Ping, &LiveshareState::new())
            .await;

        assert!(result.is_ok());

        // Check that Pong was sent
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        assert!(matches!(msg.unwrap(), ServerMessage::Pong));
    }

    #[tokio::test]
    async fn test_handle_sync_step1_requires_auth() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(
                ClientMessage::SyncStep1 {
                    state_vector: vec![1, 2, 3],
                },
                &LiveshareState::new(),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not authenticated"));
    }

    #[tokio::test]
    async fn test_handle_sync_step2_requires_auth() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(
                ClientMessage::SyncStep2 {
                    update: vec![1, 2, 3],
                },
                &LiveshareState::new(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_update_requires_auth() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(
                ClientMessage::Update {
                    update: vec![1, 2, 3],
                },
                &LiveshareState::new(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_awareness_requires_auth() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(
                ClientMessage::Awareness {
                    state: AwarenessState::default(),
                },
                &LiveshareState::new(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_graph_op_requires_auth() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(
                ClientMessage::GraphOp {
                    op: GraphOperation::CreateTable {
                        node_id: 1,
                        name: "test".to_string(),
                        position: (0.0, 0.0),
                    },
                },
                &LiveshareState::new(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_request_graph_state_requires_auth() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(ClientMessage::RequestGraphState, &LiveshareState::new())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_auth_room_not_found() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        let result = session
            .handle_message(
                ClientMessage::Auth {
                    user_id: Uuid::new_v4(),
                    username: "TestUser".to_string(),
                    password: None,
                },
                &LiveshareState::new(),
            )
            .await;

        // Should return Ok (error is sent via channel, not returned)
        assert!(result.is_ok());

        // Check that auth failed message was sent
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        match msg.unwrap() {
            ServerMessage::AuthResult { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert!(error.unwrap().contains("not found"));
            }
            _ => panic!("Expected AuthResult message"),
        }
    }

    #[tokio::test]
    async fn test_handle_auth_success() {
        let (tx, mut rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);

        // Create a state with a room
        let state = LiveshareState::new();
        let owner = super::super::auth::AuthenticatedUser::guest();
        state
            .room_manager
            .create_room_with_id(
                room_id,
                &owner,
                CreateRoomRequest {
                    name: Some("Test Room".to_string()),
                    password: None,
                    max_users: None,
                },
            )
            .unwrap();

        let user_id = Uuid::new_v4();
        let result = session
            .handle_message(
                ClientMessage::Auth {
                    user_id,
                    username: "TestUser".to_string(),
                    password: None,
                },
                &state,
            )
            .await;

        assert!(result.is_ok());
        assert!(session.is_authenticated());
        assert_eq!(session.user_id, Some(user_id));
        assert_eq!(session.username, Some("TestUser".to_string()));
        assert!(session.room.is_some());

        // Check that auth success message was sent
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        match msg.unwrap() {
            ServerMessage::AuthResult {
                success, room_info, ..
            } => {
                assert!(success);
                assert!(room_info.is_some());
            }
            _ => panic!("Expected AuthResult message"),
        }
    }

    #[tokio::test]
    async fn test_handle_auth_wrong_password() {
        let (tx, mut rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);

        // Create a password-protected room
        let state = LiveshareState::new();
        let owner = super::super::auth::AuthenticatedUser::guest();
        state
            .room_manager
            .create_room_with_id(
                room_id,
                &owner,
                CreateRoomRequest {
                    name: Some("Protected Room".to_string()),
                    password: Some("secret123".to_string()),
                    max_users: None,
                },
            )
            .unwrap();

        let result = session
            .handle_message(
                ClientMessage::Auth {
                    user_id: Uuid::new_v4(),
                    username: "TestUser".to_string(),
                    password: Some("wrong_password".to_string()),
                },
                &state,
            )
            .await;

        assert!(result.is_ok());
        assert!(!session.is_authenticated());

        // Check that auth failed message was sent
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        match msg.unwrap() {
            ServerMessage::AuthResult { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert!(error.unwrap().contains("password"));
            }
            _ => panic!("Expected AuthResult message"),
        }
    }

    #[tokio::test]
    async fn test_handle_auth_room_full() {
        let (tx, mut rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);

        // Create a room with max_users = 1
        let state = LiveshareState::new();
        let owner = super::super::auth::AuthenticatedUser::guest();
        let room = state
            .room_manager
            .create_room_with_id(
                room_id,
                &owner,
                CreateRoomRequest {
                    name: Some("Small Room".to_string()),
                    password: None,
                    max_users: Some(1),
                },
            )
            .unwrap();

        // Fill the room
        room.add_user(Uuid::new_v4(), "User1".to_string()).unwrap();

        // Try to join
        let result = session
            .handle_message(
                ClientMessage::Auth {
                    user_id: Uuid::new_v4(),
                    username: "TestUser".to_string(),
                    password: None,
                },
                &state,
            )
            .await;

        assert!(result.is_ok());
        assert!(!session.is_authenticated());

        // Check that auth failed message was sent
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        match msg.unwrap() {
            ServerMessage::AuthResult { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert!(error.unwrap().contains("full"));
            }
            _ => panic!("Expected AuthResult message"),
        }
    }

    // ========================================================================
    // Cleanup Tests
    // ========================================================================

    #[test]
    fn test_cleanup_removes_user_from_room() {
        let (tx, _rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);

        // Create a room and add user manually
        let state = LiveshareState::new();
        let owner = super::super::auth::AuthenticatedUser::guest();
        let room = state
            .room_manager
            .create_room_with_id(
                room_id,
                &owner,
                CreateRoomRequest {
                    name: None,
                    password: None,
                    max_users: None,
                },
            )
            .unwrap();

        let user_id = Uuid::new_v4();
        room.add_user(user_id, "TestUser".to_string()).unwrap();

        // Set session state as if authenticated
        session.user_id = Some(user_id);
        session.room = Some(room.clone());
        session.authenticated = true;

        assert_eq!(room.user_count(), 1);

        // Cleanup should remove user
        session.cleanup();

        assert_eq!(room.user_count(), 0);
    }

    #[test]
    fn test_cleanup_without_room() {
        let (tx, _rx) = mpsc::channel(16);
        let mut session = ConnectionSession::new(Uuid::new_v4(), tx);

        // Cleanup should not panic when no room is set
        session.cleanup();

        assert!(session.room.is_none());
    }

    #[test]
    fn test_drop_calls_cleanup() {
        let (tx, _rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();

        // Create a room
        let state = LiveshareState::new();
        let owner = super::super::auth::AuthenticatedUser::guest();
        let room = state
            .room_manager
            .create_room_with_id(
                room_id,
                &owner,
                CreateRoomRequest {
                    name: None,
                    password: None,
                    max_users: None,
                },
            )
            .unwrap();

        let user_id = Uuid::new_v4();
        room.add_user(user_id, "TestUser".to_string()).unwrap();
        assert_eq!(room.user_count(), 1);

        {
            let mut session = ConnectionSession::new(room_id, tx);
            session.user_id = Some(user_id);
            session.room = Some(room.clone());
            session.authenticated = true;
            // session drops here
        }

        // User should be removed after drop
        assert_eq!(room.user_count(), 0);
    }

    // ========================================================================
    // Integration-style Tests
    // ========================================================================

    #[tokio::test]
    async fn test_full_auth_and_sync_flow() {
        let (tx, mut rx) = mpsc::channel(16);
        let room_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);

        // Create room
        let state = LiveshareState::new();
        let owner = super::super::auth::AuthenticatedUser::guest();
        state
            .room_manager
            .create_room_with_id(
                room_id,
                &owner,
                CreateRoomRequest {
                    name: Some("Sync Test Room".to_string()),
                    password: None,
                    max_users: None,
                },
            )
            .unwrap();

        // Authenticate
        session
            .handle_message(
                ClientMessage::Auth {
                    user_id: Uuid::new_v4(),
                    username: "SyncUser".to_string(),
                    password: None,
                },
                &state,
            )
            .await
            .unwrap();

        assert!(session.is_authenticated());

        // Drain auth result
        let _ = rx.try_recv();

        // Now sync step 1 should work
        let result = session
            .handle_message(
                ClientMessage::SyncStep1 {
                    state_vector: vec![],
                },
                &state,
            )
            .await;

        assert!(result.is_ok());

        // Should receive sync messages
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        assert!(matches!(msg.unwrap(), ServerMessage::SyncStep1 { .. }));
    }

    #[tokio::test]
    async fn test_handle_cursor_move_requires_auth() {
        let (tx, _rx) = mpsc::channel(10);
        let room_id = Uuid::new_v4();
        let session = ConnectionSession::new(room_id, tx);

        // Session should not be authenticated by default
        assert!(!session.is_authenticated());
        assert!(session.require_auth().is_err());
    }

    #[tokio::test]
    async fn test_handle_idle_status_requires_auth() {
        let (tx, _rx) = mpsc::channel(10);
        let room_id = Uuid::new_v4();
        let session = ConnectionSession::new(room_id, tx);

        // Session should not be authenticated by default
        assert!(!session.is_authenticated());
        assert!(session.require_auth().is_err());
    }

    #[tokio::test]
    async fn test_handle_user_viewport_requires_auth() {
        let (tx, _rx) = mpsc::channel(10);
        let room_id = Uuid::new_v4();
        let session = ConnectionSession::new(room_id, tx);

        // Session should not be authenticated by default
        assert!(!session.is_authenticated());
        assert!(session.require_auth().is_err());
    }

    #[tokio::test]
    async fn test_handle_cursor_move_broadcasts() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let room = Arc::new(Room::with_defaults(room_id, owner_id));
        let (tx, mut rx) = mpsc::channel(10);

        let user_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);
        session.user_id = Some(user_id);
        session.username = Some("test_user".to_string());
        session.authenticated = true;
        session.room = Some(Arc::clone(&room));

        let result = session.handle_cursor_move((150.0, 250.0)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_idle_status_broadcasts() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let room = Arc::new(Room::with_defaults(room_id, owner_id));
        let (tx, mut rx) = mpsc::channel(10);

        let user_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);
        session.user_id = Some(user_id);
        session.username = Some("test_user".to_string());
        session.authenticated = true;
        session.room = Some(Arc::clone(&room));

        let result = session.handle_idle_status(false).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_user_viewport_broadcasts() {
        let room_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let room = Arc::new(Room::with_defaults(room_id, owner_id));
        let (tx, mut rx) = mpsc::channel(10);

        let user_id = Uuid::new_v4();
        let mut session = ConnectionSession::new(room_id, tx);
        session.user_id = Some(user_id);
        session.username = Some("test_user".to_string());
        session.authenticated = true;
        session.room = Some(Arc::clone(&room));

        let result = session.handle_user_viewport((800.0, 600.0), 2.0).await;

        assert!(result.is_ok());
    }
}
