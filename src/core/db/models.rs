//! Database models for Archischema
//!
//! This module defines the database entity structs that map to PostgreSQL tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Helper module for deserializing Option<Option<T>> where:
/// - Missing field -> None (don't update)
/// - Field with null -> Some(None) (set to null)
/// - Field with value -> Some(Some(value)) (set to value)
pub mod double_option {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        // This will be called only when the field is present
        // So we wrap the result in Some()
        Option::<T>::deserialize(deserializer).map(Some)
    }
}

// ============================================================================
// User Model
// ============================================================================

/// User entity representing a registered user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User data for creation (without id and timestamps)
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password_hash: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

/// User data for updates
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub password_hash: Option<String>,
}

/// User without sensitive data (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            avatar_url: user.avatar_url,
            created_at: user.created_at,
        }
    }
}

// ============================================================================
// Folder Model
// ============================================================================

/// Folder entity for organizing diagrams
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Folder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Folder data for creation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFolder {
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
}

/// Folder data for updates
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateFolder {
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub parent_id: Option<Option<Uuid>>, // None = don't update, Some(None) = set to null, Some(Some(id)) = set to id
    pub name: Option<String>,
}

// ============================================================================
// Diagram Model
// ============================================================================

/// Diagram entity representing a database schema diagram
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Diagram {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub schema_data: sqlx::types::Json<serde_json::Value>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diagram data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiagram {
    pub owner_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub schema_data: serde_json::Value,
    pub is_public: bool,
}

impl Default for CreateDiagram {
    fn default() -> Self {
        Self {
            owner_id: Uuid::nil(),
            folder_id: None,
            name: "Untitled Diagram".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: false,
        }
    }
}

/// Diagram data for updates
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateDiagram {
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub folder_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub description: Option<Option<String>>,
    pub schema_data: Option<serde_json::Value>,
    pub is_public: Option<bool>,
}

/// Diagram summary for list views (without full schema_data)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiagramSummary {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Diagram Share Model
// ============================================================================

/// Permission levels for shared diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SharePermission {
    #[default]
    View,
    Edit,
}

impl std::fmt::Display for SharePermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharePermission::View => write!(f, "view"),
            SharePermission::Edit => write!(f, "edit"),
        }
    }
}

impl std::str::FromStr for SharePermission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "view" => Ok(SharePermission::View),
            "edit" => Ok(SharePermission::Edit),
            _ => Err(format!("Invalid permission: {}", s)),
        }
    }
}

/// Diagram share entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiagramShare {
    pub id: Uuid,
    pub diagram_id: Uuid,
    pub user_id: Uuid,
    pub permission: String, // 'view' or 'edit'
    pub created_at: DateTime<Utc>,
}

/// Diagram share data for creation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDiagramShare {
    pub diagram_id: Uuid,
    pub user_id: Uuid,
    pub permission: SharePermission,
}

/// Diagram share with user info (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiagramShareWithUser {
    pub id: Uuid,
    pub diagram_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub permission: String,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Session Model
// ============================================================================

/// Session entity for refresh tokens
#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Session data for creation
#[derive(Debug, Clone)]
pub struct CreateSession {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_response_from_user() {
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "secret_hash".to_string(),
            username: "testuser".to_string(),
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.clone().into();

        assert_eq!(response.id, user.id);
        assert_eq!(response.email, user.email);
        assert_eq!(response.username, user.username);
        // password_hash should not be in response
    }

    #[test]
    fn test_share_permission_display() {
        assert_eq!(SharePermission::View.to_string(), "view");
        assert_eq!(SharePermission::Edit.to_string(), "edit");
    }

    #[test]
    fn test_share_permission_from_str() {
        assert_eq!(
            "view".parse::<SharePermission>().unwrap(),
            SharePermission::View
        );
        assert_eq!(
            "edit".parse::<SharePermission>().unwrap(),
            SharePermission::Edit
        );
        assert_eq!(
            "VIEW".parse::<SharePermission>().unwrap(),
            SharePermission::View
        );
        assert!("invalid".parse::<SharePermission>().is_err());
    }

