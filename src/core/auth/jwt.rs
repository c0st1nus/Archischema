//! JWT utilities for token generation and validation
//!
//! Provides JWT token creation and validation using HS256 algorithm.
//! Access tokens are short-lived (15 minutes), refresh tokens are long-lived (7 days).

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default access token expiration time (15 minutes)
const ACCESS_TOKEN_EXPIRATION_MINUTES: i64 = 15;

/// Default refresh token expiration time (7 days)
const REFRESH_TOKEN_EXPIRATION_DAYS: i64 = 7;

/// JWT configuration
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,
    /// Access token expiration in minutes
    pub access_token_expiration_minutes: i64,
    /// Refresh token expiration in days
    pub refresh_token_expiration_days: i64,
    /// Token issuer
    pub issuer: String,
}

impl JwtConfig {
    /// Create a new JWT configuration
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            access_token_expiration_minutes: ACCESS_TOKEN_EXPIRATION_MINUTES,
            refresh_token_expiration_days: REFRESH_TOKEN_EXPIRATION_DAYS,
            issuer: "archischema".to_string(),
        }
    }

    /// Create config from environment variables
    pub fn from_env() -> Result<Self, JwtError> {
        let secret = std::env::var("JWT_SECRET").map_err(|_| JwtError::MissingSecret)?;

        let access_exp = std::env::var("JWT_ACCESS_EXPIRATION_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(ACCESS_TOKEN_EXPIRATION_MINUTES);

        let refresh_exp = std::env::var("JWT_REFRESH_EXPIRATION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(REFRESH_TOKEN_EXPIRATION_DAYS);

        let issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "archischema".to_string());

        Ok(Self {
            secret,
            access_token_expiration_minutes: access_exp,
            refresh_token_expiration_days: refresh_exp,
            issuer,
        })
    }

    /// Set access token expiration
    pub fn access_token_expiration(mut self, minutes: i64) -> Self {
        self.access_token_expiration_minutes = minutes;
        self
    }

    /// Set refresh token expiration
    pub fn refresh_token_expiration(mut self, days: i64) -> Self {
        self.refresh_token_expiration_days = days;
        self
    }

    /// Set issuer
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = issuer.into();
        self
    }
}

/// JWT errors
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("JWT_SECRET environment variable not set")]
    MissingSecret,

    #[error("Token encoding failed: {0}")]
    EncodingError(String),

    #[error("Token decoding failed: {0}")]
    DecodingError(String),

    #[error("Token expired")]
    Expired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Invalid token type")]
    InvalidTokenType,
}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;

        match err.kind() {
            ErrorKind::ExpiredSignature => JwtError::Expired,
            ErrorKind::InvalidToken | ErrorKind::InvalidSignature | ErrorKind::InvalidAlgorithm => {
                JwtError::InvalidToken
            }
            _ => JwtError::DecodingError(err.to_string()),
        }
    }
}

/// Token type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::Access => write!(f, "access"),
            TokenType::Refresh => write!(f, "refresh"),
        }
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User email
    pub email: String,
    /// Username
    pub username: String,
    /// Token type (access or refresh)
    pub token_type: TokenType,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issuer
    pub iss: String,
    /// JWT ID (unique identifier for this token)
    pub jti: String,
}

impl Claims {
    /// Check if this is an access token
    pub fn is_access_token(&self) -> bool {
        self.token_type == TokenType::Access
    }

    /// Check if this is a refresh token
    pub fn is_refresh_token(&self) -> bool {
        self.token_type == TokenType::Refresh
    }

    /// Get user ID as UUID
    pub fn user_id(&self) -> Result<Uuid, JwtError> {
        Uuid::parse_str(&self.sub).map_err(|_| JwtError::InvalidToken)
    }
}

/// Token pair (access + refresh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token (short-lived)
    pub access_token: String,
    /// Refresh token (long-lived)
    pub refresh_token: String,
    /// Access token expiration (Unix timestamp)
    pub access_expires_at: i64,
    /// Refresh token expiration (Unix timestamp)
    pub refresh_expires_at: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
}

