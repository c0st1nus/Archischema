//! WebSocket protocol messages and DTOs for LiveShare
//!
//! This module defines all message types used for communication between
//! clients and server, as well as data transfer objects for the REST API.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Type Aliases
// ============================================================================

/// Unique user identifier
pub type UserId = Uuid;

/// Room identifier (UUID)
pub type RoomId = Uuid;

// ============================================================================
// REST API DTOs
// ============================================================================

/// Request to create a new room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    /// Optional room name (for display purposes)
    pub name: Option<String>,
    /// Optional password to protect the room
    pub password: Option<String>,
    /// Maximum number of users (default: 50, max: 50)
    pub max_users: Option<usize>,
}

/// Request to update room settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoomRequest {
    /// New room name
    pub name: Option<String>,
    /// New password (None = keep current, Some("") = remove password)
    pub password: Option<String>,
    /// New maximum users limit
    pub max_users: Option<usize>,
}

/// Room information returned by API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomResponse {
    /// Room UUID
    pub id: RoomId,
    /// Room display name
    pub name: String,
    /// WebSocket URL to connect to this room
    pub websocket_url: String,
    /// Whether the room is password protected
    pub is_protected: bool,
    /// Current number of connected users
    pub user_count: usize,
    /// Maximum allowed users
    pub max_users: usize,
    /// Room owner's user ID
    pub owner_id: UserId,
    /// Room creation timestamp (ISO 8601)
    pub created_at: String,
    /// List of currently connected users
    pub users: Vec<UserInfo>,
}

/// Basic user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: UserId,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub code: ApiErrorCode,
}

/// API error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    RoomNotFound,
    RoomFull,
    InvalidPassword,
    Unauthorized,
    Forbidden,
    BadRequest,
    InternalError,
}

impl ApiError {
    pub fn room_not_found() -> Self {
        Self {
            error: "Room not found".to_string(),
            code: ApiErrorCode::RoomNotFound,
        }
    }

    pub fn room_full() -> Self {
        Self {
            error: "Room is full".to_string(),
            code: ApiErrorCode::RoomFull,
        }
    }

    pub fn invalid_password() -> Self {
        Self {
            error: "Invalid password".to_string(),
            code: ApiErrorCode::InvalidPassword,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            error: "Authentication required".to_string(),
            code: ApiErrorCode::Unauthorized,
        }
    }

    pub fn forbidden() -> Self {
        Self {
            error: "You don't have permission to perform this action".to_string(),
            code: ApiErrorCode::Forbidden,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: ApiErrorCode::BadRequest,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: ApiErrorCode::InternalError,
        }
    }
}

// ============================================================================
// WebSocket Protocol Messages
// ============================================================================

/// Client-to-server WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Authenticate with the room
    /// This must be the first message sent after connecting
    Auth {
        /// User's unique identifier
        user_id: UserId,
        /// Display name
        username: String,
        /// Room password (if room is protected)
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },

    /// Yjs sync step 1: send client's state vector
    SyncStep1 { state_vector: Vec<u8> },

    /// Yjs sync step 2: send update based on server's state vector
    SyncStep2 { update: Vec<u8> },

    /// Incremental Yjs update (after initial sync)
    Update { update: Vec<u8> },

    /// Update awareness state (cursor, selection, etc.)
    Awareness { state: AwarenessState },

    /// Graph operation for schema synchronization
    GraphOp { op: GraphOperation },

    /// Request full graph state (for initial sync)
    RequestGraphState,

    /// Response with full graph state (to share with new user)
    GraphStateResponse {
        target_user_id: UserId,
        state: GraphStateSnapshot,
    },

    /// Ping for keepalive
    Ping,
}

/// Server-to-client WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Authentication result
    AuthResult {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        room_info: Option<RoomResponse>,
    },

    /// Yjs sync step 1: server's state vector
    SyncStep1 { state_vector: Vec<u8> },

    /// Yjs sync step 2: update from server
    SyncStep2 { update: Vec<u8> },

    /// Broadcast Yjs update to all clients
    Update { update: Vec<u8> },

    /// Broadcast awareness state from another user
    Awareness {
        user_id: UserId,
        state: AwarenessState,
    },

    /// Broadcast graph operation from another user
    GraphOp { user_id: UserId, op: GraphOperation },

    /// Full graph state (response to RequestGraphState)
    /// If target_user_id is Some, only that user should process this message
    GraphState {
        state: GraphStateSnapshot,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_user_id: Option<UserId>,
    },

    /// Request for graph state from existing users (sent to all except requester)
    RequestGraphState { requester_id: UserId },

    /// User joined the room
    UserJoined { user_id: UserId, username: String },

    /// User left the room
    UserLeft { user_id: UserId },

    /// Error message
    Error { code: WsErrorCode, message: String },

    /// Pong response to Ping
    Pong,
}

