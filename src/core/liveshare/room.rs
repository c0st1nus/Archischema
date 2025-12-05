//! Room management for LiveShare
//!
//! This module provides the core data structures and logic for managing
//! collaborative editing rooms, including:
//! - Room creation, configuration, and lifecycle
//! - User management within rooms
//! - Yjs document state synchronization
//! - Password protection

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;
use yrs::{Doc, ReadTxn, Transact, updates::decoder::Decode, updates::encoder::Encode};

use super::auth::AuthenticatedUser;
use super::protocol::*;

// ============================================================================
// Constants
// ============================================================================

/// Maximum users per room
pub const MAX_USERS_PER_ROOM: usize = 50;

/// Minimum users per room (for validation)
pub const MIN_USERS_PER_ROOM: usize = 1;

/// Broadcast channel capacity
const BROADCAST_CAPACITY: usize = 256;

// ============================================================================
// Room Configuration
// ============================================================================

/// Room configuration options
#[derive(Debug, Clone)]
pub struct RoomConfig {
    /// Maximum number of users allowed in the room
    pub max_users: usize,
    /// BCrypt hashed password (if room is password protected)
    pub password_hash: Option<String>,
    /// Room display name
    pub name: String,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            max_users: MAX_USERS_PER_ROOM,
            password_hash: None,
            name: "Untitled Room".to_string(),
        }
    }
}

impl RoomConfig {
    /// Create a new room config with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the maximum number of users
    pub fn max_users(mut self, max: usize) -> Self {
        self.max_users = max.clamp(MIN_USERS_PER_ROOM, MAX_USERS_PER_ROOM);
        self
    }

    /// Set password protection
    pub fn with_password(mut self, password: &str) -> Result<Self, bcrypt::BcryptError> {
        self.password_hash = Some(bcrypt::hash(password, bcrypt::DEFAULT_COST)?);
        Ok(self)
    }
}

// ============================================================================
// Connected User
// ============================================================================

/// A user currently connected to a room
#[derive(Debug, Clone)]
pub struct ConnectedUser {
    /// User's unique identifier
    pub user_id: UserId,
    /// User's display name
    pub username: String,
    /// When the user connected
    pub connected_at: DateTime<Utc>,
    /// User's current awareness state
    pub awareness: AwarenessState,
}

impl ConnectedUser {
    /// Create a new connected user
    pub fn new(user_id: UserId, username: String) -> Self {
        Self {
            user_id,
            username,
            connected_at: Utc::now(),
            awareness: AwarenessState::default(),
        }
    }

    /// Convert to UserInfo for API responses
    pub fn to_info(&self) -> UserInfo {
        UserInfo {
            user_id: self.user_id,
            username: self.username.clone(),
            color: self.awareness.color.clone(),
        }
    }
}

// ============================================================================
// Room
// ============================================================================

/// A collaborative editing room
pub struct Room {
    /// Room's unique identifier
    pub id: RoomId,
    /// Room configuration
    pub config: RoomConfig,
    /// User ID of the room owner/creator
    pub owner_id: UserId,
    /// When the room was created
    pub created_at: DateTime<Utc>,

    /// Yjs document for CRDT state synchronization
    doc: RwLock<Doc>,

    /// Currently connected users
    users: DashMap<UserId, ConnectedUser>,

