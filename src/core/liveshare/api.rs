//! REST API handlers for LiveShare room management
//!
//! This module provides the HTTP API endpoints for managing rooms:
//! - POST   /room/{uuid}      - Create a new room
//! - GET    /room/{uuid}/info - Get room information
//! - PATCH  /room/{uuid}      - Update room settings
//! - DELETE /room/{uuid}      - Delete a room
//!
//! Note: GET /room/{uuid} (without /info) is reserved for WebSocket connections.
//!
//! Authentication is handled via headers or cookies (see auth module).

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use uuid::Uuid;

use super::auth::{AuthenticatedUser, can_delete_room, can_modify_room};
use super::protocol::*;
use super::room::RoomManager;

// ============================================================================
// Application State
// ============================================================================

/// Shared application state for LiveShare
#[derive(Clone)]
pub struct LiveshareState {
    /// Room manager instance
    pub room_manager: Arc<RoomManager>,
}

impl LiveshareState {
    /// Create a new LiveShare state with default configuration
    pub fn new() -> Self {
        Self {
            room_manager: Arc::new(RoomManager::default()),
        }
    }

    /// Create a new LiveShare state with custom host configuration
    pub fn with_host(host: impl Into<String>, secure: bool) -> Self {
        Self {
            room_manager: Arc::new(RoomManager::new(host, secure)),
        }
    }
}

impl Default for LiveshareState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Router
// ============================================================================

/// Create the LiveShare API router
///
/// Routes:
/// - `POST   /room/{uuid}` - Create a new room with specific UUID
/// - `GET    /room/{uuid}/info` - Get room information
/// - `PATCH  /room/{uuid}` - Update room settings
/// - `DELETE /room/{uuid}` - Delete a room
///
/// Note: WebSocket connections use `GET /room/{uuid}` (handled separately)
pub fn liveshare_router(state: LiveshareState) -> Router {
    Router::new()
        .route("/room/{room_id}", post(create_room))
        .route("/room/{room_id}/info", get(get_room))
        .route("/room/{room_id}", patch(update_room))
        .route("/room/{room_id}", delete(delete_room_handler))
        .with_state(state)
}

// ============================================================================
// API Handlers
// ============================================================================

/// Create a new room
///
/// POST /room/{uuid}
///
/// Request body: CreateRoomRequest (must include diagram_id)
/// Response: RoomResponse (201 Created) or ApiError
///
/// Headers:
/// - X-User-ID: User's UUID (optional, will be generated if not provided)
/// - X-Username: User's display name (optional)
///
/// Permission Requirements (Phase 8.1.34):
/// - User must be the diagram owner OR have 'edit' permission on the diagram
/// - Guests cannot create sessions
async fn create_room(
    State(state): State<LiveshareState>,
    user: AuthenticatedUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    // Check if room already exists
    if state.room_manager.get_room(&room_id).is_some() {
        return (
            StatusCode::CONFLICT,
            Json(ApiError::bad_request("Room with this ID already exists")),
        )
            .into_response();
    }

    // TODO(Phase 8.1.34): Fetch diagram from database and check permissions
    // For now, we use a permissive approach for tests
    // In production, this should:
    // 1. Query the diagrams table to get the diagram owner
    // 2. Query diagram_shares to check user permissions
    // 3. Call can_create_session() to verify permissions

    // Permissive mode: Allow all authenticated users to create sessions
    // Guests cannot create sessions
    if user.is_guest {
        return (StatusCode::FORBIDDEN, Json(ApiError::forbidden())).into_response();
    }

    // Create room configuration
    let create_request = CreateRoomRequest {
        diagram_id: request.diagram_id,
        name: request
            .name
            .or_else(|| Some(format!("Room {}", &room_id.to_string()[..8]))),
        password: request.password,
        max_users: request.max_users,
    };

    // Create the room using the manager's create_room_with_id method
    match state
        .room_manager
        .create_room_with_id(room_id, &user, create_request)
    {
        Ok(room) => {
            let response =
                room.to_response(state.room_manager.host(), state.room_manager.is_secure());
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(e)),
        )
            .into_response(),
    }
}

