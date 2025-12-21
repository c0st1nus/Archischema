//! User repository for database operations
//!
//! Provides CRUD operations for users with secure password hashing using bcrypt.

use sqlx::PgPool;
use uuid::Uuid;

use crate::core::db::DbError;
use crate::core::db::models::{CreateUser, UpdateUser, User};

/// Cost factor for bcrypt hashing (12 is recommended for production)
const BCRYPT_COST: u32 = 12;

/// User repository error types
#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    #[error("User not found")]
    NotFound,

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Username already exists")]
    UsernameAlreadyExists,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Password hashing failed: {0}")]
    HashingError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl From<DbError> for UserRepositoryError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::ConnectionError(e) => UserRepositoryError::DatabaseError(e),
            _ => UserRepositoryError::DatabaseError(sqlx::Error::Protocol(err.to_string())),
        }
    }
}

/// User repository for database operations
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Create a new user repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Hash a password using bcrypt with automatic salt generation
    pub fn hash_password(password: &str) -> Result<String, UserRepositoryError> {
        bcrypt::hash(password, BCRYPT_COST)
            .map_err(|e| UserRepositoryError::HashingError(e.to_string()))
    }

    /// Verify a password against a bcrypt hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, UserRepositoryError> {
        bcrypt::verify(password, hash).map_err(|e| UserRepositoryError::HashingError(e.to_string()))
    }

    /// Create a new user with a plain text password (will be hashed)
    pub async fn create(
        &self,
        email: &str,
        password: &str,
        username: &str,
        avatar_url: Option<&str>,
    ) -> Result<User, UserRepositoryError> {
        // Check if email already exists
        if self.find_by_email(email).await?.is_some() {
            return Err(UserRepositoryError::EmailAlreadyExists);
        }

        // Check if username already exists
        if self.find_by_username(username).await?.is_some() {
            return Err(UserRepositoryError::UsernameAlreadyExists);
        }

        // Hash the password with bcrypt (includes automatic salt)
        let password_hash = Self::hash_password(password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, username, avatar_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, password_hash, username, avatar_url, created_at, updated_at
            "#,
        )
        .bind(email)
        .bind(&password_hash)
        .bind(username)
        .bind(avatar_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Create a user from a CreateUser struct (password_hash should already be hashed)
    pub async fn create_from_dto(&self, dto: &CreateUser) -> Result<User, UserRepositoryError> {
        // Check if email already exists
        if self.find_by_email(&dto.email).await?.is_some() {
            return Err(UserRepositoryError::EmailAlreadyExists);
        }

        // Check if username already exists
        if self.find_by_username(&dto.username).await?.is_some() {
            return Err(UserRepositoryError::UsernameAlreadyExists);
        }

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, username, avatar_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, password_hash, username, avatar_url, created_at, updated_at
            "#,
        )
        .bind(&dto.email)
        .bind(&dto.password_hash)
        .bind(&dto.username)
        .bind(&dto.avatar_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find a user by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, username, avatar_url, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find a user by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepositoryError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, username, avatar_url, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find a user by username
    pub async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, username, avatar_url, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Update a user
    pub async fn update(
        &self,
        id: Uuid,
        updates: &UpdateUser,
    ) -> Result<User, UserRepositoryError> {
        // First check if user exists
        if self.find_by_id(id).await?.is_none() {
            return Err(UserRepositoryError::NotFound);
        }

        // Check email uniqueness if being updated
        if let Some(ref email) = updates.email
            && let Some(existing) = self.find_by_email(email).await?
            && existing.id != id
        {
            return Err(UserRepositoryError::EmailAlreadyExists);
        }

        // Check username uniqueness if being updated
        if let Some(ref username) = updates.username
            && let Some(existing) = self.find_by_username(username).await?
            && existing.id != id
        {
            return Err(UserRepositoryError::UsernameAlreadyExists);
        }

        // Hash new password if provided
        let password_hash = match &updates.password_hash {
            Some(password) => Some(Self::hash_password(password)?),
            None => None,
        };

        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET
                email = COALESCE($2, email),
                username = COALESCE($3, username),
                avatar_url = COALESCE($4, avatar_url),
                password_hash = COALESCE($5, password_hash)
            WHERE id = $1
            RETURNING id, email, password_hash, username, avatar_url, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&updates.email)
        .bind(&updates.username)
        .bind(&updates.avatar_url)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Update user's password (takes plain text, hashes it)
    pub async fn update_password(
        &self,
        id: Uuid,
        new_password: &str,
    ) -> Result<(), UserRepositoryError> {
        let password_hash = Self::hash_password(new_password)?;

        let result = sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(&password_hash)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(UserRepositoryError::NotFound);
        }

        Ok(())
    }

    /// Delete a user by ID
    pub async fn delete(&self, id: Uuid) -> Result<bool, UserRepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Authenticate a user by email and password
    /// Returns the user if credentials are valid, None otherwise
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        let user = match self.find_by_email(email).await? {
            Some(u) => u,
            None => return Ok(None),
        };

        let is_valid = Self::verify_password(password, &user.password_hash)?;

        if is_valid { Ok(Some(user)) } else { Ok(None) }
    }

    /// Count total users
    pub async fn count(&self) -> Result<i64, UserRepositoryError> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// List users with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, UserRepositoryError> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, username, avatar_url, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Password Hashing Tests (don't require database)
    // ========================================================================

    #[test]
    fn test_hash_password_produces_valid_bcrypt_hash() {
        let password = "my_secure_password123!";
        let hash = UserRepository::hash_password(password).unwrap();

        // Bcrypt hashes start with $2b$ (or $2a$, $2y$)
        assert!(hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$"));

        // Bcrypt hash should be 60 characters
        assert_eq!(hash.len(), 60);
    }

    #[test]
    fn test_hash_password_produces_different_hashes_for_same_password() {
        let password = "same_password";
        let hash1 = UserRepository::hash_password(password).unwrap();
        let hash2 = UserRepository::hash_password(password).unwrap();

        // Due to random salt, hashes should be different
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "correct_password";
        let hash = UserRepository::hash_password(password).unwrap();

        let is_valid = UserRepository::verify_password(password, &hash).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hash = UserRepository::hash_password(password).unwrap();

        let is_valid = UserRepository::verify_password(wrong_password, &hash).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_verify_password_empty_password() {
        let password = "";
        let hash = UserRepository::hash_password(password).unwrap();

        let is_valid = UserRepository::verify_password(password, &hash).unwrap();
        assert!(is_valid);

        let is_invalid = UserRepository::verify_password("not_empty", &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_verify_password_unicode() {
        let password = "пароль_密码_🔐";
        let hash = UserRepository::hash_password(password).unwrap();

        let is_valid = UserRepository::verify_password(password, &hash).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_verify_password_long_password() {
        // Bcrypt has a max input length of 72 bytes
        let password = "a".repeat(72);
        let hash = UserRepository::hash_password(&password).unwrap();

        let is_valid = UserRepository::verify_password(&password, &hash).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_verify_password_invalid_hash_format() {
        let result = UserRepository::verify_password("password", "not_a_valid_hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_password_special_characters() {
        let password = r#"p@$$w0rd!@#$%^&*()_+-=[]{}|;':",.<>?/`~"#;
        let hash = UserRepository::hash_password(password).unwrap();

        let is_valid = UserRepository::verify_password(password, &hash).unwrap();
        assert!(is_valid);
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_user_repository_error_display() {
        let err = UserRepositoryError::NotFound;
        assert_eq!(format!("{}", err), "User not found");

        let err = UserRepositoryError::EmailAlreadyExists;
        assert_eq!(format!("{}", err), "Email already exists");

        let err = UserRepositoryError::UsernameAlreadyExists;
        assert_eq!(format!("{}", err), "Username already exists");

        let err = UserRepositoryError::InvalidPassword;
        assert_eq!(format!("{}", err), "Invalid password");

        let err = UserRepositoryError::HashingError("test error".to_string());
        assert!(format!("{}", err).contains("test error"));
    }

    #[test]
    fn test_user_repository_error_debug() {
        let err = UserRepositoryError::NotFound;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    // ========================================================================
    // Integration Tests (require database)
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_user() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let user = repo
            .create(
                "test_create@example.com",
                "secure_password123",
                "test_create_user",
                None,
            )
            .await
            .unwrap();

        assert_eq!(user.email, "test_create@example.com");
        assert_eq!(user.username, "test_create_user");
        assert!(user.avatar_url.is_none());
        // Password should be hashed, not plain text
        assert_ne!(user.password_hash, "secure_password123");
        assert!(user.password_hash.starts_with("$2"));

        // Cleanup
        repo.delete(user.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_user_duplicate_email() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let user = repo
            .create("duplicate@example.com", "password", "unique_user1", None)
            .await
            .unwrap();

        let result = repo
            .create("duplicate@example.com", "password", "unique_user2", None)
            .await;

        assert!(matches!(
            result,
            Err(UserRepositoryError::EmailAlreadyExists)
        ));

        // Cleanup
        repo.delete(user.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_user_duplicate_username() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let user = repo
            .create(
                "unique1@example.com",
                "password",
                "duplicate_username",
                None,
            )
            .await
            .unwrap();

        let result = repo
            .create(
                "unique2@example.com",
                "password",
                "duplicate_username",
                None,
            )
            .await;

        assert!(matches!(
            result,
            Err(UserRepositoryError::UsernameAlreadyExists)
        ));

        // Cleanup
        repo.delete(user.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_id() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create("find_id@example.com", "password", "find_by_id_user", None)
            .await
            .unwrap();

        let found = repo.find_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);

        // Cleanup
        repo.delete(created.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_id_not_found() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let found = repo.find_by_id(Uuid::new_v4()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_email() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create(
                "find_email@example.com",
                "password",
                "find_email_user",
                None,
            )
            .await
            .unwrap();

        let found = repo.find_by_email("find_email@example.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "find_email@example.com");

        // Cleanup
        repo.delete(created.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_authenticate_success() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create("auth@example.com", "correct_password", "auth_user", None)
            .await
            .unwrap();

        let result = repo
            .authenticate("auth@example.com", "correct_password")
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, created.id);

        // Cleanup
        repo.delete(created.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_authenticate_wrong_password() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create(
                "auth_fail@example.com",
                "correct_password",
                "auth_fail_user",
                None,
            )
            .await
            .unwrap();

        let result = repo
            .authenticate("auth_fail@example.com", "wrong_password")
            .await
            .unwrap();

        assert!(result.is_none());

        // Cleanup
        repo.delete(created.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_authenticate_nonexistent_user() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let result = repo
            .authenticate("nonexistent@example.com", "password")
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_update_user() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create("update@example.com", "password", "update_user", None)
            .await
            .unwrap();

        let updates = UpdateUser {
            email: Some("updated@example.com".to_string()),
            username: Some("updated_username".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            password_hash: None,
        };

        let updated = repo.update(created.id, &updates).await.unwrap();

        assert_eq!(updated.email, "updated@example.com");
        assert_eq!(updated.username, "updated_username");
        assert_eq!(
            updated.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );

        // Cleanup
        repo.delete(created.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_update_password() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create(
                "update_pass@example.com",
                "old_password",
                "update_pass_user",
                None,
            )
            .await
            .unwrap();

        repo.update_password(created.id, "new_password")
            .await
            .unwrap();

        // Old password should fail
        let result = repo
            .authenticate("update_pass@example.com", "old_password")
            .await
            .unwrap();
        assert!(result.is_none());

        // New password should work
        let result = repo
            .authenticate("update_pass@example.com", "new_password")
            .await
            .unwrap();
        assert!(result.is_some());

        // Cleanup
        repo.delete(created.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_user() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let created = repo
            .create("delete@example.com", "password", "delete_user", None)
            .await
            .unwrap();

        let deleted = repo.delete(created.id).await.unwrap();
        assert!(deleted);

        let found = repo.find_by_id(created.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_nonexistent_user() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        let deleted = repo.delete(Uuid::new_v4()).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_count_and_list() {
        let pool = create_test_pool().await;
        let repo = UserRepository::new(pool);

        // Use unique identifiers to avoid conflicts with existing data
        let unique_id = Uuid::new_v4().to_string();
        let email1 = format!("list1_{}@example.com", &unique_id[..8]);
        let email2 = format!("list2_{}@example.com", &unique_id[..8]);
        let username1 = format!("list_user1_{}", &unique_id[..8]);
        let username2 = format!("list_user2_{}", &unique_id[..8]);

        let user1 = repo
            .create(&email1, "Password123", &username1, None)
            .await
            .unwrap();
        let user2 = repo
            .create(&email2, "Password123", &username2, None)
            .await
            .unwrap();

        // Just verify count returns a positive number
        let count = repo.count().await.unwrap();
        assert!(count >= 2, "count should be at least 2");

        // Verify list returns results and contains our users
        let users = repo.list(100, 0).await.unwrap();
        assert!(!users.is_empty(), "users list should not be empty");

        let has_user1 = users.iter().any(|u| u.id == user1.id);
        let has_user2 = users.iter().any(|u| u.id == user2.id);
        assert!(has_user1, "user1 should be in the list");
        assert!(has_user2, "user2 should be in the list");

        // Cleanup
        repo.delete(user1.id).await.unwrap();
        repo.delete(user2.id).await.unwrap();
    }

    // Helper function to create test pool
    async fn create_test_pool() -> PgPool {
        use crate::core::db::pool::{DbConfig, create_pool};

        let config = DbConfig::from_env().expect("DATABASE_URL must be set for tests");
        create_pool(&config)
            .await
            .expect("Failed to create test pool")
    }
}
