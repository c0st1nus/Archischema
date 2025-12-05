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
use super::protocol::*;
use super::room::Room;

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

    // Process incoming messages
    while let Some(result) = ws_receiver.next().await {
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
                if session.is_authenticated() {
                    if let Some(ref room) = session.room {
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

    // Cleanup: remove user from room
    session.cleanup();

    // Abort the send task
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
    async fn handle_graph_op(&self, op: super::protocol::GraphOperation) -> Result<(), String> {
        if let Some(ref room) = self.room {
            if let Some(user_id) = self.user_id {
                // Broadcast to all other clients in the room
                room.broadcast(ServerMessage::GraphOp { user_id, op });
            }
        }
        Ok(())
    }

    /// Handle request for full graph state
    /// Broadcasts a request to all other users in the room - they should respond with their graph state
    async fn handle_request_graph_state(&self) -> Result<(), String> {
        if let Some(ref room) = self.room {
            if let Some(user_id) = self.user_id {
                // Broadcast request to all other users - they should respond with GraphState
                room.broadcast(ServerMessage::RequestGraphState {
                    requester_id: user_id,
                });
            }
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

                if should_send {
                    if tx.send(msg).await.is_err() {
                        break;
                    }
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
    async fn handle_awareness(&self, state: AwarenessState) -> Result<(), String> {
        let room = self.room.as_ref().ok_or("No room")?;
        let user_id = self.user_id.ok_or("No user ID")?;

        room.update_awareness(&user_id, state);

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
}
