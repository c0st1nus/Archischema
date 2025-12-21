//! Diagram repository for database operations
//!
//! Provides CRUD operations for diagrams including:
//! - Create, read, update, delete operations
//! - Permission checking (owner, shared, public)
//! - Folder organization
//! - Autosave support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::core::db::models::{
    CreateDiagram, Diagram, DiagramSummary, SharePermission, UpdateDiagram,
};

/// Shared diagram info with permission
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SharedDiagramInfo {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub permission: String,
}

/// Diagram repository error types
#[derive(Debug, thiserror::Error)]
pub enum DiagramRepositoryError {
    #[error("Diagram not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("Folder not found")]
    FolderNotFound,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Access level for a diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramAccess {
    /// User owns the diagram
    Owner,
    /// User has edit permission via sharing
    Editor,
    /// User has view permission via sharing
    Viewer,
    /// Diagram is public (view only)
    Public,
    /// No access
    None,
}

impl DiagramAccess {
    /// Check if user can view the diagram
    pub fn can_view(&self) -> bool {
        !matches!(self, DiagramAccess::None)
    }

    /// Check if user can edit the diagram
    pub fn can_edit(&self) -> bool {
        matches!(self, DiagramAccess::Owner | DiagramAccess::Editor)
    }

    /// Check if user can delete the diagram
    pub fn can_delete(&self) -> bool {
        matches!(self, DiagramAccess::Owner)
    }

    /// Check if user can share the diagram
    pub fn can_share(&self) -> bool {
        matches!(self, DiagramAccess::Owner)
    }
}

/// Diagram repository for database operations
#[derive(Clone)]
pub struct DiagramRepository {
    pool: PgPool,
}

impl DiagramRepository {
    /// Create a new diagram repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new diagram
    pub async fn create(&self, dto: &CreateDiagram) -> Result<Diagram, DiagramRepositoryError> {
        // Verify folder exists if specified
        if let Some(folder_id) = dto.folder_id {
            let folder_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND owner_id = $2)",
            )
            .bind(folder_id)
            .bind(dto.owner_id)
            .fetch_one(&self.pool)
            .await?;

            if !folder_exists {
                return Err(DiagramRepositoryError::FolderNotFound);
            }
        }

        let diagram = sqlx::query_as::<_, Diagram>(
            r#"
            INSERT INTO diagrams (owner_id, folder_id, name, description, schema_data, is_public)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, owner_id, folder_id, name, description, schema_data, is_public, created_at, updated_at
            "#,
        )
        .bind(dto.owner_id)
        .bind(dto.folder_id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(sqlx::types::Json(&dto.schema_data))
        .bind(dto.is_public)
        .fetch_one(&self.pool)
        .await?;

        Ok(diagram)
    }

