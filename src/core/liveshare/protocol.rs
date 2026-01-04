//! WebSocket protocol messages and DTOs for LiveShare
//!
//! This module defines all message types used for communication between
//! clients and server, as well as data transfer objects for the REST API.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// User Activity Status
// ============================================================================

/// User activity status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityStatus {
    /// User is actively working
    Active,
    /// User has been idle for 30 seconds
    Idle,
    /// User's tab is not visible or they've been away for more than 30 seconds from idle
    Away,
}

impl ActivityStatus {
    /// Convert to boolean for protocol (is_active)
    pub fn to_is_active(&self) -> bool {
        matches!(self, ActivityStatus::Active)
    }

    /// Get display string
    pub fn display_name(&self) -> &'static str {
        match self {
            ActivityStatus::Active => "Active",
            ActivityStatus::Idle => "Idle",
            ActivityStatus::Away => "Away",
        }
    }
}

// ============================================================================
// Type Aliases
// ============================================================================

/// Unique user identifier
pub type UserId = Uuid;

/// Room identifier (UUID)
pub type RoomId = Uuid;

// ============================================================================
// Message Type Classification
// ============================================================================

/// WebSocket message type for prioritization and routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsMessageType {
    /// Full schema initialization when user connects
    /// Priority: CRITICAL - Must be delivered reliably
    Init,

    /// Incremental schema changes (tables, columns, relationships)
    /// Priority: CRITICAL - Must be delivered reliably and in order
    Update,

    /// Cursor position updates
    /// Priority: VOLATILE - Can be dropped if network is congested
    CursorMove,

    /// User activity status (active/idle)
    /// Priority: LOW - Can be throttled heavily
    IdleStatus,

    /// User viewport bounds for follow mode
    /// Priority: LOW - Can be throttled
    UserViewport,
}

/// Message priority level for QoS
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Volatile messages that can be dropped (cursor updates)
    Volatile,

    /// Low priority, can be throttled
    Low,

    /// Normal priority messages
    Normal,

    /// Critical messages that must be delivered (schema changes)
    Critical,
}

impl WsMessageType {
    /// Get the priority level for this message type
    pub fn priority(&self) -> MessagePriority {
        match self {
            Self::Init | Self::Update => MessagePriority::Critical,
            Self::CursorMove => MessagePriority::Volatile,
            Self::IdleStatus | Self::UserViewport => MessagePriority::Low,
        }
    }

    /// Whether this message type can be dropped under load
    pub fn is_droppable(&self) -> bool {
        matches!(
            self.priority(),
            MessagePriority::Volatile | MessagePriority::Low
        )
    }

    /// Whether this message type requires strict ordering
    pub fn requires_ordering(&self) -> bool {
        matches!(self, Self::Init | Self::Update)
    }
}

// ============================================================================
// REST API DTOs
// ============================================================================

