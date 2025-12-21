//! Session repository for refresh token management
//!
//! Handles storage and validation of refresh tokens for JWT authentication.
//! Tokens are stored as SHA-256 hashes for security.

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::db::models::{CreateSession, Session};

/// Default session duration (7 days)
const DEFAULT_SESSION_DURATION_DAYS: i64 = 7;

/// Session repository error types
#[derive(Debug, thiserror::Error)]
pub enum SessionRepositoryError {
    #[error("Session not found")]
    NotFound,

    #[error("Session expired")]
    Expired,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Session repository for database operations
#[derive(Clone)]
pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    /// Create a new session repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Hash a token using SHA-256
    pub fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Create a new session with a hashed token
    /// Returns the session ID (the raw token should be sent to client)
    pub async fn create(
        &self,
        user_id: Uuid,
        raw_token: &str,
        duration_days: Option<i64>,
    ) -> Result<Session, SessionRepositoryError> {
        let token_hash = Self::hash_token(raw_token);
        let duration = duration_days.unwrap_or(DEFAULT_SESSION_DURATION_DAYS);
        let expires_at = Utc::now() + Duration::days(duration);

        let session = sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, token_hash, expires_at, created_at
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Create a session from a DTO (token_hash should already be hashed)
    pub async fn create_from_dto(
        &self,
        dto: &CreateSession,
    ) -> Result<Session, SessionRepositoryError> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, token_hash, expires_at, created_at
            "#,
        )
        .bind(dto.user_id)
        .bind(&dto.token_hash)
        .bind(dto.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Find a session by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Session>, SessionRepositoryError> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM sessions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Find a session by raw token (will be hashed for lookup)
    pub async fn find_by_token(
        &self,
        raw_token: &str,
    ) -> Result<Option<Session>, SessionRepositoryError> {
        let token_hash = Self::hash_token(raw_token);
        self.find_by_token_hash(&token_hash).await
    }

    /// Find a session by token hash
    pub async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Session>, SessionRepositoryError> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM sessions
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Validate a session token and return the session if valid
    /// Returns None if token not found, Err if expired
    pub async fn validate_token(
        &self,
        raw_token: &str,
    ) -> Result<Option<Session>, SessionRepositoryError> {
        let session = match self.find_by_token(raw_token).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        if session.expires_at < Utc::now() {
            // Clean up expired session
            self.delete(session.id).await?;
            return Err(SessionRepositoryError::Expired);
        }

        Ok(Some(session))
    }

    /// Find all sessions for a user
    pub async fn find_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Session>, SessionRepositoryError> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM sessions
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Delete a session by ID (logout from specific device)
    pub async fn delete(&self, id: Uuid) -> Result<bool, SessionRepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a session by raw token
    pub async fn delete_by_token(&self, raw_token: &str) -> Result<bool, SessionRepositoryError> {
        let token_hash = Self::hash_token(raw_token);
        self.delete_by_token_hash(&token_hash).await
    }

    /// Delete a session by token hash
    pub async fn delete_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<bool, SessionRepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all sessions for a user (logout from all devices)
    pub async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, SessionRepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Clean up expired sessions (should be run periodically)
    pub async fn cleanup_expired(&self) -> Result<u64, SessionRepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE expires_at < NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Extend session expiration
    pub async fn extend_session(
        &self,
        id: Uuid,
        additional_days: i64,
    ) -> Result<Session, SessionRepositoryError> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            UPDATE sessions
            SET expires_at = expires_at + $2 * INTERVAL '1 day'
            WHERE id = $1
            RETURNING id, user_id, token_hash, expires_at, created_at
            "#,
        )
        .bind(id)
        .bind(additional_days)
        .fetch_optional(&self.pool)
        .await?;

