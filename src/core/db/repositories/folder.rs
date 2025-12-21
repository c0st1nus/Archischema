//! Folder repository for database operations
//!
//! Provides CRUD operations for folders including:
//! - Create, read, update, delete operations
//! - Tree structure support (parent/child relationships)
//! - Ownership verification

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::core::db::models::{CreateFolder, Folder, UpdateFolder};

/// Folder with depth information for tree queries
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FolderWithDepth {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub depth: i32,
}

/// Folder tree node with children count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderNode {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub children_count: i64,
    pub diagrams_count: i64,
}

/// Folder repository error types
#[derive(Debug, thiserror::Error)]
pub enum FolderRepositoryError {
    #[error("Folder not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("Parent folder not found")]
    ParentNotFound,

    #[error("Cannot move folder into itself or its descendants")]
    CircularReference,

    #[error("Folder name already exists in this location")]
    NameAlreadyExists,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Folder repository for database operations
#[derive(Clone)]
pub struct FolderRepository {
    pool: PgPool,
}

impl FolderRepository {
    /// Create a new folder repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new folder
    pub async fn create(&self, dto: &CreateFolder) -> Result<Folder, FolderRepositoryError> {
        // Verify parent folder exists and belongs to owner if specified
        if let Some(parent_id) = dto.parent_id {
            let parent_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND owner_id = $2)",
            )
            .bind(parent_id)
            .bind(dto.owner_id)
            .fetch_one(&self.pool)
            .await?;

            if !parent_exists {
                return Err(FolderRepositoryError::ParentNotFound);
            }
        }