    /// Broadcast channel for sending messages to all connected clients
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl Room {
    /// Create a new room with the given configuration
    pub fn new(id: RoomId, owner_id: UserId, config: RoomConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        Self {
            id,
            config,
            owner_id,
            created_at: Utc::now(),
            doc: RwLock::new(Doc::new()),
            users: DashMap::new(),
            broadcast_tx,
        }
    }

    /// Create a room with default configuration
    pub fn with_defaults(id: RoomId, owner_id: UserId) -> Self {
        Self::new(id, owner_id, RoomConfig::default())
    }

    // ========================================================================
    // Password Protection
    // ========================================================================

    /// Check if the room is password protected
    pub fn is_protected(&self) -> bool {
        self.config.password_hash.is_some()
    }

    /// Verify a password against the room's password hash
    pub fn verify_password(&self, password: Option<&str>) -> bool {
        match (&self.config.password_hash, password) {
            // No password required
            (None, _) => true,
            // Password required and provided
            (Some(hash), Some(pwd)) => bcrypt::verify(pwd, hash).unwrap_or(false),
            // Password required but not provided
            (Some(_), None) => false,
        }
    }

    /// Update the room's password
    pub fn set_password(&mut self, password: Option<&str>) -> Result<(), bcrypt::BcryptError> {
        self.config.password_hash = match password {
            Some(pwd) if !pwd.is_empty() => Some(bcrypt::hash(pwd, bcrypt::DEFAULT_COST)?),
            _ => None,
        };
        Ok(())
    }

    // ========================================================================
    // User Management
    // ========================================================================

    /// Check if the room is full
    pub fn is_full(&self) -> bool {
        self.users.len() >= self.config.max_users
    }

    /// Get the current number of connected users
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Check if a user is connected to this room
    pub fn has_user(&self, user_id: &UserId) -> bool {
        self.users.contains_key(user_id)
    }

    /// Get a list of all connected users
    pub fn get_users(&self) -> Vec<UserInfo> {
        self.users.iter().map(|entry| entry.to_info()).collect()
    }

    /// Add a user to the room
    pub fn add_user(&self, user_id: UserId, username: String) -> Result<(), ApiError> {
        if self.is_full() {
            return Err(ApiError::room_full());
        }

        let user = ConnectedUser::new(user_id, username.clone());
        self.users.insert(user_id, user);

        // Broadcast user joined event
        let _ = self
            .broadcast_tx
            .send(ServerMessage::UserJoined { user_id, username });

        Ok(())
    }

    /// Remove a user from the room
    pub fn remove_user(&self, user_id: &UserId) -> Option<ConnectedUser> {
        if let Some((_, user)) = self.users.remove(user_id) {
            // Broadcast user left event
            let _ = self
                .broadcast_tx
                .send(ServerMessage::UserLeft { user_id: *user_id });
            Some(user)
        } else {
            None
        }
    }

    /// Update a user's awareness state
    pub fn update_awareness(&self, user_id: &UserId, state: AwarenessState) {
        if let Some(mut user) = self.users.get_mut(user_id) {
            user.awareness = state.clone();
        }

        // Broadcast awareness update
        let _ = self.broadcast_tx.send(ServerMessage::Awareness {
            user_id: *user_id,
            state,
        });
    }

    // ========================================================================
    // Broadcasting
    // ========================================================================

    /// Subscribe to room broadcasts
    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Broadcast a message to all connected clients
    pub fn broadcast(&self, msg: ServerMessage) {
        let _ = self.broadcast_tx.send(msg);
    }

    // ========================================================================
    // Yjs Document Synchronization
    // ========================================================================

    /// Get the Yjs document's state vector
    pub async fn get_state_vector(&self) -> Vec<u8> {
        let doc = self.doc.read().await;
        let txn = doc.transact();
        txn.state_vector().encode_v1().to_vec()
    }

    /// Get a Yjs update based on a client's state vector
    pub async fn get_update_from_sv(&self, state_vector: &[u8]) -> Option<Vec<u8>> {
        let doc = self.doc.read().await;
        let txn = doc.transact();

        yrs::StateVector::decode_v1(state_vector)
            .ok()
            .map(|sv| txn.encode_diff_v1(&sv).to_vec())
    }

    /// Get the full document state as an update
    pub async fn get_full_update(&self) -> Vec<u8> {
        let doc = self.doc.read().await;
        let txn = doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
            .to_vec()
    }

    /// Apply a Yjs update to the document
    pub async fn apply_update(&self, update: &[u8]) -> Result<(), String> {
        let doc = self.doc.write().await;
        let mut txn = doc.transact_mut();

        yrs::Update::decode_v1(update)
            .map_err(|e| format!("Failed to decode update: {:?}", e))
            .and_then(|update| {
                txn.apply_update(update)
                    .map_err(|e| format!("Failed to apply update: {:?}", e))
            })
    }

    // ========================================================================
    // Room Information
    // ========================================================================

    /// Get the WebSocket URL for this room
    pub fn websocket_url(&self, host: &str, secure: bool) -> String {
        let protocol = if secure { "wss" } else { "ws" };
        format!("{}://{}/room/{}", protocol, host, self.id)
    }

    /// Convert to RoomResponse for API
    pub fn to_response(&self, host: &str, secure: bool) -> RoomResponse {
        RoomResponse {
            id: self.id,
            name: self.config.name.clone(),
            websocket_url: self.websocket_url(host, secure),
            is_protected: self.is_protected(),
            user_count: self.user_count(),
            max_users: self.config.max_users,
            owner_id: self.owner_id,
            created_at: self.created_at.to_rfc3339(),
            users: self.get_users(),
        }
    }

    /// Update room configuration
    pub fn update_config(&mut self, request: &UpdateRoomRequest) -> Result<(), String> {
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                return Err("Room name cannot be empty".to_string());
            }
            self.config.name = name.clone();
        }