    #[test]
    fn test_create_diagram_default() {
        let diagram = CreateDiagram::default();
        assert_eq!(diagram.name, "Untitled Diagram");
        assert!(!diagram.is_public);
        assert!(diagram.folder_id.is_none());
        assert!(diagram.description.is_none());
        assert_eq!(diagram.schema_data, serde_json::json!({}));
        assert!(diagram.owner_id.is_nil());
    }

    // ========================================================================
    // User Model Tests
    // ========================================================================

    #[test]
    fn test_user_serialization_skips_password_hash() {
        let user = User {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            email: "test@example.com".to_string(),
            password_hash: "super_secret_hash".to_string(),
            username: "testuser".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&user).unwrap();

        // password_hash should be skipped during serialization
        assert!(!json.contains("super_secret_hash"));
        assert!(!json.contains("password_hash"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_user_response_excludes_sensitive_fields() {
        let user = User {
            id: Uuid::new_v4(),
            email: "secure@example.com".to_string(),
            password_hash: "hashed_password_123".to_string(),
            username: "secureuser".to_string(),
            avatar_url: Some("https://example.com/pic.jpg".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.clone().into();
        let json = serde_json::to_string(&response).unwrap();

        assert!(!json.contains("hashed_password_123"));
        assert!(!json.contains("updated_at"));
        assert!(json.contains("secure@example.com"));
        assert!(json.contains("secureuser"));
        assert!(json.contains("created_at"));
    }

    #[test]
    fn test_user_with_avatar_url() {
        let user = User {
            id: Uuid::new_v4(),
            email: "avatar@test.com".to_string(),
            password_hash: "hash".to_string(),
            username: "avataruser".to_string(),
            avatar_url: Some("https://cdn.example.com/avatar/123.png".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.into();
        assert_eq!(
            response.avatar_url,
            Some("https://cdn.example.com/avatar/123.png".to_string())
        );
    }

    #[test]
    fn test_user_without_avatar_url() {
        let user = User {
            id: Uuid::new_v4(),
            email: "noavatar@test.com".to_string(),
            password_hash: "hash".to_string(),
            username: "noavataruser".to_string(),
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.into();
        assert!(response.avatar_url.is_none());
    }

    #[test]
    fn test_create_user_deserialization() {
        let json = r#"{
            "email": "new@example.com",
            "password_hash": "argon2hash",
            "username": "newuser",
            "avatar_url": null
        }"#;

        let create_user: CreateUser = serde_json::from_str(json).unwrap();
        assert_eq!(create_user.email, "new@example.com");
        assert_eq!(create_user.password_hash, "argon2hash");
        assert_eq!(create_user.username, "newuser");
        assert!(create_user.avatar_url.is_none());
    }

    #[test]
    fn test_create_user_with_avatar() {
        let json = r#"{
            "email": "avatar@example.com",
            "password_hash": "hash123",
            "username": "avataruser",
            "avatar_url": "https://example.com/avatar.jpg"
        }"#;

        let create_user: CreateUser = serde_json::from_str(json).unwrap();
        assert_eq!(
            create_user.avatar_url,
            Some("https://example.com/avatar.jpg".to_string())
        );
    }

    #[test]
    fn test_update_user_partial() {
        let json = r#"{"username": "newname"}"#;
        let update: UpdateUser = serde_json::from_str(json).unwrap();

        assert_eq!(update.username, Some("newname".to_string()));
        assert!(update.email.is_none());
        assert!(update.avatar_url.is_none());
        assert!(update.password_hash.is_none());
    }

    #[test]
    fn test_update_user_default() {
        let update = UpdateUser::default();
        assert!(update.email.is_none());
        assert!(update.username.is_none());
        assert!(update.avatar_url.is_none());
        assert!(update.password_hash.is_none());
    }

    #[test]
    fn test_update_user_full() {
        let json = r#"{
            "email": "updated@example.com",
            "username": "updateduser",
            "avatar_url": "https://new-avatar.com/pic.png",
            "password_hash": "newhash"
        }"#;

        let update: UpdateUser = serde_json::from_str(json).unwrap();
        assert_eq!(update.email, Some("updated@example.com".to_string()));
        assert_eq!(update.username, Some("updateduser".to_string()));
        assert_eq!(
            update.avatar_url,
            Some("https://new-avatar.com/pic.png".to_string())
        );
        assert_eq!(update.password_hash, Some("newhash".to_string()));
    }

    // ========================================================================
    // Folder Model Tests
    // ========================================================================

    #[test]
    fn test_folder_serialization() {
        let folder = Folder {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            parent_id: None,
            name: "My Project".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&folder).unwrap();
        assert!(json.contains("My Project"));
        assert!(json.contains("owner_id"));
    }

    #[test]
    fn test_folder_with_parent() {
        let parent_id = Uuid::new_v4();
        let folder = Folder {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            parent_id: Some(parent_id),
            name: "Subfolder".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(folder.parent_id, Some(parent_id));
    }

    #[test]
    fn test_create_folder_deserialization() {
        let owner_id = Uuid::new_v4();
        let json = format!(
            r#"{{"owner_id": "{}", "parent_id": null, "name": "New Folder"}}"#,
            owner_id
        );

        let create_folder: CreateFolder = serde_json::from_str(&json).unwrap();
        assert_eq!(create_folder.owner_id, owner_id);
        assert!(create_folder.parent_id.is_none());
        assert_eq!(create_folder.name, "New Folder");
    }

    #[test]
    fn test_create_folder_with_parent() {
        let owner_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"owner_id": "{}", "parent_id": "{}", "name": "Nested Folder"}}"#,
            owner_id, parent_id
        );

        let create_folder: CreateFolder = serde_json::from_str(&json).unwrap();
        assert_eq!(create_folder.parent_id, Some(parent_id));
    }

    #[test]
    fn test_update_folder_default() {
        let update = UpdateFolder::default();
        assert!(update.parent_id.is_none());
        assert!(update.name.is_none());
    }

    #[test]
    fn test_update_folder_name_only() {
        let json = r#"{"name": "Renamed Folder"}"#;
        let update: UpdateFolder = serde_json::from_str(json).unwrap();

        assert_eq!(update.name, Some("Renamed Folder".to_string()));
        assert!(update.parent_id.is_none());
    }

    // ========================================================================
    // Diagram Model Tests
    // ========================================================================

    #[test]
    fn test_diagram_with_schema_data() {
        let schema_data = serde_json::json!({
            "tables": [
                {"name": "users", "columns": [{"name": "id", "type": "uuid"}]}
            ],
            "relationships": []
        });

        let diagram = CreateDiagram {
            owner_id: Uuid::new_v4(),
            folder_id: None,
            name: "User Schema".to_string(),
            description: Some("Database schema for users".to_string()),
            schema_data: schema_data.clone(),
            is_public: false,
        };

        assert_eq!(diagram.schema_data["tables"][0]["name"], "users");
    }

    #[test]
    fn test_diagram_public_visibility() {
        let diagram = CreateDiagram {
            owner_id: Uuid::new_v4(),
            folder_id: None,
            name: "Public Diagram".to_string(),
            description: None,
            schema_data: serde_json::json!({}),
            is_public: true,
        };

        assert!(diagram.is_public);
    }

    #[test]
    fn test_update_diagram_default() {
        let update = UpdateDiagram::default();
        assert!(update.folder_id.is_none());
        assert!(update.name.is_none());
        assert!(update.description.is_none());
        assert!(update.schema_data.is_none());
        assert!(update.is_public.is_none());
    }

    #[test]
    fn test_update_diagram_partial() {
        let json = r#"{"name": "Updated Name", "is_public": true}"#;
        let update: UpdateDiagram = serde_json::from_str(json).unwrap();

        assert_eq!(update.name, Some("Updated Name".to_string()));
        assert_eq!(update.is_public, Some(true));
        assert!(update.folder_id.is_none());
        assert!(update.schema_data.is_none());
    }

    #[test]
    fn test_update_diagram_with_schema() {
        let json = r#"{"schema_data": {"tables": [], "version": 2}}"#;
        let update: UpdateDiagram = serde_json::from_str(json).unwrap();

        assert!(update.schema_data.is_some());
        let schema = update.schema_data.unwrap();
        assert_eq!(schema["version"], 2);
    }

    #[test]
    fn test_diagram_summary_excludes_schema_data() {
        // DiagramSummary should not have schema_data field
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "owner_id": "550e8400-e29b-41d4-a716-446655440001",
            "folder_id": null,
            "name": "Test Diagram",
            "description": "A test",
            "is_public": false,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let summary: DiagramSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.name, "Test Diagram");
        assert_eq!(summary.description, Some("A test".to_string()));
    }

    // ========================================================================
    // SharePermission Tests
    // ========================================================================

    #[test]
    fn test_share_permission_default() {
        let perm = SharePermission::default();
        assert_eq!(perm, SharePermission::View);
    }

    #[test]
    fn test_share_permission_equality() {
        assert_eq!(SharePermission::View, SharePermission::View);
        assert_eq!(SharePermission::Edit, SharePermission::Edit);
        assert_ne!(SharePermission::View, SharePermission::Edit);
    }

    #[test]
    fn test_share_permission_copy() {
        let perm = SharePermission::Edit;
        let copied = perm;
        assert_eq!(perm, copied);
    }

    #[test]
    fn test_share_permission_serialization() {
        let view_json = serde_json::to_string(&SharePermission::View).unwrap();
        let edit_json = serde_json::to_string(&SharePermission::Edit).unwrap();

        assert_eq!(view_json, r#""view""#);
        assert_eq!(edit_json, r#""edit""#);
    }

    #[test]
    fn test_share_permission_deserialization() {
        let view: SharePermission = serde_json::from_str(r#""view""#).unwrap();
        let edit: SharePermission = serde_json::from_str(r#""edit""#).unwrap();

        assert_eq!(view, SharePermission::View);
        assert_eq!(edit, SharePermission::Edit);
    }

    #[test]
    fn test_share_permission_from_str_case_insensitive() {
        assert_eq!(
            "View".parse::<SharePermission>().unwrap(),
            SharePermission::View
        );
        assert_eq!(
            "EDIT".parse::<SharePermission>().unwrap(),
            SharePermission::Edit
        );
        assert_eq!(
            "ViEw".parse::<SharePermission>().unwrap(),
            SharePermission::View
        );
        assert_eq!(
            "eDiT".parse::<SharePermission>().unwrap(),
            SharePermission::Edit
        );
    }

    #[test]
    fn test_share_permission_from_str_invalid() {
        let result = "read".parse::<SharePermission>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid permission"));
    }

    // ========================================================================
    // DiagramShare Tests
    // ========================================================================

    #[test]
    fn test_create_diagram_share_deserialization() {
        let diagram_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let json = format!(
            r#"{{"diagram_id": "{}", "user_id": "{}", "permission": "edit"}}"#,
            diagram_id, user_id
        );

        let share: CreateDiagramShare = serde_json::from_str(&json).unwrap();
        assert_eq!(share.diagram_id, diagram_id);
        assert_eq!(share.user_id, user_id);
        assert_eq!(share.permission, SharePermission::Edit);
    }

    #[test]
    fn test_diagram_share_with_user_serialization() {
        let share = DiagramShareWithUser {
            id: Uuid::new_v4(),
            diagram_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            username: "shareduser".to_string(),
            email: "shared@example.com".to_string(),
            permission: "view".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&share).unwrap();
        assert!(json.contains("shareduser"));
        assert!(json.contains("shared@example.com"));
        assert!(json.contains(r#""permission":"view""#));
    }

    // ========================================================================
    // Session Tests
    // ========================================================================

    #[test]
    fn test_create_session() {
        let user_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::days(7);

        let session = CreateSession {
            user_id,
            token_hash: "sha256_hash_of_refresh_token".to_string(),
            expires_at,
        };

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.token_hash, "sha256_hash_of_refresh_token");
        assert!(session.expires_at > Utc::now());
    }

    #[test]
    fn test_session_clone() {
        let session = Session {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token_hash: "hash123".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
        };

        let cloned = session.clone();
        assert_eq!(session.id, cloned.id);
        assert_eq!(session.token_hash, cloned.token_hash);
    }

    // ========================================================================
    // Edge Cases and Boundary Tests
    // ========================================================================

    #[test]
    fn test_empty_string_fields() {
        let json = r#"{
            "email": "",
            "password_hash": "",
            "username": "",
            "avatar_url": null
        }"#;

        let create_user: CreateUser = serde_json::from_str(json).unwrap();
        assert_eq!(create_user.email, "");
        assert_eq!(create_user.username, "");
    }

    #[test]
    fn test_unicode_in_names() {
        let json = r#"{
            "email": "用户@例子.中国",
            "password_hash": "hash",
            "username": "пользователь",
            "avatar_url": null
        }"#;

        let create_user: CreateUser = serde_json::from_str(json).unwrap();
        assert_eq!(create_user.email, "用户@例子.中国");
        assert_eq!(create_user.username, "пользователь");
    }

    #[test]
    fn test_special_characters_in_description() {
        let diagram = CreateDiagram {
            owner_id: Uuid::new_v4(),
            folder_id: None,
            name: "Test <script>alert('xss')</script>".to_string(),
            description: Some("Description with \"quotes\" and 'apostrophes'".to_string()),
            schema_data: serde_json::json!({}),
            is_public: false,
        };

        let json = serde_json::to_string(&diagram).unwrap();
        // Should properly escape special characters
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_large_schema_data() {
        let mut tables = Vec::new();
        for i in 0..100 {
            tables.push(serde_json::json!({
                "name": format!("table_{}", i),
                "columns": [
                    {"name": "id", "type": "uuid"},
                    {"name": "data", "type": "jsonb"}
                ]
            }));
        }

        let diagram = CreateDiagram {
            owner_id: Uuid::new_v4(),
            folder_id: None,
            name: "Large Schema".to_string(),
            description: None,
            schema_data: serde_json::json!({"tables": tables}),
            is_public: false,
        };

        let json = serde_json::to_string(&diagram).unwrap();
        assert!(json.len() > 1000);
    }

    #[test]
    fn test_nil_uuid_handling() {
        let nil_uuid = Uuid::nil();
        let user = User {
            id: nil_uuid,
            email: "nil@test.com".to_string(),
            password_hash: "hash".to_string(),
            username: "niluser".to_string(),
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(user.id.is_nil());
        let response: UserResponse = user.into();
        assert!(response.id.is_nil());
    }

    #[test]
    fn test_update_folder_set_parent_to_null() {
        // Testing Option<Option<Uuid>> for clearing parent
        let json = r#"{"parent_id": null}"#;
        let update: UpdateFolder = serde_json::from_str(json).unwrap();

        // Some(None) means "set parent_id to NULL"
        assert_eq!(update.parent_id, Some(None));
    }

    #[test]
    fn test_update_folder_set_parent_to_value() {
        let parent_id = Uuid::new_v4();
        let json = format!(r#"{{"parent_id": "{}"}}"#, parent_id);
        let update: UpdateFolder = serde_json::from_str(&json).unwrap();

        // Some(Some(id)) means "set parent_id to this ID"
        assert_eq!(update.parent_id, Some(Some(parent_id)));
    }

    #[test]
    fn test_user_response_json_roundtrip() {
        let response = UserResponse {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            email: "roundtrip@test.com".to_string(),
            username: "roundtripuser".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: UserResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.id, deserialized.id);
        assert_eq!(response.email, deserialized.email);
        assert_eq!(response.username, deserialized.username);
        assert_eq!(response.avatar_url, deserialized.avatar_url);
    }

    #[test]
    fn test_diagram_share_json_roundtrip() {
        let share = DiagramShare {
            id: Uuid::new_v4(),
            diagram_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            permission: "edit".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&share).unwrap();
        let deserialized: DiagramShare = serde_json::from_str(&json).unwrap();

        assert_eq!(share.id, deserialized.id);
        assert_eq!(share.permission, deserialized.permission);
    }
}
