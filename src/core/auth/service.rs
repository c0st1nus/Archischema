//! Authentication service
//!
//! Provides business logic for user registration, login, logout, and token refresh.
//! Coordinates between user repository, session repository, and JWT service.

use uuid::Uuid;

use crate::core::auth::jwt::{JwtError, JwtService, TokenPair};
use crate::core::db::models::UserResponse;
use crate::core::db::repositories::{
    SessionRepository, SessionRepositoryError, UserRepository, UserRepositoryError,
};

/// Authentication service error types
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("Email already registered")]
    EmailAlreadyExists,

    #[error("Username already taken")]
    UsernameAlreadyExists,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Session not found or expired")]
    SessionNotFound,

    #[error("Password too short (minimum 8 characters)")]
    PasswordTooShort,

    #[error("Password too weak")]
    PasswordTooWeak,

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Invalid username format")]
    InvalidUsername,

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<UserRepositoryError> for AuthError {
    fn from(err: UserRepositoryError) -> Self {
        match err {
            UserRepositoryError::NotFound => AuthError::UserNotFound,
            UserRepositoryError::EmailAlreadyExists => AuthError::EmailAlreadyExists,
            UserRepositoryError::UsernameAlreadyExists => AuthError::UsernameAlreadyExists,
            UserRepositoryError::InvalidPassword => AuthError::InvalidCredentials,
            _ => AuthError::InternalError(err.to_string()),
        }
    }
}

impl From<SessionRepositoryError> for AuthError {
    fn from(err: SessionRepositoryError) -> Self {
        match err {
            SessionRepositoryError::NotFound => AuthError::SessionNotFound,
            SessionRepositoryError::Expired => AuthError::TokenExpired,
            _ => AuthError::InternalError(err.to_string()),
        }
    }
}

impl From<JwtError> for AuthError {
    fn from(err: JwtError) -> Self {
        match err {
            JwtError::Expired => AuthError::TokenExpired,
            JwtError::InvalidToken | JwtError::InvalidTokenType => AuthError::InvalidToken,
            _ => AuthError::InternalError(err.to_string()),
        }
    }
}

/// Registration request data
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub username: String,
}

/// Login request data
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Authentication response with user data and tokens
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}

