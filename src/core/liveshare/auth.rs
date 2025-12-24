//! Authentication and authorization for LiveShare
//!
//! This module provides authentication middleware and helpers for the LiveShare API.
//! Supports both JWT-authenticated users and guest access:
//!
//! - JWT tokens (Bearer Authorization header) -> authenticated user
//! - Guest access (X-User-ID header or auto-generated) -> guest user
//!
//! Guests can join existing LiveShare rooms but cannot create diagrams.

use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::protocol::{ApiError, UserId};

// Optional JWT validation support
// When JWT service is available, it will be used to validate tokens
#[cfg(feature = "ssr")]
use std::sync::OnceLock;

#[cfg(feature = "ssr")]
use crate::core::auth::{JwtConfig, JwtService};

/// Global JWT service for token validation (initialized once at startup)
#[cfg(feature = "ssr")]
static JWT_SERVICE: OnceLock<Option<JwtService>> = OnceLock::new();

/// Initialize the JWT service for LiveShare authentication
/// Should be called once at application startup
#[cfg(feature = "ssr")]
pub fn init_jwt_service(service: Option<JwtService>) {
    let _ = JWT_SERVICE.set(service);
}

/// Get the JWT service if available
#[cfg(feature = "ssr")]
fn get_jwt_service() -> Option<&'static JwtService> {
    JWT_SERVICE.get().and_then(|s| s.as_ref())
}

/// Initialize JWT service from environment (convenience function)
#[cfg(feature = "ssr")]
pub fn init_jwt_from_env() {
    let service = match JwtConfig::from_env() {
        Ok(config) => Some(JwtService::new(config)),
        Err(_) => {
            if cfg!(debug_assertions) {
                // Use default secret in development
                Some(JwtService::new(JwtConfig::new(
                    "archischema_dev_secret_key_not_for_production_32chars",
                )))
            } else {
                None
            }
        }
    };
    init_jwt_service(service);
}

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

/// Header name for user ID (guest mode)
pub const AUTH_HEADER_USER_ID: &str = "X-User-ID";
/// Header name for username (guest mode)
pub const AUTH_HEADER_USERNAME: &str = "X-Username";
/// Cookie name for session token
pub const AUTH_COOKIE_SESSION: &str = "archischema_session";
/// Authorization header for JWT tokens
pub const AUTH_HEADER_AUTHORIZATION: &str = "Authorization";