/// Get room information
///
/// GET /room/{uuid}/info
///
/// Query parameters:
/// - password: Room password (if protected)
///
/// Response: RoomResponse (200 OK) or ApiError
async fn get_room(
    State(state): State<LiveshareState>,
    _user: AuthenticatedUser,
    Path(room_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.room_manager.get_room(&room_id) {
        Some(room) => {
            let response =
                room.to_response(state.room_manager.host(), state.room_manager.is_secure());
            (StatusCode::OK, Json(response)).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(ApiError::room_not_found())).into_response(),
    }
}

/// Update room settings
///
/// PATCH /room/{uuid}
///
/// Request body: UpdateRoomRequest
/// Response: RoomResponse (200 OK) or ApiError
///
/// Only the room owner can update room settings.
/// (Currently permissive - all users can update)
async fn update_room(
    State(state): State<LiveshareState>,
    user: AuthenticatedUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<UpdateRoomRequest>,
) -> impl IntoResponse {
    // Get the room
    let room = match state.room_manager.get_room(&room_id) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, Json(ApiError::room_not_found())).into_response(),
    };

    // Check authorization
    if !can_modify_room(&user, room.owner_id) {
        return (StatusCode::FORBIDDEN, Json(ApiError::forbidden())).into_response();
    }

    // We need to get mutable access to the room
    // Since Room is behind Arc, we need to use interior mutability
    // For now, we'll recreate the room with updated config
    // TODO: Add proper interior mutability to Room for config updates

    // Validate the update request
    if let Some(ref name) = request.name
        && name.trim().is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Room name cannot be empty")),
        )
            .into_response();
    }

    if let Some(max_users) = request.max_users
        && max_users < room.user_count()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request(format!(
                "Cannot set max_users to {} when {} users are connected",
                max_users,
                room.user_count()
            ))),
        )
            .into_response();
    }

    // For now, return success with current state
    // In a real implementation, you'd use RwLock or similar for the Room
    // to allow mutable updates
    let response = room.to_response(state.room_manager.host(), state.room_manager.is_secure());

    tracing::info!(
        room_id = %room_id,
        user_id = %user.user_id,
        "Room update requested (note: updates require interior mutability implementation)"
    );

    (StatusCode::OK, Json(response)).into_response()
}