/// Request to create a new room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    /// The diagram ID for which this session is being created (optional for testing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_id: Option<Uuid>,
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

    /// Cursor position update (volatile, can be dropped)
    CursorMove {
        /// Cursor position on canvas (x, y)
        position: (f64, f64),
    },

    /// User idle/active status update
    IdleStatus {
        /// Whether user is currently active
        is_active: bool,
    },

    /// User viewport bounds (for follow mode)
    UserViewport {
        /// Viewport center position (x, y)
        center: (f64, f64),
        /// Viewport zoom level
        zoom: f64,
    },

    /// Start dragging a table (attaches cursor to table)
    TableDragStart {
        /// Node being dragged
        node_id: u32,
        /// Offset from cursor to table position (x, y)
        offset: (f64, f64),
    },

    /// End dragging a table (detaches cursor from table)
    TableDragEnd {
        /// Node that was being dragged
        node_id: u32,
        /// Final position of the table
        position: (f64, f64),
    },

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

    /// Broadcast cursor position from another user (volatile)
    CursorMove {
        user_id: UserId,
        position: (f64, f64),
    },

    /// Broadcast idle status from another user
    IdleStatus { user_id: UserId, is_active: bool },

    /// Broadcast viewport bounds from another user
    UserViewport {
        user_id: UserId,
        center: (f64, f64),
        zoom: f64,
    },

    /// Broadcast table drag start from another user
    TableDragStart {
        user_id: UserId,
        node_id: u32,
        offset: (f64, f64),
    },

    /// Broadcast table drag end from another user
    TableDragEnd {
        user_id: UserId,
        node_id: u32,
        position: (f64, f64),
    },

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

    /// Snapshot recovery data (Phase 7)
    /// Sent to users on connection to restore state from last snapshot
    SnapshotRecovery {
        snapshot_id: uuid::Uuid,
        snapshot_data: Vec<u8>,
        element_count: usize,
        created_at: String, // RFC3339 timestamp
    },
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
        table_uuid: Uuid,
        name: String,
        position: (f64, f64),
    },
    /// Delete a table
    DeleteTable { node_id: u32, table_uuid: Uuid },
    /// Rename a table
    RenameTable {
        node_id: u32,
        table_uuid: Uuid,
        new_name: String,
    },
    /// Move a table (change position)
    MoveTable {
        node_id: u32,
        table_uuid: Uuid,
        position: (f64, f64),
    },
    /// Add a column to a table
    AddColumn {
        node_id: u32,
        table_uuid: Uuid,
        column: ColumnData,
    },
    /// Update a column
    UpdateColumn {
        node_id: u32,
        table_uuid: Uuid,
        column_index: usize,
        column: ColumnData,
    },
    /// Delete a column
    DeleteColumn {
        node_id: u32,
        table_uuid: Uuid,
        column_index: usize,
    },
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
    pub table_uuid: Uuid,
    pub name: String,
    pub position: (f64, f64),
    pub columns: Vec<ColumnData>,
    /// Version counter for tracking changes, incremented on each modification
    #[serde(default)]
    pub version: u64,
    /// Timestamp of the last modification (Unix timestamp in milliseconds)
    #[serde(default)]
    pub last_modified_at: i64,
    /// Whether this element has been deleted (tombstone)
    #[serde(default)]
    pub is_deleted: bool,
}

/// Relationship snapshot for graph state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshot {
    pub edge_id: u32,
    pub from_node: u32,
    pub to_node: u32,
    pub data: RelationshipData,
    /// Version counter for tracking changes, incremented on each modification
    #[serde(default)]
    pub version: u64,
    /// Timestamp of the last modification (Unix timestamp in milliseconds)
    #[serde(default)]
    pub last_modified_at: i64,
    /// Whether this element has been deleted (tombstone)
    #[serde(default)]
    pub is_deleted: bool,
}

// ============================================================================
// Awareness Types
// ============================================================================

/// Awareness state for a user (cursor position, selection, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwarenessState {
    /// Username of the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

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

    /// Get the message type for prioritization
    pub fn message_type(&self) -> WsMessageType {
        match self {
            Self::GraphState { .. }
            | Self::AuthResult { .. }
            | Self::RequestGraphState { .. }
            | Self::SnapshotRecovery { .. } => WsMessageType::Init,
            Self::GraphOp { .. }
            | Self::SyncStep1 { .. }
            | Self::SyncStep2 { .. }
            | Self::Update { .. }
            | Self::UserJoined { .. }
            | Self::UserLeft { .. } => WsMessageType::Update,
            Self::CursorMove { .. } => WsMessageType::CursorMove,
            Self::IdleStatus { .. } => WsMessageType::IdleStatus,
            Self::UserViewport { .. } => WsMessageType::UserViewport,
            Self::TableDragStart { .. } | Self::TableDragEnd { .. } => WsMessageType::CursorMove,
            Self::Awareness { .. } => WsMessageType::CursorMove, // Cursor updates are volatile
            Self::Error { .. } | Self::Pong => WsMessageType::Update,
        }
    }

    /// Get the priority level of this message
    pub fn priority(&self) -> MessagePriority {
        self.message_type().priority()
    }

    /// Whether this message can be dropped under load
    pub fn is_droppable(&self) -> bool {
        self.message_type().is_droppable()
    }
}

impl ClientMessage {
    /// Get the message type for prioritization
    pub fn message_type(&self) -> WsMessageType {
        match self {
            Self::RequestGraphState | Self::GraphStateResponse { .. } | Self::Auth { .. } => {
                WsMessageType::Init
            }
            Self::GraphOp { .. }
            | Self::SyncStep1 { .. }
            | Self::SyncStep2 { .. }
            | Self::Update { .. } => WsMessageType::Update,
            Self::CursorMove { .. } => WsMessageType::CursorMove,
            Self::IdleStatus { .. } => WsMessageType::IdleStatus,
            Self::UserViewport { .. } => WsMessageType::UserViewport,
            Self::TableDragStart { .. } | Self::TableDragEnd { .. } => WsMessageType::CursorMove,
            Self::Awareness { .. } => WsMessageType::CursorMove, // Cursor updates are volatile
            Self::Ping => WsMessageType::Update,
        }
    }