        session.ok_or(SessionRepositoryError::NotFound)
    }

    /// Count active sessions for a user
    pub async fn count_user_sessions(&self, user_id: Uuid) -> Result<i64, SessionRepositoryError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sessions
            WHERE user_id = $1 AND expires_at > NOW()
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Token Hashing Tests (don't require database)
    // ========================================================================

    #[test]
    fn test_hash_token_produces_consistent_hash() {
        let token = "my_refresh_token_12345";
        let hash1 = SessionRepository::hash_token(token);
        let hash2 = SessionRepository::hash_token(token);

        // Same token should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_produces_different_hashes_for_different_tokens() {
        let token1 = "token_one";
        let token2 = "token_two";

        let hash1 = SessionRepository::hash_token(token1);
        let hash2 = SessionRepository::hash_token(token2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_produces_64_char_hex_string() {
        let token = "any_token";
        let hash = SessionRepository::hash_token(token);

        // SHA-256 produces 32 bytes = 64 hex characters
        assert_eq!(hash.len(), 64);

        // Should be valid hex
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hash_token_empty_string() {
        let hash = SessionRepository::hash_token("");

        // Empty string still produces valid hash
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_token_unicode() {
        let token = "токен_令牌_🔑";
        let hash = SessionRepository::hash_token(token);

        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_token_long_token() {
        let token = "a".repeat(1000);
        let hash = SessionRepository::hash_token(&token);

        assert_eq!(hash.len(), 64);
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_session_repository_error_display() {
        let err = SessionRepositoryError::NotFound;
        assert_eq!(format!("{}", err), "Session not found");

        let err = SessionRepositoryError::Expired;
        assert_eq!(format!("{}", err), "Session expired");
    }

    #[test]
    fn test_session_repository_error_debug() {
        let err = SessionRepositoryError::NotFound;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));

        let err = SessionRepositoryError::Expired;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Expired"));
    }

    // ========================================================================
    // Integration Tests (require database)
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_session() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        let raw_token = "test_refresh_token_123";
        let session = repo.create(user_id, raw_token, None).await.unwrap();

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.token_hash, SessionRepository::hash_token(raw_token));
        assert!(session.expires_at > Utc::now());

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_token() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        let raw_token = "findable_token";
        let created = repo.create(user_id, raw_token, None).await.unwrap();

        let found = repo.find_by_token(raw_token).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_token_not_found() {
        let pool = create_test_pool().await;
        let repo = SessionRepository::new(pool);

        let found = repo.find_by_token("nonexistent_token").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_validate_token_valid() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        let raw_token = "valid_token";
        let created = repo.create(user_id, raw_token, Some(7)).await.unwrap();

        let result = repo.validate_token(raw_token).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, created.id);

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_session() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        let raw_token = "deletable_token";
        let session = repo.create(user_id, raw_token, None).await.unwrap();

        let deleted = repo.delete(session.id).await.unwrap();
        assert!(deleted);

        let found = repo.find_by_token(raw_token).await.unwrap();
        assert!(found.is_none());

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_by_token() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        let raw_token = "token_to_delete";
        repo.create(user_id, raw_token, None).await.unwrap();

        let deleted = repo.delete_by_token(raw_token).await.unwrap();
        assert!(deleted);

        let found = repo.find_by_token(raw_token).await.unwrap();
        assert!(found.is_none());

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_all_for_user() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        // Create multiple sessions
        repo.create(user_id, "token1", None).await.unwrap();
        repo.create(user_id, "token2", None).await.unwrap();
        repo.create(user_id, "token3", None).await.unwrap();

        let count = repo.count_user_sessions(user_id).await.unwrap();
        assert_eq!(count, 3);

        let deleted = repo.delete_all_for_user(user_id).await.unwrap();
        assert_eq!(deleted, 3);

        let count_after = repo.count_user_sessions(user_id).await.unwrap();
        assert_eq!(count_after, 0);

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_user_id() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        repo.create(user_id, "session1", None).await.unwrap();
        repo.create(user_id, "session2", None).await.unwrap();

        let sessions = repo.find_by_user_id(user_id).await.unwrap();
        assert_eq!(sessions.len(), 2);

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_extend_session() {
        let (pool, user_id) = setup_test_user().await;
        let repo = SessionRepository::new(pool.clone());

        let session = repo.create(user_id, "extendable", Some(1)).await.unwrap();
        let original_expires = session.expires_at;

        let extended = repo.extend_session(session.id, 7).await.unwrap();

        assert!(extended.expires_at > original_expires);

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    // Helper functions for integration tests
    async fn create_test_pool() -> PgPool {
        use crate::core::db::pool::{DbConfig, create_pool};

        let config = DbConfig::from_env().expect("DATABASE_URL must be set for tests");
        create_pool(&config)
            .await
            .expect("Failed to create test pool")
    }

    async fn setup_test_user() -> (PgPool, Uuid) {
        let pool = create_test_pool().await;

        // Create a test user for session tests
        let user_id = Uuid::new_v4();
        let unique_email = format!("session_test_{}@example.com", user_id);
        let unique_username = format!("session_test_{}", &user_id.to_string()[..8]);

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, username)
            VALUES ($1, $2, 'test_hash', $3)
            "#,
        )
        .bind(user_id)
        .bind(&unique_email)
        .bind(&unique_username)
        .execute(&pool)
        .await
        .expect("Failed to create test user");

        (pool, user_id)
    }

    async fn cleanup_test_user(pool: &PgPool, user_id: Uuid) {
        // Sessions will be deleted by CASCADE
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .expect("Failed to cleanup test user");
    }
}