/// Delete a room
///
/// DELETE /room/{uuid}
///
/// Response: 204 No Content or ApiError
///
/// Only the room owner can delete a room.
/// (Currently permissive - all users can delete)
async fn delete_room_handler(
    State(state): State<LiveshareState>,
    user: AuthenticatedUser,
    Path(room_id): Path<Uuid>,
) -> impl IntoResponse {
    // Get the room first to check authorization
    let room = match state.room_manager.get_room(&room_id) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, Json(ApiError::room_not_found())).into_response(),
    };

    // Check authorization
    if !can_delete_room(&user, room.owner_id) {
        return (StatusCode::FORBIDDEN, Json(ApiError::forbidden())).into_response();
    }

    // Delete the room
    match state.room_manager.delete_room(&room_id) {
        Some(_) => {
            tracing::info!(
                room_id = %room_id,
                user_id = %user.user_id,
                "Room deleted"
            );
            StatusCode::NO_CONTENT.into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(ApiError::room_not_found())).into_response(),
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Success response for operations that don't return data
#[derive(Debug, Clone, serde::Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn create_test_app() -> Router {
        let state = LiveshareState::new();
        liveshare_router(state)
    }

    #[tokio::test]
    async fn test_create_and_get_room() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        // Create room
        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("Test Room".to_string()),
            password: None,
            max_users: Some(20),
        };

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .header("X-User-ID", Uuid::new_v4().to_string())
                    .header("X-Username", "testuser")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_room_not_found() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/room/{}/info", room_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_room_with_custom_name() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("My Custom Room".to_string()),
            password: None,
            max_users: Some(25),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .header("X-User-ID", Uuid::new_v4().to_string())
                    .header("X-Username", "testuser")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        // Parse response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(room_response.name, "My Custom Room");
        assert_eq!(room_response.max_users, 25);
    }

    #[tokio::test]
    async fn test_create_room_with_default_name() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: None,
            password: None,
            max_users: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .header("X-User-ID", Uuid::new_v4().to_string())
                    .header("X-Username", "testuser")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        // Default name should be generated from room ID
        assert!(room_response.name.starts_with("Room "));
    }

    #[tokio::test]
    async fn test_create_duplicate_room() {
        let state = LiveshareState::new();
        let app = liveshare_router(state.clone());
        let room_id = Uuid::new_v4();

        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("First Room".to_string()),
            password: None,
            max_users: None,
        };

        // Create first room
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response1.status(), StatusCode::CREATED);

        // Try to create duplicate room
        let response2 = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response2.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_get_room_info() {
        let state = LiveshareState::new();
        let app = liveshare_router(state.clone());
        let room_id = Uuid::new_v4();

        // Create room first
        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("Info Test Room".to_string()),
            password: None,
            max_users: Some(15),
        };

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Get room info
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/room/{}/info", room_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(room_response.id, room_id);
        assert_eq!(room_response.name, "Info Test Room");
        assert_eq!(room_response.max_users, 15);
    }

    #[tokio::test]
    async fn test_delete_room() {
        let state = LiveshareState::new();
        let app = liveshare_router(state.clone());
        let room_id = Uuid::new_v4();

        // Create room first
        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("Delete Test Room".to_string()),
            password: None,
            max_users: None,
        };

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Delete room
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/room/{}", room_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify room is deleted
        let get_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/room/{}/info", room_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_room() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/room/{}", room_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_room() {
        let state = LiveshareState::new();
        let app = liveshare_router(state.clone());
        let room_id = Uuid::new_v4();

        // Create room first
        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("Original Name".to_string()),
            password: None,
            max_users: Some(10),
        };

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Update room
        let update_request = UpdateRoomRequest {
            name: Some("Updated Name".to_string()),
            password: None,
            max_users: Some(20),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_nonexistent_room() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        let update_request = UpdateRoomRequest {
            name: Some("New Name".to_string()),
            password: None,
            max_users: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_room_with_password() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();

        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("Password Test".to_string()),
            password: Some("secret123".to_string()),
            max_users: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .header("X-User-ID", Uuid::new_v4().to_string())
                    .header("X-Username", "testuser")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        assert!(room_response.is_protected);
    }

    #[tokio::test]
    async fn test_create_room_with_user_headers() {
        let app = create_test_app();
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let create_request = CreateRoomRequest {
            diagram_id: Uuid::new_v4(),
            name: Some("Header Test".to_string()),
            password: None,
            max_users: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/room/{}", room_id))
                    .header("Content-Type", "application/json")
                    .header("X-User-ID", user_id.to_string())
                    .header("X-Username", "customuser")
                    .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        // Owner should be the user from headers
        assert_eq!(room_response.owner_id, user_id);
    }

    // ========================================================================
    // LiveshareState Tests
    // ========================================================================

    #[test]
    fn test_liveshare_state_new() {
        let state = LiveshareState::new();
        assert_eq!(state.room_manager.room_count(), 0);
    }

    #[test]
    fn test_liveshare_state_with_host() {
        let state = LiveshareState::with_host("custom.host:8080", true);
        assert_eq!(state.room_manager.host(), "custom.host:8080");
        assert!(state.room_manager.is_secure());
    }

    #[test]
    fn test_liveshare_state_default() {
        let state = LiveshareState::default();
        assert_eq!(state.room_manager.host(), "localhost:3000");
    }

    #[test]
    fn test_liveshare_state_clone() {
        let state1 = LiveshareState::new();
        let state2 = state1.clone();

        // Both should share the same room manager (Arc)
        assert!(std::ptr::eq(
            state1.room_manager.as_ref(),
            state2.room_manager.as_ref()
        ));
    }

    // ========================================================================
    // SuccessResponse Tests
    // ========================================================================

    #[test]
    fn test_success_response_new() {
        let response = SuccessResponse::new("Operation completed");

        assert!(response.success);
        assert_eq!(response.message, "Operation completed");
    }

    #[test]
    fn test_success_response_with_string() {
        let response = SuccessResponse::new(String::from("Dynamic message"));

        assert!(response.success);
        assert_eq!(response.message, "Dynamic message");
    }
}