        if let Some(ref password) = request.password {
            self.set_password(if password.is_empty() {
                None
            } else {
                Some(password)
            })
            .map_err(|e| format!("Failed to set password: {}", e))?;
        }

        if let Some(max_users) = request.max_users {
            if max_users < self.user_count() {
                return Err(format!(
                    "Cannot set max_users to {} when {} users are connected",
                    max_users,
                    self.user_count()
                ));
            }
            self.config.max_users = max_users.clamp(MIN_USERS_PER_ROOM, MAX_USERS_PER_ROOM);
        }

        Ok(())
    }
}

// ============================================================================
// Room Manager
// ============================================================================

/// Global room manager for handling multiple rooms
pub struct RoomManager {
    /// All active rooms, keyed by room ID
    rooms: DashMap<RoomId, Arc<Room>>,
    /// Default host for WebSocket URLs
    default_host: String,
    /// Whether to use secure WebSocket (wss://)
    use_secure: bool,
}

impl RoomManager {
    /// Create a new room manager
    pub fn new(default_host: impl Into<String>, use_secure: bool) -> Self {
        Self {
            rooms: DashMap::new(),
            default_host: default_host.into(),
            use_secure,
        }
    }

    /// Get the default host
    pub fn host(&self) -> &str {
        &self.default_host
    }

    /// Whether to use secure WebSocket
    pub fn is_secure(&self) -> bool {
        self.use_secure
    }

    /// Create a new room
    pub fn create_room(
        &self,
        owner: &AuthenticatedUser,
        request: CreateRoomRequest,
    ) -> Result<Arc<Room>, String> {
        let room_id = Uuid::new_v4();
        self.create_room_with_id(room_id, owner, request)
    }

    /// Create a new room with a specific UUID
    pub fn create_room_with_id(
        &self,
        room_id: RoomId,
        owner: &AuthenticatedUser,
        request: CreateRoomRequest,
    ) -> Result<Arc<Room>, String> {
        // Check if room already exists
        if self.rooms.contains_key(&room_id) {
            return Err("Room with this ID already exists".to_string());
        }

        let mut config =
            RoomConfig::with_name(request.name.unwrap_or_else(|| "Untitled Room".to_string()));

        if let Some(max_users) = request.max_users {
            config = config.max_users(max_users);
        }

        if let Some(ref password) = request.password {
            if !password.is_empty() {
                config = config
                    .with_password(password)
                    .map_err(|e| format!("Failed to set password: {}", e))?;
            }
        }

        let room = Arc::new(Room::new(room_id, owner.user_id, config));
        self.rooms.insert(room_id, room.clone());

        Ok(room)
    }

    /// Get a room by ID
    pub fn get_room(&self, room_id: &RoomId) -> Option<Arc<Room>> {
        self.rooms.get(room_id).map(|r| r.clone())
    }

    /// Delete a room by ID
    pub fn delete_room(&self, room_id: &RoomId) -> Option<Arc<Room>> {
        self.rooms.remove(room_id).map(|(_, room)| room)
    }