// ============================================================================
// Graph Synchronization Types
// ============================================================================

/// Graph operation for real-time synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op_type")]
pub enum GraphOperation {
    /// Create a new table
    CreateTable {
        node_id: u32,
        name: String,
        position: (f64, f64),
    },
    /// Delete a table
    DeleteTable { node_id: u32 },
    /// Rename a table
    RenameTable { node_id: u32, new_name: String },
    /// Move a table (change position)
    MoveTable { node_id: u32, position: (f64, f64) },
    /// Add a column to a table
    AddColumn { node_id: u32, column: ColumnData },
    /// Update a column
    UpdateColumn {
        node_id: u32,
        column_index: usize,
        column: ColumnData,
    },
    /// Delete a column
    DeleteColumn { node_id: u32, column_index: usize },
    /// Create a relationship between tables
    CreateRelationship {
        edge_id: u32,
        from_node: u32,
        to_node: u32,
        relationship: RelationshipData,
    },
    /// Delete a relationship
    DeleteRelationship { edge_id: u32 },
}

/// Serializable column data for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnData {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub default_value: Option<String>,
    pub foreign_key: Option<ForeignKeyData>,
}

/// Serializable foreign key data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyData {
    pub ref_table: String,
    pub ref_column: String,
}

/// Serializable relationship data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipData {
    pub name: String,
    pub relationship_type: String,
    pub from_column: String,
    pub to_column: String,
}

/// Full graph state snapshot for initial sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateSnapshot {
    pub tables: Vec<TableSnapshot>,
    pub relationships: Vec<RelationshipSnapshot>,
}

/// Table snapshot for graph state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSnapshot {
    pub node_id: u32,
    pub name: String,
    pub position: (f64, f64),
    pub columns: Vec<ColumnData>,
}

/// Relationship snapshot for graph state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshot {
    pub edge_id: u32,
    pub from_node: u32,
    pub to_node: u32,
    pub data: RelationshipData,
}

// ============================================================================
// Awareness Types
// ============================================================================

/// Awareness state for a user (cursor position, selection, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwarenessState {
    /// Cursor position on canvas (x, y)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<(f64, f64)>,

    /// Currently selected node IDs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_nodes: Vec<String>,

    /// User's display color (hex, e.g., "#ff5733")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Whether user is currently active/typing
    #[serde(default)]
    pub is_active: bool,
}

/// WebSocket error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WsErrorCode {
    RoomFull,
    InvalidPassword,
    NotAuthenticated,
    RoomNotFound,
    InvalidMessage,
    InternalError,
}

// ============================================================================
// Helper Implementations
// ============================================================================

impl ServerMessage {
    /// Create an error message
    pub fn error(code: WsErrorCode, message: impl Into<String>) -> Self {
        Self::Error {
            code,
            message: message.into(),
        }
    }

    /// Create a successful auth result
    pub fn auth_success(room_info: RoomResponse) -> Self {
        Self::AuthResult {
            success: true,
            error: None,
            room_info: Some(room_info),
        }
    }

    /// Create a failed auth result
    pub fn auth_failed(error: impl Into<String>) -> Self {
        Self::AuthResult {
            success: false,
            error: Some(error.into()),
            room_info: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Auth {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            password: Some("secret".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::Auth { username, .. } => {
                assert_eq!(username, "test_user");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::error(WsErrorCode::RoomFull, "Room is full");
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("room_full"));
        assert!(json.contains("Room is full"));
    }

    #[test]
    fn test_awareness_state_defaults() {
        let state = AwarenessState::default();
        assert!(state.cursor.is_none());
        assert!(state.selected_nodes.is_empty());
        assert!(!state.is_active);
    }
}
