//! DiagramShare repository for database operations
//!
//! Provides CRUD operations for diagram sharing including:
//! - Create, read, update, delete share operations
//! - Permission management (view, edit)
//! - User lookup by email or username for sharing

use sqlx::PgPool;
use uuid::Uuid;

use crate::core::db::models::{
    CreateDiagramShare, DiagramShare, DiagramShareWithUser, SharePermission,
};

/// DiagramShare repository error types
#[derive(Debug, thiserror::Error)]
pub enum ShareRepositoryError {
    #[error("Share not found")]
    NotFound,

    #[error("Diagram not found")]
    DiagramNotFound,

    #[error("User not found")]
    UserNotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("Cannot share with yourself")]
    CannotShareWithSelf,

    #[error("Share already exists")]
    AlreadyExists,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// DiagramShare repository for database operations
#[derive(Clone)]
pub struct ShareRepository {
    pool: PgPool,
}

impl ShareRepository {
    /// Create a new share repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new diagram share
    pub async fn create(
        &self,
        dto: &CreateDiagramShare,
    ) -> Result<DiagramShare, ShareRepositoryError> {
        // Check if share already exists
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM diagram_shares WHERE diagram_id = $1 AND user_id = $2)",
        )
        .bind(dto.diagram_id)
        .bind(dto.user_id)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            return Err(ShareRepositoryError::AlreadyExists);
        }

        let share = sqlx::query_as::<_, DiagramShare>(
            r#"
            INSERT INTO diagram_shares (diagram_id, user_id, permission)
            VALUES ($1, $2, $3)
            RETURNING id, diagram_id, user_id, permission, created_at
            "#,
        )
        .bind(dto.diagram_id)
        .bind(dto.user_id)
        .bind(dto.permission.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(share)
    }

    /// Share a diagram with a user by email
    pub async fn share_with_email(
        &self,
        diagram_id: Uuid,
        owner_id: Uuid,
        email: &str,
        permission: SharePermission,
    ) -> Result<DiagramShareWithUser, ShareRepositoryError> {
        // Verify diagram exists and belongs to owner
        let diagram_owner =
            sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM diagrams WHERE id = $1")
                .bind(diagram_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(ShareRepositoryError::DiagramNotFound)?;

        if diagram_owner != owner_id {
            return Err(ShareRepositoryError::AccessDenied);
        }

        // Find user by email
        let user =
            sqlx::query_as::<_, (Uuid, String)>("SELECT id, username FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(ShareRepositoryError::UserNotFound)?;

        // Cannot share with yourself
        if user.0 == owner_id {
            return Err(ShareRepositoryError::CannotShareWithSelf);
        }

        // Create the share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: user.0,
            permission,
        };

        let share = self.create(&dto).await?;

        Ok(DiagramShareWithUser {
            id: share.id,
            diagram_id: share.diagram_id,
            user_id: share.user_id,
            username: user.1,
            email: email.to_string(),
            permission: share.permission,
            created_at: share.created_at,
        })
    }

    /// Share a diagram with a user by username
    pub async fn share_with_username(
        &self,
        diagram_id: Uuid,
        owner_id: Uuid,
        username: &str,
        permission: SharePermission,
    ) -> Result<DiagramShareWithUser, ShareRepositoryError> {
        // Verify diagram exists and belongs to owner
        let diagram_owner =
            sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM diagrams WHERE id = $1")
                .bind(diagram_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(ShareRepositoryError::DiagramNotFound)?;

        if diagram_owner != owner_id {
            return Err(ShareRepositoryError::AccessDenied);
        }

        // Find user by username
        let user =
            sqlx::query_as::<_, (Uuid, String)>("SELECT id, email FROM users WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(ShareRepositoryError::UserNotFound)?;

        // Cannot share with yourself
        if user.0 == owner_id {
            return Err(ShareRepositoryError::CannotShareWithSelf);
        }

        // Create the share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: user.0,
            permission,
        };

        let share = self.create(&dto).await?;

        Ok(DiagramShareWithUser {
            id: share.id,
            diagram_id: share.diagram_id,
            user_id: share.user_id,
            username: username.to_string(),
            email: user.1,
            permission: share.permission,
            created_at: share.created_at,
        })
    }

    /// Find a share by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<DiagramShare>, ShareRepositoryError> {
        let share = sqlx::query_as::<_, DiagramShare>(
            r#"
            SELECT id, diagram_id, user_id, permission, created_at
            FROM diagram_shares
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(share)
    }

    /// Find a share by diagram and user
    pub async fn find_by_diagram_and_user(
        &self,
        diagram_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<DiagramShare>, ShareRepositoryError> {
        let share = sqlx::query_as::<_, DiagramShare>(
            r#"
            SELECT id, diagram_id, user_id, permission, created_at
            FROM diagram_shares
            WHERE diagram_id = $1 AND user_id = $2
            "#,
        )
        .bind(diagram_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(share)
    }

    /// List all shares for a diagram (with user info)
    pub async fn list_by_diagram(
        &self,
        diagram_id: Uuid,
    ) -> Result<Vec<DiagramShareWithUser>, ShareRepositoryError> {
        let shares = sqlx::query_as::<_, DiagramShareWithUser>(
            r#"
            SELECT
                ds.id, ds.diagram_id, ds.user_id,
                u.username, u.email,
                ds.permission, ds.created_at
            FROM diagram_shares ds
            INNER JOIN users u ON ds.user_id = u.id
            WHERE ds.diagram_id = $1
            ORDER BY ds.created_at ASC
            "#,
        )
        .bind(diagram_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(shares)
    }

    /// List all shares for a user (diagrams shared with them)
    pub async fn list_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<DiagramShare>, ShareRepositoryError> {
        let shares = sqlx::query_as::<_, DiagramShare>(
            r#"
            SELECT id, diagram_id, user_id, permission, created_at
            FROM diagram_shares
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(shares)
    }

    /// Update share permission
    pub async fn update_permission(
        &self,
        diagram_id: Uuid,
        user_id: Uuid,
        permission: SharePermission,
    ) -> Result<DiagramShare, ShareRepositoryError> {
        let share = sqlx::query_as::<_, DiagramShare>(
            r#"
            UPDATE diagram_shares
            SET permission = $3
            WHERE diagram_id = $1 AND user_id = $2
            RETURNING id, diagram_id, user_id, permission, created_at
            "#,
        )
        .bind(diagram_id)
        .bind(user_id)
        .bind(permission.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ShareRepositoryError::NotFound)?;

        Ok(share)
    }

    /// Update share permission by share ID (with owner verification)
    pub async fn update_permission_by_id(
        &self,
        share_id: Uuid,
        owner_id: Uuid,
        permission: SharePermission,
    ) -> Result<DiagramShare, ShareRepositoryError> {
        // First verify the share exists and diagram belongs to owner
        let share = self
            .find_by_id(share_id)
            .await?
            .ok_or(ShareRepositoryError::NotFound)?;

        // Verify diagram ownership
        let diagram_owner =
            sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM diagrams WHERE id = $1")
                .bind(share.diagram_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(ShareRepositoryError::DiagramNotFound)?;

        if diagram_owner != owner_id {
            return Err(ShareRepositoryError::AccessDenied);
        }

        self.update_permission(share.diagram_id, share.user_id, permission)
            .await
    }

    /// Delete a share by diagram and user
    pub async fn delete(
        &self,
        diagram_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, ShareRepositoryError> {
        let result =
            sqlx::query("DELETE FROM diagram_shares WHERE diagram_id = $1 AND user_id = $2")
                .bind(diagram_id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a share (with owner verification)
    pub async fn delete_by_owner(
        &self,
        diagram_id: Uuid,
        share_user_id: Uuid,
        owner_id: Uuid,
    ) -> Result<bool, ShareRepositoryError> {
        // Verify diagram ownership
        let diagram_owner =
            sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM diagrams WHERE id = $1")
                .bind(diagram_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(ShareRepositoryError::DiagramNotFound)?;

        if diagram_owner != owner_id {
            return Err(ShareRepositoryError::AccessDenied);
        }

        self.delete(diagram_id, share_user_id).await
    }

    /// Delete all shares for a diagram
    pub async fn delete_all_for_diagram(
        &self,
        diagram_id: Uuid,
    ) -> Result<u64, ShareRepositoryError> {
        let result = sqlx::query("DELETE FROM diagram_shares WHERE diagram_id = $1")
            .bind(diagram_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Count shares for a diagram
    pub async fn count_by_diagram(&self, diagram_id: Uuid) -> Result<i64, ShareRepositoryError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM diagram_shares WHERE diagram_id = $1",
        )
        .bind(diagram_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Check if user has access to diagram via share
    pub async fn has_access(
        &self,
        diagram_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<SharePermission>, ShareRepositoryError> {
        let permission = sqlx::query_scalar::<_, String>(
            "SELECT permission FROM diagram_shares WHERE diagram_id = $1 AND user_id = $2",
        )
        .bind(diagram_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(permission.and_then(|p| p.parse().ok()))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Unit Tests (no database)
    // ========================================================================

    #[test]
    fn test_share_repository_error_display() {
        assert_eq!(
            ShareRepositoryError::NotFound.to_string(),
            "Share not found"
        );
        assert_eq!(
            ShareRepositoryError::DiagramNotFound.to_string(),
            "Diagram not found"
        );
        assert_eq!(
            ShareRepositoryError::UserNotFound.to_string(),
            "User not found"
        );
        assert_eq!(
            ShareRepositoryError::AccessDenied.to_string(),
            "Access denied"
        );
        assert_eq!(
            ShareRepositoryError::CannotShareWithSelf.to_string(),
            "Cannot share with yourself"
        );
        assert_eq!(
            ShareRepositoryError::AlreadyExists.to_string(),
            "Share already exists"
        );
    }

    #[test]
    fn test_share_repository_error_debug() {
        let err = ShareRepositoryError::NotFound;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }

    // ========================================================================
    // Integration Tests (require database)
    // ========================================================================

    #[tokio::test]
    #[ignore] // Run with: cargo test --features ssr -- --ignored
    async fn test_create_share() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::View,
        };

        let share = repo.create(&dto).await.unwrap();

        assert_eq!(share.diagram_id, diagram_id);
        assert_eq!(share.user_id, target_user_id);
        assert_eq!(share.permission, "view");

        // Cleanup
        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_share_duplicate() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::View,
        };

        // First share should succeed
        repo.create(&dto).await.unwrap();

        // Second share should fail
        let result = repo.create(&dto).await;
        assert!(matches!(result, Err(ShareRepositoryError::AlreadyExists)));

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_share_with_email() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        let email = format!("share_target_{}@test.com", target_user_id);
        let share = repo
            .share_with_email(diagram_id, owner_id, &email, SharePermission::Edit)
            .await
            .unwrap();

        assert_eq!(share.user_id, target_user_id);
        assert_eq!(share.permission, "edit");

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_share_with_self() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        let email = format!("share_owner_{}@test.com", owner_id);
        let result = repo
            .share_with_email(diagram_id, owner_id, &email, SharePermission::View)
            .await;

        assert!(matches!(
            result,
            Err(ShareRepositoryError::CannotShareWithSelf)
        ));

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_by_diagram() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        // Create a share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::View,
        };
        repo.create(&dto).await.unwrap();

        let shares = repo.list_by_diagram(diagram_id).await.unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].user_id, target_user_id);

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_update_permission() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        // Create a share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::View,
        };
        repo.create(&dto).await.unwrap();

        // Update permission
        let updated = repo
            .update_permission(diagram_id, target_user_id, SharePermission::Edit)
            .await
            .unwrap();

        assert_eq!(updated.permission, "edit");

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete_share() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        // Create a share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::View,
        };
        repo.create(&dto).await.unwrap();

        // Delete share
        let deleted = repo.delete(diagram_id, target_user_id).await.unwrap();
        assert!(deleted);

        // Verify deleted
        let share = repo
            .find_by_diagram_and_user(diagram_id, target_user_id)
            .await
            .unwrap();
        assert!(share.is_none());

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_has_access() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        // No access initially
        let access = repo.has_access(diagram_id, target_user_id).await.unwrap();
        assert!(access.is_none());

        // Create a share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::Edit,
        };
        repo.create(&dto).await.unwrap();

        // Now has access
        let access = repo.has_access(diagram_id, target_user_id).await.unwrap();
        assert_eq!(access, Some(SharePermission::Edit));

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_count_by_diagram() {
        let pool = create_test_pool().await;
        let (owner_id, target_user_id, diagram_id) = setup_test_data(&pool).await;
        let repo = ShareRepository::new(pool.clone());

        // Create a share
        let dto = CreateDiagramShare {
            diagram_id,
            user_id: target_user_id,
            permission: SharePermission::View,
        };
        repo.create(&dto).await.unwrap();

        let count = repo.count_by_diagram(diagram_id).await.unwrap();
        assert_eq!(count, 1);

        cleanup_test_data(&pool, owner_id, target_user_id).await;
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    async fn create_test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
        PgPool::connect(&database_url).await.unwrap()
    }

    async fn setup_test_data(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
        let owner_id = Uuid::new_v4();
        let target_user_id = Uuid::new_v4();
        let diagram_id = Uuid::new_v4();

        // Create owner user
        let owner_email = format!("share_owner_{}@test.com", owner_id);
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, username) VALUES ($1, $2, $3, $4)",
        )
        .bind(owner_id)
        .bind(&owner_email)
        .bind("$2b$12$test_hash_not_real")
        .bind(format!("share_owner_{}", owner_id))
        .execute(pool)
        .await
        .unwrap();

        // Create target user
        let target_email = format!("share_target_{}@test.com", target_user_id);
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, username) VALUES ($1, $2, $3, $4)",
        )
        .bind(target_user_id)
        .bind(&target_email)
        .bind("$2b$12$test_hash_not_real")
        .bind(format!("share_target_{}", target_user_id))
        .execute(pool)
        .await
        .unwrap();

        // Create diagram
        sqlx::query(
            r#"
            INSERT INTO diagrams (id, owner_id, name, schema_data)
            VALUES ($1, $2, 'Test Diagram', '{}')
            "#,
        )
        .bind(diagram_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .unwrap();

        (owner_id, target_user_id, diagram_id)
    }

    async fn cleanup_test_data(pool: &PgPool, owner_id: Uuid, target_user_id: Uuid) {
        // Delete users (cascade deletes diagrams and shares)
        sqlx::query("DELETE FROM users WHERE id = $1 OR id = $2")
            .bind(owner_id)
            .bind(target_user_id)
            .execute(pool)
            .await
            .unwrap();
    }
}