/// Token refresh request
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Authentication service
#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    session_repo: SessionRepository,
    jwt_service: JwtService,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(
        user_repo: UserRepository,
        session_repo: SessionRepository,
        jwt_service: JwtService,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            jwt_service,
        }
    }

    /// Validate email format
    fn validate_email(email: &str) -> Result<(), AuthError> {
        // Basic email validation
        if email.is_empty() {
            return Err(AuthError::InvalidEmail);
        }

        if !email.contains('@') || !email.contains('.') {
            return Err(AuthError::InvalidEmail);
        }

        // Check for valid structure: something@something.something
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(AuthError::InvalidEmail);
        }

        let local = parts[0];
        let domain = parts[1];

        if local.is_empty() || domain.is_empty() {
            return Err(AuthError::InvalidEmail);
        }

        if !domain.contains('.') {
            return Err(AuthError::InvalidEmail);
        }

        // Check domain has something after the dot
        let domain_parts: Vec<&str> = domain.split('.').collect();
        if domain_parts.iter().any(|p| p.is_empty()) {
            return Err(AuthError::InvalidEmail);
        }

        Ok(())
    }

    /// Validate username format
    fn validate_username(username: &str) -> Result<(), AuthError> {
        // Username must be 3-50 characters
        if username.len() < 3 || username.len() > 50 {
            return Err(AuthError::InvalidUsername);
        }

        // Username must start with a letter
        if !username
            .chars()
            .next()
            .map(|c| c.is_alphabetic())
            .unwrap_or(false)
        {
            return Err(AuthError::InvalidUsername);
        }

        // Username can only contain letters, numbers, underscores, and hyphens
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AuthError::InvalidUsername);
        }

        Ok(())
    }

    /// Validate password strength
    fn validate_password(password: &str) -> Result<(), AuthError> {
        // Minimum length of 8 characters
        if password.len() < 8 {
            return Err(AuthError::PasswordTooShort);
        }

        // Check for password complexity (at least one of each: uppercase, lowercase, digit)
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());

        if !has_uppercase || !has_lowercase || !has_digit {
            return Err(AuthError::PasswordTooWeak);
        }

        Ok(())
    }

    /// Register a new user
    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AuthError> {
        // Validate input
        Self::validate_email(&request.email)?;
        Self::validate_username(&request.username)?;
        Self::validate_password(&request.password)?;

        // Create user (password will be hashed in repository)
        let user = self
            .user_repo
            .create(&request.email, &request.password, &request.username, None)
            .await?;

        // Generate tokens
        let tokens = self
            .jwt_service
            .generate_token_pair(user.id, &user.email, &user.username)?;

        // Store refresh token in session
        self.session_repo
            .create(
                user.id,
                &tokens.refresh_token,
                Some(self.jwt_service.refresh_token_expiration_days()),
            )
            .await?;

        Ok(AuthResponse {
            user: user.into(),
            tokens,
        })
    }

    /// Login an existing user
    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AuthError> {
        // Authenticate user
        let user = self
            .user_repo
            .authenticate(&request.email, &request.password)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        // Generate tokens
        let tokens = self
            .jwt_service
            .generate_token_pair(user.id, &user.email, &user.username)?;

        // Store refresh token in session
        self.session_repo
            .create(
                user.id,
                &tokens.refresh_token,
                Some(self.jwt_service.refresh_token_expiration_days()),
            )
            .await?;

        Ok(AuthResponse {
            user: user.into(),
            tokens,
        })
    }

    /// Logout a user (invalidate refresh token)
    pub async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        self.session_repo.delete_by_token(refresh_token).await?;
        Ok(())
    }

    /// Logout from all devices (invalidate all refresh tokens for user)
    pub async fn logout_all(&self, user_id: Uuid) -> Result<u64, AuthError> {
        let count = self.session_repo.delete_all_for_user(user_id).await?;
        Ok(count)
    }

    /// Refresh access token using refresh token
    pub async fn refresh(&self, request: RefreshRequest) -> Result<TokenPair, AuthError> {
        // Validate the refresh token JWT
        let claims = self
            .jwt_service
            .validate_refresh_token(&request.refresh_token)?;

        // Check if session exists and is valid
        let session = self
            .session_repo
            .validate_token(&request.refresh_token)
            .await?
            .ok_or(AuthError::SessionNotFound)?;

        // Get user to ensure they still exist
        let user_id = claims.user_id()?;
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        // Delete old session
        self.session_repo.delete(session.id).await?;

        // Generate new token pair
        let tokens = self
            .jwt_service
            .generate_token_pair(user.id, &user.email, &user.username)?;

        // Store new refresh token
        self.session_repo
            .create(
                user.id,
                &tokens.refresh_token,
                Some(self.jwt_service.refresh_token_expiration_days()),
            )
            .await?;

        Ok(tokens)
    }

    /// Get current user from access token
    pub async fn get_current_user(&self, access_token: &str) -> Result<UserResponse, AuthError> {
        // Validate access token
        let claims = self.jwt_service.validate_access_token(access_token)?;

        // Get user
        let user_id = claims.user_id()?;
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        Ok(user.into())
    }

    /// Validate an access token and return the user ID if valid
    pub fn validate_access_token(&self, token: &str) -> Result<Uuid, AuthError> {
        let claims = self.jwt_service.validate_access_token(token)?;
        Ok(claims.user_id()?)
    }

    /// Change user password
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        // Get user
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        // Verify current password
        let is_valid = UserRepository::verify_password(current_password, &user.password_hash)
            .map_err(|e| AuthError::InternalError(e.to_string()))?;

        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        // Validate new password
        Self::validate_password(new_password)?;

        // Update password
        self.user_repo
            .update_password(user_id, new_password)
            .await?;

        // Invalidate all existing sessions (force re-login)
        self.session_repo.delete_all_for_user(user_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Validation Tests
    // ========================================================================

    #[test]
    fn test_validate_email_valid() {
        assert!(AuthService::validate_email("user@example.com").is_ok());
        assert!(AuthService::validate_email("user.name@example.com").is_ok());
        assert!(AuthService::validate_email("user+tag@example.co.uk").is_ok());
        assert!(AuthService::validate_email("a@b.co").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(AuthService::validate_email("").is_err());
        assert!(AuthService::validate_email("invalid").is_err());
        assert!(AuthService::validate_email("@example.com").is_err());
        assert!(AuthService::validate_email("user@").is_err());
        assert!(AuthService::validate_email("user@example").is_err());
        assert!(AuthService::validate_email("user@@example.com").is_err());
        assert!(AuthService::validate_email("user@.com").is_err());
        assert!(AuthService::validate_email("user@example.").is_err());
    }

    #[test]
    fn test_validate_username_valid() {
        assert!(AuthService::validate_username("user").is_ok());
        assert!(AuthService::validate_username("user123").is_ok());
        assert!(AuthService::validate_username("user_name").is_ok());
        assert!(AuthService::validate_username("user-name").is_ok());
        assert!(AuthService::validate_username("User123_test-name").is_ok());
    }

    #[test]
    fn test_validate_username_invalid() {
        assert!(AuthService::validate_username("").is_err()); // empty
        assert!(AuthService::validate_username("ab").is_err()); // too short
        assert!(AuthService::validate_username("a".repeat(51).as_str()).is_err()); // too long
        assert!(AuthService::validate_username("123user").is_err()); // starts with number
        assert!(AuthService::validate_username("_user").is_err()); // starts with underscore
        assert!(AuthService::validate_username("user name").is_err()); // contains space
        assert!(AuthService::validate_username("user@name").is_err()); // contains @
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(AuthService::validate_password("Password1").is_ok());
        assert!(AuthService::validate_password("MyP@ssw0rd!").is_ok());
        assert!(AuthService::validate_password("Abcdefg1").is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        assert!(matches!(
            AuthService::validate_password("Pass1"),
            Err(AuthError::PasswordTooShort)
        ));
        assert!(matches!(
            AuthService::validate_password("Abc123"),
            Err(AuthError::PasswordTooShort)
        ));
    }

    #[test]
    fn test_validate_password_too_weak() {
        // No uppercase
        assert!(matches!(
            AuthService::validate_password("password1"),
            Err(AuthError::PasswordTooWeak)
        ));
        // No lowercase
        assert!(matches!(
            AuthService::validate_password("PASSWORD1"),
            Err(AuthError::PasswordTooWeak)
        ));
        // No digit
        assert!(matches!(
            AuthService::validate_password("Password"),
            Err(AuthError::PasswordTooWeak)
        ));
    }

    // ========================================================================
    // Error Conversion Tests
    // ========================================================================

    #[test]
    fn test_auth_error_display() {
        assert_eq!(
            format!("{}", AuthError::InvalidCredentials),
            "Invalid credentials"
        );
        assert_eq!(format!("{}", AuthError::UserNotFound), "User not found");
        assert_eq!(
            format!("{}", AuthError::EmailAlreadyExists),
            "Email already registered"
        );
        assert_eq!(
            format!("{}", AuthError::UsernameAlreadyExists),
            "Username already taken"
        );
        assert_eq!(format!("{}", AuthError::InvalidToken), "Invalid token");
        assert_eq!(format!("{}", AuthError::TokenExpired), "Token expired");
        assert_eq!(
            format!("{}", AuthError::PasswordTooShort),
            "Password too short (minimum 8 characters)"
        );
        assert_eq!(
            format!("{}", AuthError::PasswordTooWeak),
            "Password too weak"
        );
    }

    #[test]
    fn test_auth_error_from_user_repository_error() {
        let err: AuthError = UserRepositoryError::NotFound.into();
        assert!(matches!(err, AuthError::UserNotFound));

        let err: AuthError = UserRepositoryError::EmailAlreadyExists.into();
        assert!(matches!(err, AuthError::EmailAlreadyExists));

        let err: AuthError = UserRepositoryError::UsernameAlreadyExists.into();
        assert!(matches!(err, AuthError::UsernameAlreadyExists));
    }

    #[test]
    fn test_auth_error_from_session_repository_error() {
        let err: AuthError = SessionRepositoryError::NotFound.into();
        assert!(matches!(err, AuthError::SessionNotFound));

        let err: AuthError = SessionRepositoryError::Expired.into();
        assert!(matches!(err, AuthError::TokenExpired));
    }

    #[test]
    fn test_auth_error_from_jwt_error() {
        let err: AuthError = JwtError::Expired.into();
        assert!(matches!(err, AuthError::TokenExpired));

        let err: AuthError = JwtError::InvalidToken.into();
        assert!(matches!(err, AuthError::InvalidToken));

        let err: AuthError = JwtError::InvalidTokenType.into();
        assert!(matches!(err, AuthError::InvalidToken));
    }

    // ========================================================================
    // Request/Response Serialization Tests
    // ========================================================================

    #[test]
    fn test_register_request_deserialization() {
        let json = r#"{
            "email": "user@example.com",
            "password": "Password123",
            "username": "testuser"
        }"#;

        let request: RegisterRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.email, "user@example.com");
        assert_eq!(request.password, "Password123");
        assert_eq!(request.username, "testuser");
    }

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{
            "email": "user@example.com",
            "password": "Password123"
        }"#;

        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.email, "user@example.com");
        assert_eq!(request.password, "Password123");
    }

    #[test]
    fn test_refresh_request_deserialization() {
        let json = r#"{
            "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
        }"#;

        let request: RefreshRequest = serde_json::from_str(json).unwrap();
        assert!(request.refresh_token.starts_with("eyJ"));
    }
}