    /// Find a diagram by ID (no permission check)
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Diagram>, DiagramRepositoryError> {
        let diagram = sqlx::query_as::<_, Diagram>(
            r#"
            SELECT id, owner_id, folder_id, name, description, schema_data, is_public, created_at, updated_at
            FROM diagrams
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(diagram)
    }

    /// Find a diagram by ID with permission check
    pub async fn find_by_id_with_access(
        &self,
        id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<Option<(Diagram, DiagramAccess)>, DiagramRepositoryError> {
        let diagram = match self.find_by_id(id).await? {
            Some(d) => d,
            None => return Ok(None),
        };

        let access = self.get_access_level(id, user_id).await?;

        if access.can_view() {
            Ok(Some((diagram, access)))
        } else {
            Ok(None)
        }
    }

    /// Get access level for a user on a diagram
    pub async fn get_access_level(
        &self,
        diagram_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<DiagramAccess, DiagramRepositoryError> {
        // Check if diagram exists and get basic info
        let diagram_info = sqlx::query_as::<_, (Uuid, bool)>(
            "SELECT owner_id, is_public FROM diagrams WHERE id = $1",
        )
        .bind(diagram_id)
        .fetch_optional(&self.pool)
        .await?;

        let (owner_id, is_public) = match diagram_info {
            Some(info) => info,
            None => return Ok(DiagramAccess::None),
        };

        // Check if user is owner
        if let Some(uid) = user_id {
            if uid == owner_id {
                return Ok(DiagramAccess::Owner);
            }

            // Check if user has shared access
            let permission = sqlx::query_scalar::<_, String>(
                "SELECT permission FROM diagram_shares WHERE diagram_id = $1 AND user_id = $2",
            )
            .bind(diagram_id)
            .bind(uid)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(perm) = permission {
                return Ok(match perm.as_str() {
                    "edit" => DiagramAccess::Editor,
                    "view" => DiagramAccess::Viewer,
                    _ => DiagramAccess::Viewer,
                });
            }
        }

        // Check if public
        if is_public {
            return Ok(DiagramAccess::Public);
        }

        Ok(DiagramAccess::None)
    }

    /// List diagrams owned by a user
    pub async fn list_by_owner(
        &self,
        owner_id: Uuid,
        folder_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiagramSummary>, DiagramRepositoryError> {
        let diagrams = if let Some(fid) = folder_id {
            sqlx::query_as::<_, DiagramSummary>(
                r#"
                SELECT id, owner_id, folder_id, name, description, is_public, created_at, updated_at
                FROM diagrams
                WHERE owner_id = $1 AND folder_id = $2
                ORDER BY updated_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(owner_id)
            .bind(fid)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DiagramSummary>(
                r#"
                SELECT id, owner_id, folder_id, name, description, is_public, created_at, updated_at
                FROM diagrams
                WHERE owner_id = $1 AND folder_id IS NULL
                ORDER BY updated_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(owner_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(diagrams)
    }

    /// List all diagrams owned by a user (regardless of folder)
    pub async fn list_all_by_owner(
        &self,
        owner_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiagramSummary>, DiagramRepositoryError> {
        let diagrams = sqlx::query_as::<_, DiagramSummary>(
            r#"
            SELECT id, owner_id, folder_id, name, description, is_public, created_at, updated_at
            FROM diagrams
            WHERE owner_id = $1
            ORDER BY updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(owner_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(diagrams)
    }

    /// List diagrams shared with a user
    pub async fn list_shared_with(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(DiagramSummary, SharePermission)>, DiagramRepositoryError> {
        let rows = sqlx::query_as::<_, SharedDiagramInfo>(
            r#"
            SELECT
                d.id, d.owner_id, d.folder_id, d.name, d.description, d.is_public, d.created_at, d.updated_at,
                ds.permission
            FROM diagrams d
            INNER JOIN diagram_shares ds ON d.id = ds.diagram_id
            WHERE ds.user_id = $1
            ORDER BY d.updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let result = rows
            .into_iter()
            .map(|row| {
                let summary = DiagramSummary {
                    id: row.id,
                    owner_id: row.owner_id,
                    folder_id: row.folder_id,
                    name: row.name,
                    description: row.description,
                    is_public: row.is_public,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                let permission = row.permission.parse().unwrap_or(SharePermission::View);
                (summary, permission)
            })
            .collect();

        Ok(result)
    }

    /// Update a diagram
    pub async fn update(
        &self,
        id: Uuid,
        updates: &UpdateDiagram,
    ) -> Result<Diagram, DiagramRepositoryError> {
        // Build dynamic update query
        let mut set_clauses = Vec::new();
        let mut param_count = 1;

        if updates.name.is_some() {
            param_count += 1;
            set_clauses.push(format!("name = ${}", param_count));
        }
        if updates.description.is_some() {
            param_count += 1;
            set_clauses.push(format!("description = ${}", param_count));
        }
        if updates.schema_data.is_some() {
            param_count += 1;
            set_clauses.push(format!("schema_data = ${}", param_count));
        }
        if updates.is_public.is_some() {
            param_count += 1;
            set_clauses.push(format!("is_public = ${}", param_count));
        }
        if updates.folder_id.is_some() {
            param_count += 1;
            set_clauses.push(format!("folder_id = ${}", param_count));
        }

        if set_clauses.is_empty() {
            // No updates, just return the existing diagram
            return self
                .find_by_id(id)
                .await?
                .ok_or(DiagramRepositoryError::NotFound);
        }

        // Use a simpler approach with COALESCE
        let diagram = sqlx::query_as::<_, Diagram>(
            r#"
            UPDATE diagrams
            SET
                name = COALESCE($2, name),
                description = CASE WHEN $3::boolean THEN $4 ELSE description END,
                schema_data = COALESCE($5, schema_data),
                is_public = COALESCE($6, is_public),
                folder_id = CASE WHEN $7::boolean THEN $8 ELSE folder_id END
            WHERE id = $1
            RETURNING id, owner_id, folder_id, name, description, schema_data, is_public, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&updates.name)
        .bind(updates.description.is_some()) // Flag for description update
        .bind(updates.description.as_ref().and_then(|d| d.as_ref())) // Actual description value
        .bind(updates.schema_data.as_ref().map(sqlx::types::Json))
        .bind(updates.is_public)
        .bind(updates.folder_id.is_some()) // Flag for folder_id update
        .bind(updates.folder_id.flatten()) // Actual folder_id value
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DiagramRepositoryError::NotFound)?;

        Ok(diagram)
    }

    /// Update only the schema_data (for autosave)
    pub async fn update_schema(
        &self,
        id: Uuid,
        schema_data: &serde_json::Value,
    ) -> Result<(), DiagramRepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE diagrams
            SET schema_data = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(sqlx::types::Json(schema_data))
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DiagramRepositoryError::NotFound);
        }

        Ok(())
    }

    /// Move diagram to a different folder
    pub async fn move_to_folder(
        &self,
        id: Uuid,
        owner_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<(), DiagramRepositoryError> {
        // Verify folder exists and belongs to user if specified
        if let Some(fid) = folder_id {
            let folder_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND owner_id = $2)",
            )
            .bind(fid)
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await?;

            if !folder_exists {
                return Err(DiagramRepositoryError::FolderNotFound);
            }
        }

        let result = sqlx::query(
            r#"
            UPDATE diagrams
            SET folder_id = $2
            WHERE id = $1 AND owner_id = $3
            "#,
        )
        .bind(id)
        .bind(folder_id)
        .bind(owner_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DiagramRepositoryError::NotFound);
        }

        Ok(())
    }

    /// Delete a diagram
    pub async fn delete(&self, id: Uuid) -> Result<bool, DiagramRepositoryError> {
        let result = sqlx::query("DELETE FROM diagrams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a diagram with owner check
    pub async fn delete_by_owner(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<bool, DiagramRepositoryError> {
        let result = sqlx::query("DELETE FROM diagrams WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Count diagrams owned by a user
    pub async fn count_by_owner(&self, owner_id: Uuid) -> Result<i64, DiagramRepositoryError> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM diagrams WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Count diagrams in a folder
    pub async fn count_in_folder(
        &self,
        owner_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<i64, DiagramRepositoryError> {
        let count: (i64,) = if let Some(fid) = folder_id {
            sqlx::query_as("SELECT COUNT(*) FROM diagrams WHERE owner_id = $1 AND folder_id = $2")
                .bind(owner_id)
                .bind(fid)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as(
                "SELECT COUNT(*) FROM diagrams WHERE owner_id = $1 AND folder_id IS NULL",
            )
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count.0)
    }

    /// Search diagrams by name (owned or shared with user)
    pub async fn search(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<DiagramSummary>, DiagramRepositoryError> {
        let search_pattern = format!("%{}%", query);

        let diagrams = sqlx::query_as::<_, DiagramSummary>(
            r#"
            SELECT DISTINCT d.id, d.owner_id, d.folder_id, d.name, d.description, d.is_public, d.created_at, d.updated_at
            FROM diagrams d
            LEFT JOIN diagram_shares ds ON d.id = ds.diagram_id AND ds.user_id = $1
            WHERE (d.owner_id = $1 OR ds.user_id IS NOT NULL OR d.is_public = true)
              AND d.name ILIKE $2
            ORDER BY d.updated_at DESC
            LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(diagrams)
    }

    /// Get recent diagrams for a user (owned or shared)
    pub async fn get_recent(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<DiagramSummary>, DiagramRepositoryError> {
        let diagrams = sqlx::query_as::<_, DiagramSummary>(
            r#"
            SELECT DISTINCT d.id, d.owner_id, d.folder_id, d.name, d.description, d.is_public, d.created_at, d.updated_at
            FROM diagrams d
            LEFT JOIN diagram_shares ds ON d.id = ds.diagram_id AND ds.user_id = $1
            WHERE d.owner_id = $1 OR ds.user_id IS NOT NULL
            ORDER BY d.updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(diagrams)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // DiagramAccess Tests
    // ========================================================================

    #[test]
    fn test_diagram_access_owner() {
        let access = DiagramAccess::Owner;
        assert!(access.can_view());
        assert!(access.can_edit());
        assert!(access.can_delete());
        assert!(access.can_share());
    }

    #[test]
    fn test_diagram_access_editor() {
        let access = DiagramAccess::Editor;
        assert!(access.can_view());
        assert!(access.can_edit());
        assert!(!access.can_delete());
        assert!(!access.can_share());
    }

    #[test]
    fn test_diagram_access_viewer() {
        let access = DiagramAccess::Viewer;
        assert!(access.can_view());
        assert!(!access.can_edit());
        assert!(!access.can_delete());
        assert!(!access.can_share());
    }

    #[test]
    fn test_diagram_access_public() {
        let access = DiagramAccess::Public;
        assert!(access.can_view());
        assert!(!access.can_edit());
        assert!(!access.can_delete());
        assert!(!access.can_share());
    }

    #[test]
    fn test_diagram_access_none() {
        let access = DiagramAccess::None;
        assert!(!access.can_view());
        assert!(!access.can_edit());
        assert!(!access.can_delete());
        assert!(!access.can_share());
    }

    // ========================================================================
    // Error Tests
    // ========================================================================

    #[test]
    fn test_diagram_repository_error_display() {
        let err = DiagramRepositoryError::NotFound;
        assert_eq!(format!("{}", err), "Diagram not found");

        let err = DiagramRepositoryError::AccessDenied;
        assert_eq!(format!("{}", err), "Access denied");

        let err = DiagramRepositoryError::FolderNotFound;
        assert_eq!(format!("{}", err), "Folder not found");
    }

    #[test]
    fn test_diagram_repository_error_debug() {
        let err = DiagramRepositoryError::NotFound;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    // ========================================================================
    // Integration Tests (require database)
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_diagram() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Test Diagram".to_string(),
            description: Some("A test diagram".to_string()),
            schema_data: serde_json::json!({"tables": []}),
            is_public: false,
        };

        let diagram = repo.create(&dto).await.unwrap();

        assert_eq!(diagram.owner_id, user_id);
        assert_eq!(diagram.name, "Test Diagram");
        assert!(!diagram.is_public);

        // Cleanup
        repo.delete(diagram.id).await.unwrap();
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_find_by_id() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Find Test".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: false,
        };

        let created = repo.create(&dto).await.unwrap();
        let found = repo.find_by_id(created.id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);

        // Cleanup
        repo.delete(created.id).await.unwrap();
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_access_level_owner() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Access Test".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: false,
        };

        let diagram = repo.create(&dto).await.unwrap();
        let access = repo
            .get_access_level(diagram.id, Some(user_id))
            .await
            .unwrap();

        assert_eq!(access, DiagramAccess::Owner);

        // Cleanup
        repo.delete(diagram.id).await.unwrap();
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_access_level_public() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Public Test".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: true,
        };

        let diagram = repo.create(&dto).await.unwrap();

        // Anonymous user should have public access
        let access = repo.get_access_level(diagram.id, None).await.unwrap();
        assert_eq!(access, DiagramAccess::Public);

        // Cleanup
        repo.delete(diagram.id).await.unwrap();
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_access_level_none() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Private Test".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: false,
        };

        let diagram = repo.create(&dto).await.unwrap();

        // Anonymous user should have no access to private diagram
        let access = repo.get_access_level(diagram.id, None).await.unwrap();
        assert_eq!(access, DiagramAccess::None);

        // Cleanup
        repo.delete(diagram.id).await.unwrap();
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_update_schema() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Update Test".to_string(),
            description: None,
            schema_data: serde_json::json!({"version": 1}),
            is_public: false,
        };

        let diagram = repo.create(&dto).await.unwrap();

        // Update schema
        let new_schema = serde_json::json!({"version": 2, "tables": []});
        repo.update_schema(diagram.id, &new_schema).await.unwrap();

        // Verify update
        let updated = repo.find_by_id(diagram.id).await.unwrap().unwrap();
        assert_eq!(updated.schema_data.0["version"], 2);

        // Cleanup
        repo.delete(diagram.id).await.unwrap();
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_list_by_owner() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        // Create multiple diagrams
        for i in 1..=3 {
            let dto = CreateDiagram {
                owner_id: user_id,
                folder_id: None,
                name: format!("List Test {}", i),
                description: None,
                schema_data: serde_json::json!({}),
                is_public: false,
            };
            repo.create(&dto).await.unwrap();
        }

        let diagrams = repo.list_by_owner(user_id, None, 10, 0).await.unwrap();
        assert!(diagrams.len() >= 3);

        // Cleanup
        for diagram in diagrams {
            repo.delete(diagram.id).await.unwrap();
        }
        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_by_owner() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Delete Test".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: false,
        };

        let diagram = repo.create(&dto).await.unwrap();

        // Delete by owner
        let deleted = repo.delete_by_owner(diagram.id, user_id).await.unwrap();
        assert!(deleted);

        // Verify deletion
        let found = repo.find_by_id(diagram.id).await.unwrap();
        assert!(found.is_none());

        cleanup_test_user(&pool, user_id).await;
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_delete_by_wrong_owner() {
        let (pool, user_id) = setup_test_user().await;
        let repo = DiagramRepository::new(pool.clone());

        let dto = CreateDiagram {
            owner_id: user_id,
            folder_id: None,
            name: "Delete Test 2".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: false,
        };

        let diagram = repo.create(&dto).await.unwrap();

        // Try to delete with wrong owner
        let wrong_user_id = Uuid::new_v4();
        let deleted = repo
            .delete_by_owner(diagram.id, wrong_user_id)
            .await
            .unwrap();
        assert!(!deleted);

        // Verify still exists
        let found = repo.find_by_id(diagram.id).await.unwrap();
        assert!(found.is_some());

        // Cleanup
        repo.delete(diagram.id).await.unwrap();
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

        let user_id = Uuid::new_v4();
        let unique_email = format!("diagram_test_{}@example.com", user_id);
        let unique_username = format!("diagram_test_{}", &user_id.to_string()[..8]);

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
        // Diagrams and shares will be deleted by CASCADE
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .expect("Failed to cleanup test user");
    }
}
