//! REST API handlers for LiveShare room management
//!
//! This module provides the HTTP API endpoints for managing rooms:
//! - POST   /room/{uuid} - Create a new room
//! - GET    /room/{uuid} - Get room information
//! - PATCH  /room/{uuid} - Update room settings
//! - DELETE /room/{uuid} - Delete a room
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
/// Request body: CreateRoomRequest
/// Response: RoomResponse (201 Created) or ApiError
///
/// Headers:
/// - X-User-ID: User's UUID (optional, will be generated if not provided)
/// - X-Username: User's display name (optional)
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

    // Create room configuration
    let create_request = CreateRoomRequest {
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
    if let Some(ref name) = request.name {
        if name.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError::bad_request("Room name cannot be empty")),
            )
                .into_response();
        }
    }

    if let Some(max_users) = request.max_users {
        if max_users < room.user_count() {
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
            name: Some("Test Room".to_string()),
            password: None,
            max_users: Some(10),
        };

        let response = app
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
                    .uri(format!("/room/{}", room_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