        // Check for duplicate name in same location
        let name_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM folders
                WHERE owner_id = $1
                AND COALESCE(parent_id, '00000000-0000-0000-0000-000000000000') = COALESCE($2, '00000000-0000-0000-0000-000000000000')
                AND LOWER(name) = LOWER($3)
            )
            "#,
        )
        .bind(dto.owner_id)
        .bind(dto.parent_id)
        .bind(&dto.name)
        .fetch_one(&self.pool)
        .await?;

        if name_exists {
            return Err(FolderRepositoryError::NameAlreadyExists);
        }

        let folder = sqlx::query_as::<_, Folder>(
            r#"
            INSERT INTO folders (owner_id, parent_id, name)
            VALUES ($1, $2, $3)
            RETURNING id, owner_id, parent_id, name, created_at, updated_at
            "#,
        )
        .bind(dto.owner_id)
        .bind(dto.parent_id)
        .bind(&dto.name)
        .fetch_one(&self.pool)
        .await?;

        Ok(folder)
    }

    /// Find a folder by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Folder>, FolderRepositoryError> {
        let folder = sqlx::query_as::<_, Folder>(
            r#"
            SELECT id, owner_id, parent_id, name, created_at, updated_at
            FROM folders
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(folder)
    }

    /// Find a folder by ID with ownership check
    pub async fn find_by_id_and_owner(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Folder>, FolderRepositoryError> {
        let folder = sqlx::query_as::<_, Folder>(
            r#"
            SELECT id, owner_id, parent_id, name, created_at, updated_at
            FROM folders
            WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(folder)
    }

    /// List folders by owner at root level (no parent)
    pub async fn list_root_folders(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<Folder>, FolderRepositoryError> {
        let folders = sqlx::query_as::<_, Folder>(
            r#"
            SELECT id, owner_id, parent_id, name, created_at, updated_at
            FROM folders
            WHERE owner_id = $1 AND parent_id IS NULL
            ORDER BY name ASC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    /// List child folders of a parent folder
    pub async fn list_children(
        &self,
        parent_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Vec<Folder>, FolderRepositoryError> {
        let folders = sqlx::query_as::<_, Folder>(
            r#"
            SELECT id, owner_id, parent_id, name, created_at, updated_at
            FROM folders
            WHERE owner_id = $1 AND parent_id = $2
            ORDER BY name ASC
            "#,
        )
        .bind(owner_id)
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    /// List all folders for a user (flat list)
    pub async fn list_all_by_owner(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<Folder>, FolderRepositoryError> {
        let folders = sqlx::query_as::<_, Folder>(
            r#"
            SELECT id, owner_id, parent_id, name, created_at, updated_at
            FROM folders
            WHERE owner_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    /// Get folder tree with depth information using recursive CTE
    pub async fn get_folder_tree(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<FolderWithDepth>, FolderRepositoryError> {
        let folders = sqlx::query_as::<_, FolderWithDepth>(
            r#"
            WITH RECURSIVE folder_tree AS (
                -- Base case: root folders
                SELECT id, owner_id, parent_id, name, created_at, updated_at, 0 as depth
                FROM folders
                WHERE owner_id = $1 AND parent_id IS NULL

                UNION ALL

                -- Recursive case: child folders
                SELECT f.id, f.owner_id, f.parent_id, f.name, f.created_at, f.updated_at, ft.depth + 1
                FROM folders f
                INNER JOIN folder_tree ft ON f.parent_id = ft.id
                WHERE f.owner_id = $1
            )
            SELECT id, owner_id, parent_id, name, created_at, updated_at, depth
            FROM folder_tree
            ORDER BY depth, name
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    /// Get folder nodes with children and diagrams count
    pub async fn get_folder_nodes(
        &self,
        owner_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<FolderNode>, FolderRepositoryError> {
        let folders = if let Some(pid) = parent_id {
            sqlx::query_as::<
                _,
                (
                    Uuid,
                    Uuid,
                    Option<Uuid>,
                    String,
                    DateTime<Utc>,
                    DateTime<Utc>,
                    i64,
                    i64,
                ),
            >(
                r#"
                SELECT
                    f.id, f.owner_id, f.parent_id, f.name, f.created_at, f.updated_at,
                    (SELECT COUNT(*) FROM folders WHERE parent_id = f.id) as children_count,
                    (SELECT COUNT(*) FROM diagrams WHERE folder_id = f.id) as diagrams_count
                FROM folders f
                WHERE f.owner_id = $1 AND f.parent_id = $2
                ORDER BY f.name ASC
                "#,
            )
            .bind(owner_id)
            .bind(pid)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<
                _,
                (
                    Uuid,
                    Uuid,
                    Option<Uuid>,
                    String,
                    DateTime<Utc>,
                    DateTime<Utc>,
                    i64,
                    i64,
                ),
            >(
                r#"
                SELECT
                    f.id, f.owner_id, f.parent_id, f.name, f.created_at, f.updated_at,
                    (SELECT COUNT(*) FROM folders WHERE parent_id = f.id) as children_count,
                    (SELECT COUNT(*) FROM diagrams WHERE folder_id = f.id) as diagrams_count
                FROM folders f
                WHERE f.owner_id = $1 AND f.parent_id IS NULL
                ORDER BY f.name ASC
                "#,
            )
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await?
        };

        let nodes = folders
            .into_iter()
            .map(
                |(
                    id,
                    owner_id,
                    parent_id,
                    name,
                    created_at,
                    updated_at,
                    children_count,
                    diagrams_count,
                )| {
                    FolderNode {
                        id,
                        owner_id,
                        parent_id,
                        name,
                        created_at,
                        updated_at,
                        children_count,
                        diagrams_count,
                    }
                },
            )
            .collect();

        Ok(nodes)
    }

    /// Update a folder
    pub async fn update(
        &self,
        id: Uuid,
        owner_id: Uuid,
        updates: &UpdateFolder,
    ) -> Result<Folder, FolderRepositoryError> {
        // Verify folder exists and belongs to owner
        let existing = self.find_by_id_and_owner(id, owner_id).await?;
        if existing.is_none() {
            return Err(FolderRepositoryError::NotFound);
        }

        // If changing parent, verify no circular reference
        if let Some(new_parent_id) = updates.parent_id
            && let Some(new_pid) = new_parent_id
        {
            // Check if new parent exists
            let parent_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND owner_id = $2)",
            )
            .bind(new_pid)
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await?;

            if !parent_exists {
                return Err(FolderRepositoryError::ParentNotFound);
            }

            // Check for circular reference
            if self.would_create_cycle(id, new_pid).await? {
                return Err(FolderRepositoryError::CircularReference);
            }
        }

        // Check for duplicate name if name is changing
        if let Some(ref new_name) = updates.name {
            // Determine target parent: use new parent if specified, otherwise keep existing
            let target_parent = match updates.parent_id {
                Some(new_pid) => new_pid,
                None => existing.as_ref().unwrap().parent_id,
            };

            let name_exists = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM folders
                    WHERE owner_id = $1
                    AND COALESCE(parent_id, '00000000-0000-0000-0000-000000000000') = COALESCE($2, '00000000-0000-0000-0000-000000000000')
                    AND LOWER(name) = LOWER($3)
                    AND id != $4
                )
                "#,
            )
            .bind(owner_id)
            .bind(target_parent)
            .bind(new_name)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            if name_exists {
                return Err(FolderRepositoryError::NameAlreadyExists);
            }
        }

        // Build dynamic update query
        let existing = existing.unwrap();
        // Handle Option<Option<Uuid>>: None = keep existing, Some(None) = set to null, Some(Some(id)) = set to id
        let new_parent_id = match updates.parent_id {
            Some(new_pid) => new_pid,   // Use new value (could be None or Some(id))
            None => existing.parent_id, // Keep existing value
        };
        let new_name = updates.name.clone().unwrap_or(existing.name);

        let folder = sqlx::query_as::<_, Folder>(
            r#"
            UPDATE folders
            SET parent_id = $1, name = $2, updated_at = NOW()
            WHERE id = $3 AND owner_id = $4
            RETURNING id, owner_id, parent_id, name, created_at, updated_at
            "#,
        )
        .bind(new_parent_id)
        .bind(&new_name)
        .bind(id)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(folder)
    }

    /// Rename a folder
    pub async fn rename(
        &self,
        id: Uuid,
        owner_id: Uuid,
        new_name: &str,
    ) -> Result<Folder, FolderRepositoryError> {
        let updates = UpdateFolder {
            parent_id: None,
            name: Some(new_name.to_string()),
        };
        self.update(id, owner_id, &updates).await
    }

    /// Move a folder to a new parent
    pub async fn move_to_parent(
        &self,
        id: Uuid,
        owner_id: Uuid,
        new_parent_id: Option<Uuid>,
    ) -> Result<Folder, FolderRepositoryError> {
        let updates = UpdateFolder {
            parent_id: Some(new_parent_id),
            name: None,
        };
        self.update(id, owner_id, &updates).await
    }

    /// Delete a folder (will cascade to children and move diagrams to root)
    pub async fn delete(&self, id: Uuid, owner_id: Uuid) -> Result<bool, FolderRepositoryError> {
        let result = sqlx::query("DELETE FROM folders WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Count folders for a user
    pub async fn count_by_owner(&self, owner_id: Uuid) -> Result<i64, FolderRepositoryError> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM folders WHERE owner_id = $1")
                .bind(owner_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }

    /// Count children of a folder
    pub async fn count_children(&self, parent_id: Uuid) -> Result<i64, FolderRepositoryError> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM folders WHERE parent_id = $1")
                .bind(parent_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }

    /// Get folder path (breadcrumb) from root to the folder
    pub async fn get_path(&self, id: Uuid) -> Result<Vec<Folder>, FolderRepositoryError> {
        let folders = sqlx::query_as::<_, Folder>(
            r#"
            WITH RECURSIVE folder_path AS (
                -- Start from the target folder
                SELECT id, owner_id, parent_id, name, created_at, updated_at, 0 as depth
                FROM folders
                WHERE id = $1

                UNION ALL

                -- Walk up to parents
                SELECT f.id, f.owner_id, f.parent_id, f.name, f.created_at, f.updated_at, fp.depth + 1
                FROM folders f
                INNER JOIN folder_path fp ON f.id = fp.parent_id
            )
            SELECT id, owner_id, parent_id, name, created_at, updated_at
            FROM folder_path
            ORDER BY depth DESC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    /// Check if moving a folder would create a circular reference
    async fn would_create_cycle(
        &self,
        folder_id: Uuid,
        new_parent_id: Uuid,
    ) -> Result<bool, FolderRepositoryError> {
        // Can't move folder into itself
        if folder_id == new_parent_id {
            return Ok(true);
        }

        // Check if new_parent_id is a descendant of folder_id
        let is_descendant = sqlx::query_scalar::<_, bool>(
            r#"
            WITH RECURSIVE descendants AS (
                SELECT id FROM folders WHERE parent_id = $1
                UNION ALL
                SELECT f.id FROM folders f
                INNER JOIN descendants d ON f.parent_id = d.id
            )
            SELECT EXISTS(SELECT 1 FROM descendants WHERE id = $2)
            "#,
        )
        .bind(folder_id)
        .bind(new_parent_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(is_descendant)
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
    fn test_folder_repository_error_display() {
        assert_eq!(
            FolderRepositoryError::NotFound.to_string(),
            "Folder not found"
        );
        assert_eq!(
            FolderRepositoryError::AccessDenied.to_string(),
            "Access denied"
        );
        assert_eq!(
            FolderRepositoryError::ParentNotFound.to_string(),
            "Parent folder not found"
        );
        assert_eq!(
            FolderRepositoryError::CircularReference.to_string(),
            "Cannot move folder into itself or its descendants"
        );
        assert_eq!(
            FolderRepositoryError::NameAlreadyExists.to_string(),
            "Folder name already exists in this location"
        );
    }

    #[test]
    fn test_folder_repository_error_debug() {
        let err = FolderRepositoryError::NotFound;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn test_folder_with_depth_serialization() {
        let folder = FolderWithDepth {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            parent_id: None,
            name: "Test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            depth: 0,
        };

        let json = serde_json::to_string(&folder).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("depth"));
    }

    #[test]
    fn test_folder_node_serialization() {
        let node = FolderNode {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            parent_id: None,
            name: "Test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            children_count: 5,
            diagrams_count: 10,
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("children_count"));
        assert!(json.contains("diagrams_count"));
    }

    #[test]
    fn test_folder_with_depth_clone() {
        let folder = FolderWithDepth {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            parent_id: Some(Uuid::new_v4()),
            name: "Clone Test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            depth: 3,
        };

        let cloned = folder.clone();
        assert_eq!(folder.id, cloned.id);
        assert_eq!(folder.name, cloned.name);
        assert_eq!(folder.depth, cloned.depth);
    }

    #[test]
    fn test_folder_node_clone() {
        let node = FolderNode {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            parent_id: None,
            name: "Node Test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            children_count: 3,
            diagrams_count: 7,
        };

        let cloned = node.clone();
        assert_eq!(node.id, cloned.id);
        assert_eq!(node.children_count, cloned.children_count);
        assert_eq!(node.diagrams_count, cloned.diagrams_count);
    }

    // ========================================================================
    // Integration Tests (require database)
    // ========================================================================

    #[tokio::test]
    #[ignore] // Run with: cargo test --features ssr -- --ignored
    async fn test_create_folder() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        let dto = CreateFolder {
            owner_id: user_id,
            parent_id: None,
            name: "Test Folder".to_string(),
        };

        let folder = repo.create(&dto).await.unwrap();

        assert_eq!(folder.name, "Test Folder");
        assert_eq!(folder.owner_id, user_id);
        assert!(folder.parent_id.is_none());

        // Cleanup
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_nested_folder() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        // Create parent folder
        let parent = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Parent".to_string(),
            })
            .await
            .unwrap();

        // Create child folder
        let child = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: Some(parent.id),
                name: "Child".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(child.parent_id, Some(parent.id));

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_create_folder_duplicate_name() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        // Create first folder
        repo.create(&CreateFolder {
            owner_id: user_id,
            parent_id: None,
            name: "Duplicate".to_string(),
        })
        .await
        .unwrap();

        // Try to create folder with same name
        let result = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Duplicate".to_string(),
            })
            .await;

        assert!(matches!(
            result,
            Err(FolderRepositoryError::NameAlreadyExists)
        ));

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_by_id() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        let created = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Find Me".to_string(),
            })
            .await
            .unwrap();

        let found = repo.find_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Find Me");

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_root_folders() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        // Create root folders
        repo.create(&CreateFolder {
            owner_id: user_id,
            parent_id: None,
            name: "Root 1".to_string(),
        })
        .await
        .unwrap();

        repo.create(&CreateFolder {
            owner_id: user_id,
            parent_id: None,
            name: "Root 2".to_string(),
        })
        .await
        .unwrap();

        let roots = repo.list_root_folders(user_id).await.unwrap();
        assert_eq!(roots.len(), 2);

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_folder_tree() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        // Create folder structure
        let root = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Root".to_string(),
            })
            .await
            .unwrap();

        let child = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: Some(root.id),
                name: "Child".to_string(),
            })
            .await
            .unwrap();

        repo.create(&CreateFolder {
            owner_id: user_id,
            parent_id: Some(child.id),
            name: "Grandchild".to_string(),
        })
        .await
        .unwrap();

        let tree = repo.get_folder_tree(user_id).await.unwrap();
        assert_eq!(tree.len(), 3);
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[1].depth, 1);
        assert_eq!(tree[2].depth, 2);

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_rename_folder() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        let folder = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Original".to_string(),
            })
            .await
            .unwrap();

        let renamed = repo.rename(folder.id, user_id, "Renamed").await.unwrap();
        assert_eq!(renamed.name, "Renamed");

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_move_folder() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        // Create two root folders
        let folder1 = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Folder 1".to_string(),
            })
            .await
            .unwrap();

        let folder2 = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Folder 2".to_string(),
            })
            .await
            .unwrap();

        // Move folder2 into folder1
        let moved = repo
            .move_to_parent(folder2.id, user_id, Some(folder1.id))
            .await
            .unwrap();

        assert_eq!(moved.parent_id, Some(folder1.id));

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete_folder() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        let folder = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "To Delete".to_string(),
            })
            .await
            .unwrap();

        let deleted = repo.delete(folder.id, user_id).await.unwrap();
        assert!(deleted);

        let found = repo.find_by_id(folder.id).await.unwrap();
        assert!(found.is_none());

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_path() {
        let pool = create_test_pool().await;
        let user_id = setup_test_user(&pool).await;
        let repo = FolderRepository::new(pool.clone());

        // Create folder structure
        let root = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: None,
                name: "Root".to_string(),
            })
            .await
            .unwrap();

        let child = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: Some(root.id),
                name: "Child".to_string(),
            })
            .await
            .unwrap();

        let grandchild = repo
            .create(&CreateFolder {
                owner_id: user_id,
                parent_id: Some(child.id),
                name: "Grandchild".to_string(),
            })
            .await
            .unwrap();

        let path = repo.get_path(grandchild.id).await.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "Root");
        assert_eq!(path[1].name, "Child");
        assert_eq!(path[2].name, "Grandchild");

        cleanup_test_user(&pool, user_id).await;
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    async fn create_test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
        PgPool::connect(&database_url).await.unwrap()
    }

    async fn setup_test_user(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        let email = format!("folder_test_{}@test.com", user_id);

        sqlx::query(
            "INSERT INTO users (id, email, password_hash, username) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(&email)
        .bind("$2b$12$test_hash_not_real")
        .bind(format!("folder_test_{}", user_id))
        .execute(pool)
        .await
        .unwrap();

        user_id
    }

    async fn cleanup_test_user(pool: &PgPool, user_id: Uuid) {
        // Folders will be cascade deleted with user
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }
}