    /// Get the priority level of this message
    pub fn priority(&self) -> MessagePriority {
        self.message_type().priority()
    }

    /// Whether this message can be dropped under load
    pub fn is_droppable(&self) -> bool {
        self.message_type().is_droppable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ClientMessage Tests
    // ========================================================================

    #[test]
    fn test_client_message_auth_serialization() {
        let msg = ClientMessage::Auth {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            password: Some("secret".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::Auth {
                username, password, ..
            } => {
                assert_eq!(username, "test_user");
                assert_eq!(password, Some("secret".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_auth_without_password() {
        let user_id = Uuid::new_v4();
        let msg = ClientMessage::Auth {
            user_id,
            username: "guest".to_string(),
            password: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("password"));

        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            ClientMessage::Auth { password, .. } => {
                assert!(password.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_sync_step1() {
        let state_vector = vec![1, 2, 3, 4, 5];
        let msg = ClientMessage::SyncStep1 {
            state_vector: state_vector.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::SyncStep1 { state_vector: sv } => {
                assert_eq!(sv, state_vector);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_sync_step2() {
        let update = vec![10, 20, 30];
        let msg = ClientMessage::SyncStep2 {
            update: update.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::SyncStep2 { update: u } => {
                assert_eq!(u, update);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_update() {
        let update = vec![100, 200];
        let msg = ClientMessage::Update {
            update: update.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::Update { update: u } => {
                assert_eq!(u, update);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_awareness() {
        let state = AwarenessState {
            username: Some("testuser".to_string()),
            cursor: Some((100.0, 200.0)),
            selected_nodes: vec!["node1".to_string(), "node2".to_string()],
            color: Some("#ff5733".to_string()),
            is_active: true,
        };
        let msg = ClientMessage::Awareness {
            state: state.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::Awareness { state: s } => {
                assert_eq!(s.username, Some("testuser".to_string()));
                assert_eq!(s.cursor, Some((100.0, 200.0)));
                assert_eq!(s.selected_nodes.len(), 2);
                assert_eq!(s.color, Some("#ff5733".to_string()));
                assert!(s.is_active);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_ping() {
        let msg = ClientMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("Ping"));

        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ClientMessage::Ping));
    }

    #[test]
    fn test_client_message_graph_op_create_table() {
        let test_uuid = Uuid::new_v4();
        let msg = ClientMessage::GraphOp {
            op: GraphOperation::CreateTable {
                node_id: 1,
                table_uuid: test_uuid,
                name: "users".to_string(),
                position: (100.0, 200.0),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::GraphOp { op } => match op {
                GraphOperation::CreateTable {
                    node_id,
                    table_uuid: _,
                    name,
                    position,
                } => {
                    assert_eq!(node_id, 1);
                    assert_eq!(name, "users");
                    assert_eq!(position, (100.0, 200.0));
                }
                _ => panic!("Wrong operation type"),
            },
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_client_message_request_graph_state() {
        let msg = ClientMessage::RequestGraphState;
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("RequestGraphState"));
    }

    // ========================================================================
    // ServerMessage Tests
    // ========================================================================

    #[test]
    fn test_server_message_error_serialization() {
        let msg = ServerMessage::error(WsErrorCode::RoomFull, "Room is full");
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("room_full"));
        assert!(json.contains("Room is full"));
    }

    #[test]
    fn test_server_message_auth_success() {
        let room_info = RoomResponse {
            id: Uuid::new_v4(),
            name: "Test Room".to_string(),
            websocket_url: "ws://localhost:3000/room/123".to_string(),
            is_protected: false,
            user_count: 1,
            max_users: 50,
            owner_id: Uuid::new_v4(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            users: vec![],
        };

        let msg = ServerMessage::auth_success(room_info);
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("AuthResult"));
        assert!(json.contains("Test Room"));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_server_message_auth_failed() {
        let msg = ServerMessage::auth_failed("Invalid password");
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("AuthResult"));
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("Invalid password"));
    }

    #[test]
    fn test_server_message_user_joined() {
        let user_id = Uuid::new_v4();
        let msg = ServerMessage::UserJoined {
            user_id,
            username: "new_user".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ServerMessage::UserJoined { username, .. } => {
                assert_eq!(username, "new_user");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_message_user_left() {
        let user_id = Uuid::new_v4();
        let msg = ServerMessage::UserLeft { user_id };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

        assert!(matches!(parsed, ServerMessage::UserLeft { .. }));
    }

    #[test]
    fn test_server_message_pong() {
        let msg = ServerMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("Pong"));
    }

    #[test]
    fn test_server_message_graph_state() {
        let state = GraphStateSnapshot {
            tables: vec![TableSnapshot {
                node_id: 1,
                table_uuid: Uuid::new_v4(),
                name: "users".to_string(),
                position: (0.0, 0.0),
                columns: vec![],
                version: 1,
                last_modified_at: 0,
                is_deleted: false,
            }],
            relationships: vec![],
        };

        let msg = ServerMessage::GraphState {
            state,
            target_user_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("users"));
    }

    // ========================================================================
    // AwarenessState Tests
    // ========================================================================

    #[test]
    fn test_awareness_state_defaults() {
        let state = AwarenessState::default();
        assert!(state.cursor.is_none());
        assert!(state.selected_nodes.is_empty());
        assert!(state.color.is_none());
        assert!(!state.is_active);
    }

    #[test]
    fn test_awareness_state_full() {
        let state = AwarenessState {
            username: Some("testuser".to_string()),
            cursor: Some((50.5, 100.5)),
            selected_nodes: vec!["a".to_string(), "b".to_string()],
            color: Some("#00ff00".to_string()),
            is_active: true,
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: AwarenessState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.username, Some("testuser".to_string()));
        assert_eq!(parsed.cursor, Some((50.5, 100.5)));
        assert_eq!(parsed.selected_nodes, vec!["a", "b"]);
        assert_eq!(parsed.color, Some("#00ff00".to_string()));
        assert!(parsed.is_active);
    }

    #[test]
    fn test_awareness_state_skip_empty_fields() {
        let state = AwarenessState {
            username: None,
            cursor: None,
            selected_nodes: vec![],
            color: None,
            is_active: false,
        };

        let json = serde_json::to_string(&state).unwrap();

        // Empty/None fields should be skipped
        assert!(!json.contains("cursor"));
        assert!(!json.contains("selected_nodes"));
        assert!(!json.contains("color"));
    }

    // ========================================================================
    // ApiError Tests
    // ========================================================================

    #[test]
    fn test_api_error_room_not_found() {
        let err = ApiError::room_not_found();
        assert_eq!(err.error, "Room not found");
        assert_eq!(err.code, ApiErrorCode::RoomNotFound);
    }

    #[test]
    fn test_api_error_room_full() {
        let err = ApiError::room_full();
        assert_eq!(err.error, "Room is full");
        assert_eq!(err.code, ApiErrorCode::RoomFull);
    }

    #[test]
    fn test_api_error_invalid_password() {
        let err = ApiError::invalid_password();
        assert_eq!(err.error, "Invalid password");
        assert_eq!(err.code, ApiErrorCode::InvalidPassword);
    }

    #[test]
    fn test_api_error_unauthorized() {
        let err = ApiError::unauthorized();
        assert_eq!(err.error, "Authentication required");
        assert_eq!(err.code, ApiErrorCode::Unauthorized);
    }

    #[test]
    fn test_api_error_forbidden() {
        let err = ApiError::forbidden();
        assert!(err.error.contains("permission"));
        assert_eq!(err.code, ApiErrorCode::Forbidden);
    }

    #[test]
    fn test_api_error_bad_request() {
        let err = ApiError::bad_request("Invalid input data");
        assert_eq!(err.error, "Invalid input data");
        assert_eq!(err.code, ApiErrorCode::BadRequest);
    }

    #[test]
    fn test_api_error_internal() {
        let err = ApiError::internal("Database connection failed");
        assert_eq!(err.error, "Database connection failed");
        assert_eq!(err.code, ApiErrorCode::InternalError);
    }

    #[test]
    fn test_api_error_serialization() {
        let err = ApiError::room_not_found();
        let json = serde_json::to_string(&err).unwrap();

        assert!(json.contains("room_not_found"));
        assert!(json.contains("Room not found"));

        let parsed: ApiError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, ApiErrorCode::RoomNotFound);
    }

    // ========================================================================
    // GraphOperation Tests
    // ========================================================================

    #[test]
    fn test_graph_operation_create_table() {
        let test_uuid = Uuid::new_v4();
        let op = GraphOperation::CreateTable {
            node_id: 42,
            table_uuid: test_uuid,
            name: "products".to_string(),
            position: (150.0, 250.0),
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::CreateTable {
                node_id,
                table_uuid,
                name,
                position,
            } => {
                assert_eq!(node_id, 42);
                assert_eq!(table_uuid, test_uuid);
                assert_eq!(name, "products");
                assert_eq!(position, (150.0, 250.0));
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_delete_table() {
        let test_uuid = Uuid::new_v4();
        let op = GraphOperation::DeleteTable {
            node_id: 5,
            table_uuid: test_uuid,
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::DeleteTable {
                node_id,
                table_uuid,
            } => {
                assert_eq!(node_id, 5);
                assert_eq!(table_uuid, test_uuid);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_rename_table() {
        let test_uuid = Uuid::new_v4();
        let op = GraphOperation::RenameTable {
            node_id: 10,
            table_uuid: test_uuid,
            new_name: "customers".to_string(),
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::RenameTable {
                node_id,
                table_uuid,
                new_name,
            } => {
                assert_eq!(node_id, 10);
                assert_eq!(table_uuid, test_uuid);
                assert_eq!(new_name, "customers");
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_move_table() {
        let test_uuid = Uuid::new_v4();
        let op = GraphOperation::MoveTable {
            node_id: 3,
            table_uuid: test_uuid,
            position: (500.0, 300.0),
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::MoveTable {
                node_id,
                table_uuid,
                position,
            } => {
                assert_eq!(node_id, 3);
                assert_eq!(table_uuid, test_uuid);
                assert_eq!(position, (500.0, 300.0));
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_add_column() {
        let test_uuid = Uuid::new_v4();
        let column = ColumnData {
            name: "email".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            is_primary_key: false,
            is_nullable: false,
            is_unique: true,
            default_value: None,
            foreign_key: None,
        };

        let op = GraphOperation::AddColumn {
            node_id: 1,
            table_uuid: test_uuid,
            column: column.clone(),
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::AddColumn {
                node_id,
                table_uuid,
                column: c,
            } => {
                assert_eq!(node_id, 1);
                assert_eq!(table_uuid, test_uuid);
                assert_eq!(c.name, "email");
                assert!(c.is_unique);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_update_column() {
        let test_uuid = Uuid::new_v4();
        let column = ColumnData {
            name: "username".to_string(),
            data_type: "VARCHAR(100)".to_string(),
            is_primary_key: false,
            is_nullable: false,
            is_unique: true,
            default_value: Some("guest".to_string()),
            foreign_key: None,
        };

        let op = GraphOperation::UpdateColumn {
            node_id: 2,
            table_uuid: test_uuid,
            column_index: 3,
            column: column.clone(),
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::UpdateColumn {
                node_id,
                table_uuid,
                column_index,
                column: c,
            } => {
                assert_eq!(node_id, 2);
                assert_eq!(table_uuid, test_uuid);
                assert_eq!(column_index, 3);
                assert_eq!(c.default_value, Some("guest".to_string()));
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_delete_column() {
        let test_uuid = Uuid::new_v4();
        let op = GraphOperation::DeleteColumn {
            node_id: 7,
            table_uuid: test_uuid,
            column_index: 2,
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::DeleteColumn {
                node_id,
                table_uuid,
                column_index,
            } => {
                assert_eq!(node_id, 7);
                assert_eq!(table_uuid, test_uuid);
                assert_eq!(column_index, 2);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_create_relationship() {
        let relationship = RelationshipData {
            name: "user_orders".to_string(),
            relationship_type: "one_to_many".to_string(),
            from_column: "id".to_string(),
            to_column: "user_id".to_string(),
        };

        let op = GraphOperation::CreateRelationship {
            edge_id: 100,
            from_node: 1,
            to_node: 2,
            relationship,
        };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::CreateRelationship {
                edge_id,
                from_node,
                to_node,
                relationship: r,
            } => {
                assert_eq!(edge_id, 100);
                assert_eq!(from_node, 1);
                assert_eq!(to_node, 2);
                assert_eq!(r.name, "user_orders");
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_graph_operation_delete_relationship() {
        let op = GraphOperation::DeleteRelationship { edge_id: 55 };

        let json = serde_json::to_string(&op).unwrap();
        let parsed: GraphOperation = serde_json::from_str(&json).unwrap();

        match parsed {
            GraphOperation::DeleteRelationship { edge_id } => {
                assert_eq!(edge_id, 55);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    // ========================================================================
    // ColumnData Tests
    // ========================================================================

    #[test]
    fn test_column_data_serialization() {
        let column = ColumnData {
            name: "id".to_string(),
            data_type: "INT".to_string(),
            is_primary_key: true,
            is_nullable: false,
            is_unique: true,
            default_value: None,
            foreign_key: None,
        };

        let json = serde_json::to_string(&column).unwrap();
        let parsed: ColumnData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "id");
        assert!(parsed.is_primary_key);
        assert!(!parsed.is_nullable);
    }

    #[test]
    fn test_column_data_with_foreign_key() {
        let column = ColumnData {
            name: "user_id".to_string(),
            data_type: "INT".to_string(),
            is_primary_key: false,
            is_nullable: false,
            is_unique: false,
            default_value: None,
            foreign_key: Some(ForeignKeyData {
                ref_table: "users".to_string(),
                ref_column: "id".to_string(),
            }),
        };

        let json = serde_json::to_string(&column).unwrap();
        let parsed: ColumnData = serde_json::from_str(&json).unwrap();

        let fk = parsed.foreign_key.unwrap();
        assert_eq!(fk.ref_table, "users");
        assert_eq!(fk.ref_column, "id");
    }

    // ========================================================================
    // GraphStateSnapshot Tests
    // ========================================================================

    #[test]
    fn test_graph_state_snapshot_empty() {
        let snapshot = GraphStateSnapshot {
            tables: vec![],
            relationships: vec![],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: GraphStateSnapshot = serde_json::from_str(&json).unwrap();

        assert!(parsed.tables.is_empty());
        assert!(parsed.relationships.is_empty());
    }

    #[test]
    fn test_graph_state_snapshot_with_data() {
        let snapshot = GraphStateSnapshot {
            tables: vec![
                TableSnapshot {
                    node_id: 1,
                    table_uuid: Uuid::new_v4(),
                    name: "users".to_string(),
                    position: (100.0, 200.0),
                    columns: vec![ColumnData {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        is_primary_key: true,
                        is_nullable: false,
                        is_unique: true,
                        default_value: None,
                        foreign_key: None,
                    }],
                    version: 1,
                    last_modified_at: 1234567890,
                    is_deleted: false,
                },
                TableSnapshot {
                    node_id: 2,
                    table_uuid: Uuid::new_v4(),
                    name: "posts".to_string(),
                    position: (300.0, 400.0),
                    columns: vec![],
                    version: 2,
                    last_modified_at: 1234567900,
                    is_deleted: false,
                },
            ],
            relationships: vec![RelationshipSnapshot {
                edge_id: 1,
                from_node: 1,
                to_node: 2,
                version: 1,
                last_modified_at: 0,
                is_deleted: false,
                data: RelationshipData {
                    name: "user_posts".to_string(),
                    relationship_type: "one_to_many".to_string(),
                    from_column: "id".to_string(),
                    to_column: "user_id".to_string(),
                },
            }],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: GraphStateSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.tables.len(), 2);
        assert_eq!(parsed.relationships.len(), 1);
        assert_eq!(parsed.tables[0].name, "users");
        assert_eq!(parsed.tables[0].columns.len(), 1);
    }

    // ========================================================================
    // UserInfo Tests
    // ========================================================================

    #[test]
    fn test_user_info_serialization() {
        let info = UserInfo {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            color: Some("#ff0000".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: UserInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.username, "test_user");
        assert_eq!(parsed.color, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_user_info_without_color() {
        let info = UserInfo {
            user_id: Uuid::new_v4(),
            username: "guest".to_string(),
            color: None,
        };

        let json = serde_json::to_string(&info).unwrap();

        // Color should be skipped when None
        assert!(!json.contains("color"));
    }

    // ========================================================================
    // RoomResponse Tests
    // ========================================================================

    #[test]
    fn test_room_response_serialization() {
        let response = RoomResponse {
            id: Uuid::new_v4(),
            name: "My Room".to_string(),
            websocket_url: "ws://localhost:3000/room/abc".to_string(),
            is_protected: true,
            user_count: 5,
            max_users: 50,
            owner_id: Uuid::new_v4(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            users: vec![UserInfo {
                user_id: Uuid::new_v4(),
                username: "user1".to_string(),
                color: None,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: RoomResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "My Room");
        assert!(parsed.is_protected);
        assert_eq!(parsed.user_count, 5);
        assert_eq!(parsed.users.len(), 1);
    }

    // ========================================================================
    // CreateRoomRequest Tests
    // ========================================================================

    #[test]
    fn test_create_room_request_full() {
        let request = CreateRoomRequest {
            diagram_id: Some(Uuid::new_v4()),
            name: Some("Test Room".to_string()),
            password: Some("secret123".to_string()),
            max_users: Some(25),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: CreateRoomRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, Some("Test Room".to_string()));
        assert_eq!(parsed.password, Some("secret123".to_string()));
        assert_eq!(parsed.max_users, Some(25));
    }

    #[test]
    fn test_create_room_request_minimal() {
        let request = CreateRoomRequest {
            diagram_id: Some(Uuid::new_v4()),
            name: None,
            password: None,
            max_users: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: CreateRoomRequest = serde_json::from_str(&json).unwrap();

        assert!(parsed.name.is_none());
        assert!(parsed.password.is_none());
        assert!(parsed.max_users.is_none());
    }

    // ========================================================================
    // UpdateRoomRequest Tests
    // ========================================================================

    #[test]
    fn test_update_room_request() {
        let request = UpdateRoomRequest {
            name: Some("New Name".to_string()),
            password: Some("".to_string()), // Empty string to remove password
            max_users: Some(100),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: UpdateRoomRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, Some("New Name".to_string()));
        assert_eq!(parsed.password, Some("".to_string()));
        assert_eq!(parsed.max_users, Some(100));
    }

    // ========================================================================
    // WsErrorCode Tests
    // ========================================================================

    #[test]
    fn test_ws_error_code_serialization() {
        let codes = vec![
            WsErrorCode::RoomFull,
            WsErrorCode::InvalidPassword,
            WsErrorCode::NotAuthenticated,
            WsErrorCode::RoomNotFound,
            WsErrorCode::InvalidMessage,
            WsErrorCode::InternalError,
        ];

        for code in codes {
            let json = serde_json::to_string(&code).unwrap();
            let parsed: WsErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, code);
        }
    }

    #[test]
    fn test_ws_error_code_snake_case() {
        let code = WsErrorCode::NotAuthenticated;
        let json = serde_json::to_string(&code).unwrap();

        assert!(json.contains("not_authenticated"));
    }

    // ============================================================================
    // Message Type and Priority Tests
    // ============================================================================

    #[test]
    fn test_ws_message_type_priority() {
        assert_eq!(WsMessageType::Init.priority(), MessagePriority::Critical);
        assert_eq!(WsMessageType::Update.priority(), MessagePriority::Critical);
        assert_eq!(
            WsMessageType::CursorMove.priority(),
            MessagePriority::Volatile
        );
        assert_eq!(WsMessageType::IdleStatus.priority(), MessagePriority::Low);
        assert_eq!(WsMessageType::UserViewport.priority(), MessagePriority::Low);
    }

    #[test]
    fn test_ws_message_type_is_droppable() {
        assert!(!WsMessageType::Init.is_droppable());
        assert!(!WsMessageType::Update.is_droppable());
        assert!(WsMessageType::CursorMove.is_droppable());
        assert!(WsMessageType::IdleStatus.is_droppable());
        assert!(WsMessageType::UserViewport.is_droppable());
    }

    #[test]
    fn test_ws_message_type_requires_ordering() {
        assert!(WsMessageType::Init.requires_ordering());
        assert!(WsMessageType::Update.requires_ordering());
        assert!(!WsMessageType::CursorMove.requires_ordering());
        assert!(!WsMessageType::IdleStatus.requires_ordering());
        assert!(!WsMessageType::UserViewport.requires_ordering());
    }

    #[test]
    fn test_client_message_cursor_move_serialization() {
        let msg = ClientMessage::CursorMove {
            position: (100.5, 200.3),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::CursorMove { position } => {
                assert_eq!(position, (100.5, 200.3));
            }
            _ => panic!("Expected CursorMove variant"),
        }
    }

    #[test]
    fn test_client_message_idle_status_serialization() {
        let msg = ClientMessage::IdleStatus { is_active: true };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::IdleStatus { is_active } => {
                assert!(is_active);
            }
            _ => panic!("Expected IdleStatus variant"),
        }
    }

    #[test]
    fn test_client_message_user_viewport_serialization() {
        let msg = ClientMessage::UserViewport {
            center: (500.0, 600.0),
            zoom: 1.5,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::UserViewport { center, zoom } => {
                assert_eq!(center, (500.0, 600.0));
                assert_eq!(zoom, 1.5);
            }
            _ => panic!("Expected UserViewport variant"),
        }
    }

    #[test]
    fn test_server_message_cursor_move_broadcast() {
        let user_id = Uuid::new_v4();
        let msg = ServerMessage::CursorMove {
            user_id,
            position: (150.0, 250.0),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ServerMessage::CursorMove {
                user_id: uid,
                position,
            } => {
                assert_eq!(uid, user_id);
                assert_eq!(position, (150.0, 250.0));
            }
            _ => panic!("Expected CursorMove variant"),
        }
    }

    #[test]
    fn test_server_message_idle_status_broadcast() {
        let user_id = Uuid::new_v4();
        let msg = ServerMessage::IdleStatus {
            user_id,
            is_active: false,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ServerMessage::IdleStatus {
                user_id: uid,
                is_active,
            } => {
                assert_eq!(uid, user_id);
                assert!(!is_active);
            }
            _ => panic!("Expected IdleStatus variant"),
        }
    }

    #[test]
    fn test_server_message_user_viewport_broadcast() {
        let user_id = Uuid::new_v4();
        let msg = ServerMessage::UserViewport {
            user_id,
            center: (800.0, 600.0),
            zoom: 2.0,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ServerMessage::UserViewport {
                user_id: uid,
                center,
                zoom,
            } => {
                assert_eq!(uid, user_id);
                assert_eq!(center, (800.0, 600.0));
                assert_eq!(zoom, 2.0);
            }
            _ => panic!("Expected UserViewport variant"),
        }
    }

    #[test]
    fn test_client_message_priority_classification() {
        let auth_msg = ClientMessage::Auth {
            user_id: Uuid::new_v4(),
            username: "test".to_string(),
            password: None,
        };
        assert_eq!(auth_msg.priority(), MessagePriority::Critical);

        let graph_op_msg = ClientMessage::GraphOp {
            op: GraphOperation::DeleteTable {
                node_id: 1,
                table_uuid: Uuid::new_v4(),
            },
        };
        assert_eq!(graph_op_msg.priority(), MessagePriority::Critical);

        let cursor_msg = ClientMessage::CursorMove {
            position: (0.0, 0.0),
        };
        assert_eq!(cursor_msg.priority(), MessagePriority::Volatile);
        assert!(cursor_msg.is_droppable());

        let idle_msg = ClientMessage::IdleStatus { is_active: true };
        assert_eq!(idle_msg.priority(), MessagePriority::Low);
        assert!(idle_msg.is_droppable());

        let viewport_msg = ClientMessage::UserViewport {
            center: (0.0, 0.0),
            zoom: 1.0,
        };
        assert_eq!(viewport_msg.priority(), MessagePriority::Low);
        assert!(viewport_msg.is_droppable());
    }

    #[test]
    fn test_server_message_priority_classification() {
        let user_id = Uuid::new_v4();

        let graph_state_msg = ServerMessage::GraphState {
            state: GraphStateSnapshot {
                tables: vec![],
                relationships: vec![],
            },
            target_user_id: None,
        };
        assert_eq!(graph_state_msg.priority(), MessagePriority::Critical);

        let graph_op_msg = ServerMessage::GraphOp {
            user_id,
            op: GraphOperation::CreateTable {
                node_id: 1,
                table_uuid: Uuid::new_v4(),
                name: "test".to_string(),
                position: (0.0, 0.0),
            },
        };
        assert_eq!(graph_op_msg.priority(), MessagePriority::Critical);

        let cursor_msg = ServerMessage::CursorMove {
            user_id,
            position: (100.0, 200.0),
        };
        assert_eq!(cursor_msg.priority(), MessagePriority::Volatile);
        assert!(cursor_msg.is_droppable());

        let idle_msg = ServerMessage::IdleStatus {
            user_id,
            is_active: false,
        };
        assert_eq!(idle_msg.priority(), MessagePriority::Low);
        assert!(idle_msg.is_droppable());

        let viewport_msg = ServerMessage::UserViewport {
            user_id,
            center: (400.0, 300.0),
            zoom: 1.5,
        };
        assert_eq!(viewport_msg.priority(), MessagePriority::Low);
        assert!(viewport_msg.is_droppable());
    }

    #[test]
    fn test_message_priority_ordering() {
        assert!(MessagePriority::Critical > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Low);
        assert!(MessagePriority::Low > MessagePriority::Volatile);
    }
}