/// Axum extractor for authenticated users
///
/// Authentication flow:
/// 1. Check for Bearer token in Authorization header -> JWT authenticated user
/// 2. Fall back to guest mode with X-User-ID/X-Username headers
/// 3. If nothing provided, create anonymous guest
///
/// Guests can join LiveShare rooms but cannot create/save diagrams.
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // First, try JWT authentication from Authorization header
        if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION)
            && let Ok(auth_str) = auth_header.to_str()
            && auth_str.starts_with("Bearer ")
        {
            let token = auth_str.trim_start_matches("Bearer ");
            if !token.is_empty() {
                // Try to validate JWT token
                #[cfg(feature = "ssr")]
                if let Some(jwt_service) = get_jwt_service() {
                    match jwt_service.validate_access_token(token) {
                        Ok(claims) => {
                            if let Ok(user_id) = claims.user_id() {
                                return Ok(AuthenticatedUser::authenticated(
                                    user_id,
                                    claims.username,
                                    Some(claims.email),
                                ));
                            }
                        }
                        Err(e) => {
                            tracing::debug!("JWT validation failed: {:?}", e);
                            // Don't reject - fall through to guest mode
                            // This allows guests to join rooms even with invalid/expired tokens
                        }
                    }
                }
            }
        }

        // Fall back to guest mode
        // Try to get user ID from header (for reconnection with same identity)
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

        // Create guest user based on headers or generate new one
        match (user_id, username) {
            (Some(id), Some(name)) => Ok(AuthenticatedUser {
                user_id: id,
                username: name,
                email: None,
                is_guest: true,
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
    // Only room owner can modify room settings
    user.user_id == room_owner_id
}

/// Check if a user can delete a room
///
/// Currently returns `true` for all users (permissive mode).
pub fn can_delete_room(user: &AuthenticatedUser, room_owner_id: UserId) -> bool {
    // Only room owner can delete a room
    user.user_id == room_owner_id
}

/// Check if a user can join a room (basic check, doesn't verify password)
pub fn can_join_room(user: &AuthenticatedUser) -> bool {
    // All authenticated and guest users can attempt to join rooms
    // Password validation happens separately
    let _ = user;
    true
}

/// Check if a user can create a LiveShare session for a diagram
///
/// Requirements:
/// - User must be the diagram owner OR have 'edit' permission on the diagram
///
/// # Arguments
/// * `user` - The authenticated user
/// * `diagram_owner_id` - The owner of the diagram
/// * `user_permission` - The user's permission level on the diagram (None if owner)
///
/// # Returns
/// `true` if user can create a session, `false` otherwise
pub fn can_create_session(
    user: &AuthenticatedUser,
    diagram_owner_id: UserId,
    user_permission: Option<&str>,
) -> bool {
    // User must be diagram owner OR have 'edit' permission
    if user.user_id == diagram_owner_id {
        return true;
    }

    // Check if user has edit permission via diagram_shares
    matches!(user_permission, Some("edit"))
}

/// Check if a user can connect to a LiveShare session for a diagram
///
/// Requirements:
/// - User must be the diagram owner, OR
/// - Have any permission level ('view' or 'edit') on the diagram
///
/// # Arguments
/// * `user` - The authenticated user
/// * `diagram_owner_id` - The owner of the diagram
/// * `user_permission` - The user's permission level on the diagram (None if owner)
///
/// # Returns
/// `true` if user can join the session, `false` otherwise
pub fn can_join_session(
    user: &AuthenticatedUser,
    diagram_owner_id: UserId,
    user_permission: Option<&str>,
) -> bool {
    // User must be diagram owner OR have any permission level
    if user.user_id == diagram_owner_id {
        return true;
    }

    // Check if user has any permission via diagram_shares
    // Guests cannot join shared sessions
    !user.is_guest && user_permission.is_some()
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

    // ========================================================================
    // AuthenticatedUser Tests
    // ========================================================================

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
    fn test_guest_username_contains_id_prefix() {
        let id = Uuid::new_v4();
        let guest = AuthenticatedUser::guest_with_id(id);

        // Username should contain first 8 chars of UUID
        let expected_suffix = &id.to_string()[..8];
        assert!(guest.username.contains(expected_suffix));
    }

    #[test]
    fn test_authenticated_user_creation() {
        let user_id = Uuid::new_v4();
        let user = AuthenticatedUser::authenticated(
            user_id,
            "John Doe".to_string(),
            Some("john@example.com".to_string()),
        );

        assert_eq!(user.user_id, user_id);
        assert_eq!(user.username, "John Doe");
        assert_eq!(user.email, Some("john@example.com".to_string()));
        assert!(!user.is_guest);
    }

    #[test]
    fn test_authenticated_user_without_email() {
        let user_id = Uuid::new_v4();
        let user = AuthenticatedUser::authenticated(user_id, "Jane".to_string(), None);

        assert!(!user.is_guest);
        assert!(user.email.is_none());
    }

    #[test]
    fn test_authenticated_user_clone() {
        let user = AuthenticatedUser::guest();
        let cloned = user.clone();

        assert_eq!(user.user_id, cloned.user_id);
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.is_guest, cloned.is_guest);
    }

    #[test]
    fn test_authenticated_user_debug() {
        let user = AuthenticatedUser::guest();
        let debug_str = format!("{:?}", user);

        assert!(debug_str.contains("AuthenticatedUser"));
        assert!(debug_str.contains("user_id"));
        assert!(debug_str.contains("is_guest"));
    }

    #[test]
    fn test_authenticated_user_serialization() {
        let user = AuthenticatedUser::authenticated(
            Uuid::new_v4(),
            "TestUser".to_string(),
            Some("test@example.com".to_string()),
        );

        let json = serde_json::to_string(&user).unwrap();
        let parsed: AuthenticatedUser = serde_json::from_str(&json).unwrap();

        assert_eq!(user.user_id, parsed.user_id);
        assert_eq!(user.username, parsed.username);
        assert_eq!(user.email, parsed.email);
        assert_eq!(user.is_guest, parsed.is_guest);
    }

    // ========================================================================
    // Cookie Extraction Tests
    // ========================================================================

    #[test]
    fn test_extract_cookie() {
        let cookies = "session=abc123; theme=dark; archischema_session=xyz789";

        assert_eq!(
            extract_cookie(cookies, "session"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_cookie(cookies, "archischema_session"),
            Some("xyz789".to_string())
        );
        assert_eq!(extract_cookie(cookies, "nonexistent"), None);
    }

    #[test]
    fn test_extract_cookie_single() {
        let cookies = "only_cookie=value123";
        assert_eq!(
            extract_cookie(cookies, "only_cookie"),
            Some("value123".to_string())
        );
    }

    #[test]
    fn test_extract_cookie_empty_string() {
        let cookies = "";
        assert_eq!(extract_cookie(cookies, "any"), None);
    }

    #[test]
    fn test_extract_cookie_with_spaces() {
        let cookies = "  spaced = value ;  another=test  ";
        // The current implementation trims spaces
        assert_eq!(extract_cookie(cookies, "another"), Some("test".to_string()));
    }

    #[test]
    fn test_extract_cookie_with_special_chars_in_value() {
        let cookies = "token=abc123!@#$%^&*()";
        assert_eq!(
            extract_cookie(cookies, "token"),
            Some("abc123!@#$%^&*()".to_string())
        );
    }

    #[test]
    fn test_extract_cookie_partial_match() {
        let cookies = "session_id=123; session=456";
        // Should match exact cookie name, not partial
        assert_eq!(extract_cookie(cookies, "session"), Some("456".to_string()));
    }

    // ========================================================================
    // Authorization Tests
    // ========================================================================

    #[test]
    fn test_authorization_permissive() {
        let user = AuthenticatedUser::guest();
        let owner_id = Uuid::new_v4();

        // In permissive mode, all checks should pass
        assert!(can_modify_room(&user, owner_id));
        assert!(can_delete_room(&user, owner_id));
        assert!(can_join_room(&user));
    }

    #[test]
    fn test_can_modify_room_as_owner() {
        let user_id = Uuid::new_v4();
        let user = AuthenticatedUser::guest_with_id(user_id);

        // Even when user is the owner, permissive mode returns true
        assert!(can_modify_room(&user, user_id));
    }

    #[test]
    fn test_can_modify_room_as_non_owner() {
        let user = AuthenticatedUser::guest();
        let different_owner = Uuid::new_v4();

        // Permissive mode - non-owner can still modify
        assert!(can_modify_room(&user, different_owner));
    }

    #[test]
    fn test_can_delete_room_as_owner() {
        let user_id = Uuid::new_v4();
        let user = AuthenticatedUser::guest_with_id(user_id);

        assert!(can_delete_room(&user, user_id));
    }

    #[test]
    fn test_can_delete_room_as_non_owner() {
        let user = AuthenticatedUser::guest();
        let different_owner = Uuid::new_v4();

        // Permissive mode - non-owner can still delete
        assert!(can_delete_room(&user, different_owner));
    }

    #[test]
    fn test_can_join_room_guest() {
        let guest = AuthenticatedUser::guest();
        assert!(can_join_room(&guest));
    }

    #[test]
    fn test_can_join_room_authenticated() {
        let user = AuthenticatedUser::authenticated(
            Uuid::new_v4(),
            "AuthUser".to_string(),
            Some("auth@example.com".to_string()),
        );
        assert!(can_join_room(&user));
    }

    // ========================================================================
    // AuthError Tests
    // ========================================================================

    #[test]
    fn test_auth_error_unauthorized() {
        let error = AuthError::unauthorized("Please login");

        assert_eq!(error.message, "Please login");
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_forbidden() {
        let error = AuthError::forbidden("Access denied");

        assert_eq!(error.message, "Access denied");
        assert_eq!(error.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_auth_error_unauthorized_with_string() {
        let error = AuthError::unauthorized(String::from("Dynamic message"));
        assert_eq!(error.message, "Dynamic message");
    }

    #[test]
    fn test_auth_error_forbidden_with_string() {
        let error = AuthError::forbidden(String::from("No permission"));
        assert_eq!(error.message, "No permission");
    }

    #[test]
    fn test_auth_error_debug() {
        let error = AuthError::unauthorized("Test error");
        let debug_str = format!("{:?}", error);

        assert!(debug_str.contains("AuthError"));
        assert!(debug_str.contains("Test error"));
    }

    // ========================================================================
    // Constants Tests
    // ========================================================================

    #[test]
    fn test_auth_header_constants() {
        assert_eq!(AUTH_HEADER_USER_ID, "X-User-ID");
        assert_eq!(AUTH_HEADER_USERNAME, "X-Username");
        assert_eq!(AUTH_COOKIE_SESSION, "archischema_session");
    }

    // ========================================================================
    // OptionalUser Tests
    // ========================================================================

    #[test]
    fn test_optional_user_with_some() {
        let user = AuthenticatedUser::guest();
        let optional = OptionalUser(Some(user.clone()));

        assert!(optional.0.is_some());
        assert_eq!(optional.0.unwrap().user_id, user.user_id);
    }

    #[test]
    fn test_optional_user_with_none() {
        let optional = OptionalUser(None);
        assert!(optional.0.is_none());
    }

    #[test]
    fn test_optional_user_clone() {
        let user = AuthenticatedUser::guest();
        let optional = OptionalUser(Some(user));
        let cloned = optional.clone();

        assert!(cloned.0.is_some());
    }

    // ========================================================================
    // Tests for diagram-based permission checks (Phase 8)
    // ========================================================================

    #[test]
    fn test_can_create_session_as_diagram_owner() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = user.user_id;

        // Owner can create session
        assert!(can_create_session(&user, diagram_owner_id, None));
    }

    #[test]
    fn test_can_create_session_with_edit_permission() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = uuid::Uuid::new_v4(); // Different owner

        // User with 'edit' permission can create session
        assert!(can_create_session(&user, diagram_owner_id, Some("edit")));
    }

    #[test]
    fn test_can_create_session_with_view_permission_fails() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = uuid::Uuid::new_v4(); // Different owner

        // User with only 'view' permission cannot create session
        assert!(!can_create_session(&user, diagram_owner_id, Some("view")));
    }

    #[test]
    fn test_can_create_session_no_permission_fails() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = uuid::Uuid::new_v4(); // Different owner

        // User with no permission cannot create session
        assert!(!can_create_session(&user, diagram_owner_id, None));
    }

    #[test]
    fn test_can_join_session_as_diagram_owner() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = user.user_id;

        // Owner can join session
        assert!(can_join_session(&user, diagram_owner_id, None));
    }

    #[test]
    fn test_can_join_session_with_edit_permission() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = uuid::Uuid::new_v4(); // Different owner

        // User with 'edit' permission can join session
        assert!(can_join_session(&user, diagram_owner_id, Some("edit")));
    }

    #[test]
    fn test_can_join_session_with_view_permission() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = uuid::Uuid::new_v4(); // Different owner

        // User with 'view' permission can join session
        assert!(can_join_session(&user, diagram_owner_id, Some("view")));
    }

    #[test]
    fn test_can_join_session_no_permission_fails() {
        let user = AuthenticatedUser::authenticated(
            uuid::Uuid::new_v4(),
            "testuser".to_string(),
            Some("test@example.com".to_string()),
        );
        let diagram_owner_id = uuid::Uuid::new_v4(); // Different owner

        // User with no permission cannot join session
        assert!(!can_join_session(&user, diagram_owner_id, None));
    }

    #[test]
    fn test_can_join_session_guest_fails() {
        let user = AuthenticatedUser::guest();
        let diagram_owner_id = uuid::Uuid::new_v4();

        // Guest cannot join shared session even with permission
        assert!(!can_join_session(&user, diagram_owner_id, Some("view")));
    }
}
