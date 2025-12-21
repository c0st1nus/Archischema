//! Auth API endpoints
//!
//! Provides REST API endpoints for authentication:
//! - POST /api/auth/register - Register a new user
//! - POST /api/auth/login - Login and get tokens
//! - POST /api/auth/logout - Logout (invalidate refresh token)
//! - POST /api/auth/refresh - Refresh access token
//! - GET /api/auth/me - Get current user info

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::core::auth::{
    AuthError, AuthResponse, AuthService, LoginRequest, RefreshRequest, RegisterRequest, TokenPair,
};
use crate::core::db::models::UserResponse;

/// Auth API state containing the auth service
#[derive(Clone)]
pub struct AuthApiState {
    pub auth_service: AuthService,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
}

impl ApiError {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
        }
    }
}

/// Convert AuthError to API response
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
            AuthError::UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND"),
            AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, "EMAIL_EXISTS"),
            AuthError::UsernameAlreadyExists => (StatusCode::CONFLICT, "USERNAME_EXISTS"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED"),
            AuthError::SessionNotFound => (StatusCode::UNAUTHORIZED, "SESSION_NOT_FOUND"),
            AuthError::PasswordTooShort => (StatusCode::BAD_REQUEST, "PASSWORD_TOO_SHORT"),
            AuthError::PasswordTooWeak => (StatusCode::BAD_REQUEST, "PASSWORD_TOO_WEAK"),
            AuthError::InvalidEmail => (StatusCode::BAD_REQUEST, "INVALID_EMAIL"),
            AuthError::InvalidUsername => (StatusCode::BAD_REQUEST, "INVALID_USERNAME"),
            AuthError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = ApiError::new(self.to_string(), code);

        (status, Json(body)).into_response()
    }
}

/// Response wrapper for successful auth operations
#[derive(Debug, Serialize)]
pub struct AuthApiResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}

impl From<AuthResponse> for AuthApiResponse {
    fn from(resp: AuthResponse) -> Self {
        Self {
            user: resp.user,
            tokens: resp.tokens,
        }
    }
}

/// Response for token refresh
#[derive(Debug, Serialize)]
pub struct RefreshApiResponse {
    pub tokens: TokenPair,
}

/// Response for logout
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// Request for changing password
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Create the auth API router
pub fn auth_api_router(state: AuthApiState) -> Router {
    let state = Arc::new(state);

    Router::new()
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/auth/refresh", post(refresh_handler))
        .route("/api/auth/me", get(me_handler))
        .route("/api/auth/password", post(change_password_handler))
        .with_state(state)
}

/// POST /api/auth/register
/// Register a new user
async fn register_handler(
    State(state): State<Arc<AuthApiState>>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<AuthApiResponse>, AuthError> {
    tracing::info!("Registration attempt for email: {}", request.email);

    let response = state.auth_service.register(request).await?;

    tracing::info!("User registered successfully: {}", response.user.email);

    Ok(Json(response.into()))
}

/// POST /api/auth/login
/// Login and get access/refresh tokens
async fn login_handler(
    State(state): State<Arc<AuthApiState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthApiResponse>, AuthError> {
    tracing::info!("Login attempt for email: {}", request.email);

    let response = state.auth_service.login(request).await?;

    tracing::info!("User logged in successfully: {}", response.user.email);

    Ok(Json(response.into()))
}

/// POST /api/auth/logout
/// Logout and invalidate refresh token
async fn logout_handler(
    State(state): State<Arc<AuthApiState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<LogoutResponse>, AuthError> {
    tracing::info!("Logout request");

    state.auth_service.logout(&request.refresh_token).await?;

    Ok(Json(LogoutResponse {
        message: "Logged out successfully".to_string(),
    }))
}

/// POST /api/auth/refresh
/// Refresh access token using refresh token
async fn refresh_handler(
    State(state): State<Arc<AuthApiState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshApiResponse>, AuthError> {
    tracing::debug!("Token refresh request");

    let tokens = state.auth_service.refresh(request).await?;

    Ok(Json(RefreshApiResponse { tokens }))
}

/// GET /api/auth/me
/// Get current user info from access token
async fn me_handler(
    State(state): State<Arc<AuthApiState>>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, AuthError> {
    let token = extract_bearer_token(&headers)?;

    let user = state.auth_service.get_current_user(&token).await?;

    Ok(Json(user))
}

/// POST /api/auth/password
/// Change password (requires current password)
async fn change_password_handler(
    State(state): State<Arc<AuthApiState>>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<SuccessResponse>, AuthError> {
    let token = extract_bearer_token(&headers)?;

    // Get user ID from token
    let user_id = state.auth_service.validate_access_token(&token)?;

    // Change password
    state
        .auth_service
        .change_password(user_id, &request.current_password, &request.new_password)
        .await?;

    tracing::info!("Password changed for user: {}", user_id);

    Ok(Json(SuccessResponse {
        success: true,
        message: "Password changed successfully. Please login again.".to_string(),
    }))
}

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Result<String, AuthError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::InvalidToken)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AuthError::InvalidToken);
    }

    let token = auth_header.trim_start_matches("Bearer ").to_string();

    if token.is_empty() {
        return Err(AuthError::InvalidToken);
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_bearer_token_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer my_token_123"),
        );

        let token = extract_bearer_token(&headers).unwrap();
        assert_eq!(token, "my_token_123");
    }

    #[test]
    fn test_extract_bearer_token_missing_header() {
        let headers = HeaderMap::new();

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic base64credentials"),
        );

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_api_error_serialization() {
        let error = ApiError::new("Something went wrong", "ERROR_CODE");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("Something went wrong"));
        assert!(json.contains("ERROR_CODE"));
    }

    #[test]
    fn test_auth_api_response_from_auth_response() {
        use chrono::Utc;
        use uuid::Uuid;

        let auth_response = AuthResponse {
            user: UserResponse {
                id: Uuid::new_v4(),
                email: "test@example.com".to_string(),
                username: "testuser".to_string(),
                avatar_url: None,
                created_at: Utc::now(),
            },
            tokens: TokenPair {
                access_token: "access123".to_string(),
                refresh_token: "refresh456".to_string(),
                access_expires_at: 123456789,
                refresh_expires_at: 987654321,
                token_type: "Bearer".to_string(),
            },
        };

        let api_response: AuthApiResponse = auth_response.into();

        assert_eq!(api_response.user.email, "test@example.com");
        assert_eq!(api_response.tokens.access_token, "access123");
    }

    #[test]
    fn test_logout_response_serialization() {
        let response = LogoutResponse {
            message: "Logged out successfully".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("Logged out successfully"));
    }

    #[test]
    fn test_success_response_serialization() {
        let response = SuccessResponse {
            success: true,
            message: "Operation completed".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("true"));
        assert!(json.contains("Operation completed"));
    }

    #[test]
    fn test_change_password_request_deserialization() {
        let json = r#"{
            "current_password": "OldPassword123",
            "new_password": "NewPassword456"
        }"#;

        let request: ChangePasswordRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.current_password, "OldPassword123");
        assert_eq!(request.new_password, "NewPassword456");
    }
}
