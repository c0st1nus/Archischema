//! Authentication and authorization for LiveShare
//!
//! This module provides authentication middleware and helpers for the LiveShare API.
//! Currently implements a permissive "stub" auth that allows all users.
//!
//! TODO: Implement proper authentication when user registration is added:
//! - JWT token validation
//! - Session management
//! - User database integration

use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::protocol::{ApiError, UserId};

// ============================================================================
// Authentication Types
// ============================================================================

/// Authenticated user information extracted from request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    /// User's unique identifier
    pub user_id: UserId,
    /// User's display name
    pub username: String,
    /// User's email (if available)
    pub email: Option<String>,
    /// Whether this is a guest/anonymous user
    pub is_guest: bool,
}

impl AuthenticatedUser {
    /// Create a guest user with a random ID
    pub fn guest() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: format!("Guest_{}", &Uuid::new_v4().to_string()[..8]),
            email: None,
            is_guest: true,
        }
    }

    /// Create a guest user with a specific ID
    pub fn guest_with_id(user_id: UserId) -> Self {
        Self {
            user_id,
            username: format!("Guest_{}", &user_id.to_string()[..8]),
            email: None,
            is_guest: true,
        }
    }

    /// Create an authenticated user (for future use)
    #[allow(dead_code)]
    pub fn authenticated(user_id: UserId, username: String, email: Option<String>) -> Self {
        Self {
            user_id,
            username,
            email,
            is_guest: false,
        }
    }
}

// ============================================================================
// Authentication Extractor
// ============================================================================

/// Header name for user ID authentication
pub const AUTH_HEADER_USER_ID: &str = "X-User-ID";
/// Header name for username
pub const AUTH_HEADER_USERNAME: &str = "X-Username";
/// Cookie name for session token (future use)
pub const AUTH_COOKIE_SESSION: &str = "diagramix_session";

/// Axum extractor for authenticated users
///
/// Currently implements permissive authentication:
/// - If `X-User-ID` header is present, uses that user ID
/// - If `X-Username` header is present, uses that username
/// - Otherwise, creates a guest user
///
/// # Future Implementation
/// When user registration is implemented, this will:
/// 1. Check for session cookie or Authorization header
/// 2. Validate JWT token
/// 3. Look up user in database
/// 4. Return authenticated user or reject request
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get user ID from header
        let user_id = parts
            .headers
            .get(AUTH_HEADER_USER_ID)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok());

        // Try to get username from header
        let username = parts
            .headers
            .get(AUTH_HEADER_USERNAME)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // TODO: Check for session cookie
        // let session_cookie = parts
        //     .headers
        //     .get(header::COOKIE)
        //     .and_then(|v| v.to_str().ok())
        //     .and_then(|cookies| extract_cookie(cookies, AUTH_COOKIE_SESSION));

        // TODO: Validate session/JWT and look up user in database
        // if let Some(session) = session_cookie {
        //     return validate_session(session).await;
        // }

        // For now, create guest user based on headers or generate new one
        match (user_id, username) {
            (Some(id), Some(name)) => Ok(AuthenticatedUser {
                user_id: id,
                username: name,
                email: None,
                is_guest: true, // Still a guest until proper auth is implemented
            }),
            (Some(id), None) => Ok(AuthenticatedUser::guest_with_id(id)),
            (None, Some(name)) => Ok(AuthenticatedUser {
                user_id: Uuid::new_v4(),
                username: name,
                email: None,
                is_guest: true,
            }),
            (None, None) => Ok(AuthenticatedUser::guest()),
        }
    }
}

// ============================================================================
// Optional Authentication Extractor
// ============================================================================

/// Optional authenticated user - never fails, always returns Some or None
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<AuthenticatedUser>);

impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalUser(
            AuthenticatedUser::from_request_parts(parts, state)
                .await
                .ok(),
        ))
    }
}

// ============================================================================
// Authorization Helpers
// ============================================================================

/// Check if a user can modify a room
///
/// Currently returns `true` for all users (permissive mode).
/// When proper auth is implemented, this will check:
/// - If user is the room owner
/// - If user has admin/moderator role
pub fn can_modify_room(user: &AuthenticatedUser, room_owner_id: UserId) -> bool {
    // TODO: Implement proper authorization
    // For now, allow everyone (permissive mode for development)
    //
    // Future implementation:
    // user.user_id == room_owner_id || user.is_admin()

    let _ = (user, room_owner_id); // Suppress unused warnings
    true
}

/// Check if a user can delete a room
///
/// Currently returns `true` for all users (permissive mode).
pub fn can_delete_room(user: &AuthenticatedUser, room_owner_id: UserId) -> bool {
    // TODO: Implement proper authorization
    // Only owner should be able to delete
    //
    // Future implementation:
    // user.user_id == room_owner_id

    let _ = (user, room_owner_id);
    true
}

/// Check if a user can join a room (basic check, doesn't verify password)
pub fn can_join_room(user: &AuthenticatedUser) -> bool {
    // Currently all users can attempt to join rooms
    let _ = user;
    true
}

// ============================================================================
// Authentication Errors
// ============================================================================

/// Authentication error type
#[derive(Debug)]
pub struct AuthError {
    pub message: String,
    pub status: StatusCode,
}

impl AuthError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::FORBIDDEN,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let body = Json(ApiError {
            error: self.message,
            code: if self.status == StatusCode::UNAUTHORIZED {
                super::protocol::ApiErrorCode::Unauthorized
            } else {
                super::protocol::ApiErrorCode::Forbidden
            },
        });

        (self.status, body).into_response()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract a specific cookie value from the Cookie header
#[allow(dead_code)]
fn extract_cookie(cookies: &str, name: &str) -> Option<String> {
    cookies
        .split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with(&format!("{}=", name)))
        .map(|s| s[name.len() + 1..].to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_user_creation() {
        let guest = AuthenticatedUser::guest();
        assert!(guest.is_guest);
        assert!(guest.username.starts_with("Guest_"));
        assert!(guest.email.is_none());
    }

    #[test]
    fn test_guest_with_id() {
        let id = Uuid::new_v4();
        let guest = AuthenticatedUser::guest_with_id(id);
        assert_eq!(guest.user_id, id);
        assert!(guest.is_guest);
    }

    #[test]
    fn test_extract_cookie() {
        let cookies = "session=abc123; theme=dark; diagramix_session=xyz789";

        assert_eq!(
            extract_cookie(cookies, "session"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_cookie(cookies, "diagramix_session"),
            Some("xyz789".to_string())
        );
        assert_eq!(extract_cookie(cookies, "nonexistent"), None);
    }

    #[test]
    fn test_authorization_permissive() {
        let user = AuthenticatedUser::guest();
        let owner_id = Uuid::new_v4();

        // In permissive mode, all checks should pass
        assert!(can_modify_room(&user, owner_id));
        assert!(can_delete_room(&user, owner_id));
        assert!(can_join_room(&user));
    }
}