    /// Get the total number of rooms
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Get the total number of connected users across all rooms
    pub fn total_user_count(&self) -> usize {
        self.rooms.iter().map(|r| r.user_count()).sum()
    }

    /// Clean up empty rooms (rooms with no connected users)
    pub fn cleanup_empty_rooms(&self) -> usize {
        let before = self.rooms.len();
        self.rooms.retain(|_, room| room.user_count() > 0);
        before - self.rooms.len()
    }

    /// Get all rooms (for admin purposes)
    pub fn list_rooms(&self) -> Vec<RoomResponse> {
        self.rooms
            .iter()
            .map(|entry| entry.to_response(&self.default_host, self.use_secure))
            .collect()
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new("localhost:3000", false)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_room() -> Room {
        Room::with_defaults(Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn test_room_creation() {
        let room = create_test_room();
        assert_eq!(room.user_count(), 0);
        assert!(!room.is_full());
        assert!(!room.is_protected());
    }

    #[test]
    fn test_password_protection() {
        let config = RoomConfig::default().with_password("secret123").unwrap();
        let room = Room::new(Uuid::new_v4(), Uuid::new_v4(), config);

        assert!(room.is_protected());
        assert!(room.verify_password(Some("secret123")));
        assert!(!room.verify_password(Some("wrong")));
        assert!(!room.verify_password(None));
    }

    #[test]
    fn test_user_management() {
        let room = create_test_room();
        let user_id = Uuid::new_v4();

        // Add user
        assert!(room.add_user(user_id, "TestUser".to_string()).is_ok());
        assert_eq!(room.user_count(), 1);
        assert!(room.has_user(&user_id));

        // Get users
        let users = room.get_users();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "TestUser");

        // Remove user
        let removed = room.remove_user(&user_id);
        assert!(removed.is_some());
        assert_eq!(room.user_count(), 0);
    }

    #[test]
    fn test_room_full() {
        let config = RoomConfig::default().max_users(2);
        let room = Room::new(Uuid::new_v4(), Uuid::new_v4(), config);

        assert!(room.add_user(Uuid::new_v4(), "User1".to_string()).is_ok());
        assert!(room.add_user(Uuid::new_v4(), "User2".to_string()).is_ok());
        assert!(room.is_full());

        // Should fail when full
        let result = room.add_user(Uuid::new_v4(), "User3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_room_manager() {
        let manager = RoomManager::default();
        let owner = AuthenticatedUser::guest();

        // Create room
        let room = manager
            .create_room(
                &owner,
                CreateRoomRequest {
                    name: Some("Test Room".to_string()),
                    password: None,
                    max_users: Some(10),
                },
            )
            .unwrap();

        assert_eq!(manager.room_count(), 1);

        // Get room
        let fetched = manager.get_room(&room.id);
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().config.name, "Test Room");

        // Delete room
        let deleted = manager.delete_room(&room.id);
        assert!(deleted.is_some());
        assert_eq!(manager.room_count(), 0);
    }

    #[test]
    fn test_websocket_url() {
        let room = create_test_room();

        let url = room.websocket_url("example.com", false);
        assert!(url.starts_with("ws://"));
        assert!(url.contains("example.com"));

        let secure_url = room.websocket_url("example.com", true);
        assert!(secure_url.starts_with("wss://"));
    }

    #[test]
    fn test_update_config() {
        let mut room = create_test_room();

        // Update name
        room.update_config(&UpdateRoomRequest {
            name: Some("New Name".to_string()),
            password: None,
            max_users: None,
        })
        .unwrap();
        assert_eq!(room.config.name, "New Name");

        // Set password
        room.update_config(&UpdateRoomRequest {
            name: None,
            password: Some("newpass".to_string()),
            max_users: None,
        })
        .unwrap();
        assert!(room.is_protected());

        // Remove password
        room.update_config(&UpdateRoomRequest {
            name: None,
            password: Some("".to_string()),
            max_users: None,
        })
        .unwrap();
        assert!(!room.is_protected());
    }
}