/// JWT service for token operations
#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Create a new JWT service
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Create JWT service from environment variables
    pub fn from_env() -> Result<Self, JwtError> {
        let config = JwtConfig::from_env()?;
        Ok(Self::new(config))
    }

    /// Generate an access token
    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        username: &str,
    ) -> Result<(String, i64), JwtError> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.config.access_token_expiration_minutes);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            username: username.to_string(),
            token_type: TokenType::Access,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.config.issuer.clone(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::EncodingError(e.to_string()))?;

        Ok((token, exp.timestamp()))
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(
        &self,
        user_id: Uuid,
        email: &str,
        username: &str,
    ) -> Result<(String, i64), JwtError> {
        let now = Utc::now();
        let exp = now + Duration::days(self.config.refresh_token_expiration_days);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            username: username.to_string(),
            token_type: TokenType::Refresh,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.config.issuer.clone(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::EncodingError(e.to_string()))?;

        Ok((token, exp.timestamp()))
    }

    /// Generate both access and refresh tokens
    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        email: &str,
        username: &str,
    ) -> Result<TokenPair, JwtError> {
        let (access_token, access_expires_at) =
            self.generate_access_token(user_id, email, username)?;
        let (refresh_token, refresh_expires_at) =
            self.generate_refresh_token(user_id, email, username)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_expires_at,
            refresh_expires_at,
            token_type: "Bearer".to_string(),
        })
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        // Set leeway to 0 for strict expiration checking
        validation.leeway = 0;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;

        Ok(token_data.claims)
    }

    /// Validate an access token specifically
    pub fn validate_access_token(&self, token: &str) -> Result<Claims, JwtError> {
        let claims = self.validate_token(token)?;

        if !claims.is_access_token() {
            return Err(JwtError::InvalidTokenType);
        }

        Ok(claims)
    }

    /// Validate a refresh token specifically
    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims, JwtError> {
        let claims = self.validate_token(token)?;

        if !claims.is_refresh_token() {
            return Err(JwtError::InvalidTokenType);
        }

        Ok(claims)
    }

    /// Get the refresh token expiration in days
    pub fn refresh_token_expiration_days(&self) -> i64 {
        self.config.refresh_token_expiration_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> JwtService {
        let config = JwtConfig::new("test_secret_key_for_testing_only_32bytes!");
        JwtService::new(config)
    }

    // ========================================================================
    // JwtConfig Tests
    // ========================================================================

    #[test]
    fn test_jwt_config_new() {
        let config = JwtConfig::new("my_secret");

        assert_eq!(config.secret, "my_secret");
        assert_eq!(
            config.access_token_expiration_minutes,
            ACCESS_TOKEN_EXPIRATION_MINUTES
        );
        assert_eq!(
            config.refresh_token_expiration_days,
            REFRESH_TOKEN_EXPIRATION_DAYS
        );
        assert_eq!(config.issuer, "archischema");
    }

    #[test]
    fn test_jwt_config_builder() {
        let config = JwtConfig::new("secret")
            .access_token_expiration(30)
            .refresh_token_expiration(14)
            .issuer("my_app");

        assert_eq!(config.access_token_expiration_minutes, 30);
        assert_eq!(config.refresh_token_expiration_days, 14);
        assert_eq!(config.issuer, "my_app");
    }

    #[test]
    fn test_jwt_config_from_env_missing_secret() {
        let original = std::env::var("JWT_SECRET").ok();
        // SAFETY: test environment
        unsafe { std::env::remove_var("JWT_SECRET") };

        let result = JwtConfig::from_env();
        assert!(matches!(result, Err(JwtError::MissingSecret)));

        if let Some(val) = original {
            // SAFETY: test environment
            unsafe { std::env::set_var("JWT_SECRET", val) };
        }
    }

    // ========================================================================
    // Token Type Tests
    // ========================================================================

    #[test]
    fn test_token_type_display() {
        assert_eq!(TokenType::Access.to_string(), "access");
        assert_eq!(TokenType::Refresh.to_string(), "refresh");
    }

    #[test]
    fn test_token_type_serialization() {
        let access_json = serde_json::to_string(&TokenType::Access).unwrap();
        let refresh_json = serde_json::to_string(&TokenType::Refresh).unwrap();

        assert_eq!(access_json, r#""access""#);
        assert_eq!(refresh_json, r#""refresh""#);
    }

    #[test]
    fn test_token_type_deserialization() {
        let access: TokenType = serde_json::from_str(r#""access""#).unwrap();
        let refresh: TokenType = serde_json::from_str(r#""refresh""#).unwrap();

        assert_eq!(access, TokenType::Access);
        assert_eq!(refresh, TokenType::Refresh);
    }

    // ========================================================================
    // JWT Service Tests
    // ========================================================================

    #[test]
    fn test_generate_access_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let result = service.generate_access_token(user_id, "test@example.com", "testuser");

        assert!(result.is_ok());
        let (token, exp) = result.unwrap();
        assert!(!token.is_empty());
        assert!(exp > Utc::now().timestamp());
    }

    #[test]
    fn test_generate_refresh_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let result = service.generate_refresh_token(user_id, "test@example.com", "testuser");

        assert!(result.is_ok());
        let (token, exp) = result.unwrap();
        assert!(!token.is_empty());
        assert!(exp > Utc::now().timestamp());
    }

    #[test]
    fn test_generate_token_pair() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let result = service.generate_token_pair(user_id, "test@example.com", "testuser");

        assert!(result.is_ok());
        let pair = result.unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_ne!(pair.access_token, pair.refresh_token);
        assert_eq!(pair.token_type, "Bearer");
        assert!(pair.refresh_expires_at > pair.access_expires_at);
    }

    #[test]
    fn test_validate_access_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (token, _) = service
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let claims = service.validate_access_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.username, "testuser");
        assert!(claims.is_access_token());
    }

    #[test]
    fn test_validate_refresh_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (token, _) = service
            .generate_refresh_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let claims = service.validate_refresh_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert!(claims.is_refresh_token());
    }

    #[test]
    fn test_validate_access_token_with_refresh_token_fails() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (refresh_token, _) = service
            .generate_refresh_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let result = service.validate_access_token(&refresh_token);
        assert!(matches!(result, Err(JwtError::InvalidTokenType)));
    }

    #[test]
    fn test_validate_refresh_token_with_access_token_fails() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (access_token, _) = service
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let result = service.validate_refresh_token(&access_token);
        assert!(matches!(result, Err(JwtError::InvalidTokenType)));
    }

    #[test]
    fn test_validate_invalid_token() {
        let service = create_test_service();

        let result = service.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let service1 = JwtService::new(JwtConfig::new("secret_one"));
        let service2 = JwtService::new(JwtConfig::new("secret_two"));

        let user_id = Uuid::new_v4();
        let (token, _) = service1
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let result = service2.validate_token(&token);
        assert!(matches!(result, Err(JwtError::InvalidToken)));
    }

    #[test]
    fn test_claims_user_id() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (token, _) = service
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let claims = service.validate_token(&token).unwrap();
        assert_eq!(claims.user_id().unwrap(), user_id);
    }

    #[test]
    fn test_token_contains_jti() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (token1, _) = service
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();
        let (token2, _) = service
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();

        let claims1 = service.validate_token(&token1).unwrap();
        let claims2 = service.validate_token(&token2).unwrap();

        // Each token should have a unique JTI
        assert_ne!(claims1.jti, claims2.jti);
    }

    #[test]
    fn test_expired_token() {
        // Create a service with negative expiration to ensure token is already expired
        let config = JwtConfig::new("test_secret").access_token_expiration(-1);
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let (token, _) = service
            .generate_access_token(user_id, "test@example.com", "testuser")
            .unwrap();

        // Token should be expired immediately since expiration is in the past
        let result = service.validate_token(&token);
        assert!(
            matches!(result, Err(JwtError::Expired)),
            "Expected Expired error, got: {:?}",
            result
        );
    }

    // ========================================================================
    // Error Tests
    // ========================================================================

    #[test]
    fn test_jwt_error_display() {
        assert_eq!(
            format!("{}", JwtError::MissingSecret),
            "JWT_SECRET environment variable not set"
        );
        assert_eq!(format!("{}", JwtError::Expired), "Token expired");
        assert_eq!(format!("{}", JwtError::InvalidToken), "Invalid token");
        assert_eq!(
            format!("{}", JwtError::InvalidTokenType),
            "Invalid token type"
        );
    }

    #[test]
    fn test_jwt_error_debug() {
        let err = JwtError::Expired;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Expired"));
    }

    // ========================================================================
    // TokenPair Tests
    // ========================================================================

    #[test]
    fn test_token_pair_serialization() {
        let pair = TokenPair {
            access_token: "access123".to_string(),
            refresh_token: "refresh456".to_string(),
            access_expires_at: 1234567890,
            refresh_expires_at: 1234567890 + 86400 * 7,
            token_type: "Bearer".to_string(),
        };

        let json = serde_json::to_string(&pair).unwrap();
        assert!(json.contains("access123"));
        assert!(json.contains("refresh456"));
        assert!(json.contains("Bearer"));
    }

    #[test]
    fn test_token_pair_deserialization() {
        let json = r#"{
            "access_token": "access123",
            "refresh_token": "refresh456",
            "access_expires_at": 1234567890,
            "refresh_expires_at": 1234567891,
            "token_type": "Bearer"
        }"#;

        let pair: TokenPair = serde_json::from_str(json).unwrap();
        assert_eq!(pair.access_token, "access123");
        assert_eq!(pair.refresh_token, "refresh456");
        assert_eq!(pair.token_type, "Bearer");
    }
}
