//! Folder API endpoints
//!
//! Provides REST API endpoints for folder management:
//! - POST /api/folders - Create a new folder (auth required)
//! - GET /api/folders - List user's folders
//! - GET /api/folders/tree - Get folder tree structure
//! - GET /api/folders/:id - Get folder by ID
//! - GET /api/folders/:id/path - Get folder path (breadcrumb)
//! - GET /api/folders/:id/children - Get folder children
//! - PUT /api/folders/:id - Update folder (rename)
//! - PATCH /api/folders/:id/move - Move folder to new parent
//! - DELETE /api/folders/:id - Delete folder

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::core::auth::JwtService;
use crate::core::db::models::{CreateFolder, Folder};
use crate::core::db::repositories::{
    FolderNode, FolderRepository, FolderRepositoryError, FolderWithDepth,
};

/// Folder API state containing the folder repository and JWT service
#[derive(Clone)]
pub struct FolderApiState {
    pub folder_repo: FolderRepository,
    pub jwt_service: JwtService,
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

/// Folder API error types
#[derive(Debug, thiserror::Error)]
pub enum FolderApiError {
    #[error("Folder not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("Authentication required")]
    Unauthorized,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Parent folder not found")]
    ParentNotFound,

    #[error("Cannot move folder into itself or its descendants")]
    CircularReference,

    #[error("Folder name already exists in this location")]
    NameAlreadyExists,

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<FolderRepositoryError> for FolderApiError {
    fn from(err: FolderRepositoryError) -> Self {
        match err {
            FolderRepositoryError::NotFound => FolderApiError::NotFound,
            FolderRepositoryError::AccessDenied => FolderApiError::AccessDenied,
            FolderRepositoryError::ParentNotFound => FolderApiError::ParentNotFound,
            FolderRepositoryError::CircularReference => FolderApiError::CircularReference,
            FolderRepositoryError::NameAlreadyExists => FolderApiError::NameAlreadyExists,
            FolderRepositoryError::DatabaseError(e) => FolderApiError::InternalError(e.to_string()),
        }
    }
}

impl IntoResponse for FolderApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            FolderApiError::NotFound => (StatusCode::NOT_FOUND, "FOLDER_NOT_FOUND"),
            FolderApiError::AccessDenied => (StatusCode::FORBIDDEN, "ACCESS_DENIED"),
            FolderApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            FolderApiError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN"),
            FolderApiError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED"),
            FolderApiError::ParentNotFound => (StatusCode::BAD_REQUEST, "PARENT_NOT_FOUND"),
            FolderApiError::CircularReference => (StatusCode::BAD_REQUEST, "CIRCULAR_REFERENCE"),
            FolderApiError::NameAlreadyExists => (StatusCode::CONFLICT, "NAME_ALREADY_EXISTS"),
            FolderApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            FolderApiError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        };

        let body = ApiError::new(self.to_string(), code);

        (status, Json(body)).into_response()
    }
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request for creating a new folder
#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
}

/// Request for updating a folder
#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    pub name: String,
}

/// Request for moving a folder
#[derive(Debug, Deserialize)]
pub struct MoveFolderRequest {
    /// New parent folder ID (None = move to root)
    pub parent_id: Option<Uuid>,
}

/// Query parameters for listing folders
#[derive(Debug, Deserialize, Default)]
pub struct ListFoldersQuery {
    /// Filter by parent (None = root level only)
    pub parent_id: Option<Uuid>,
    /// Include all folders (flat list, ignore parent_id)
    #[serde(default)]
    pub all: bool,
}

/// Response for a single folder
#[derive(Debug, Serialize)]
pub struct FolderResponse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Folder> for FolderResponse {
    fn from(folder: Folder) -> Self {
        Self {
            id: folder.id,
            owner_id: folder.owner_id,
            parent_id: folder.parent_id,
            name: folder.name,
            created_at: folder.created_at,
            updated_at: folder.updated_at,
        }
    }
}

/// Response for folder list
#[derive(Debug, Serialize)]
pub struct FolderListResponse {
    pub folders: Vec<FolderResponse>,
    pub count: usize,
}

/// Response for folder tree
#[derive(Debug, Serialize)]
pub struct FolderTreeResponse {
    pub folders: Vec<FolderWithDepth>,
    pub count: usize,
}

/// Response for folder nodes with counts
#[derive(Debug, Serialize)]
pub struct FolderNodesResponse {
    pub folders: Vec<FolderNode>,
    pub count: usize,
}

/// Response for folder path (breadcrumb)
#[derive(Debug, Serialize)]
pub struct FolderPathResponse {
    pub path: Vec<FolderResponse>,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Response for delete operation
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
    pub id: Uuid,
}

// ============================================================================
// Router
// ============================================================================

/// Create the folder API router
pub fn folder_api_router(state: FolderApiState) -> Router {
    let state = Arc::new(state);

    Router::new()
        .route("/api/folders", post(create_folder_handler))
        .route("/api/folders", get(list_folders_handler))
        .route("/api/folders/tree", get(get_folder_tree_handler))
        .route("/api/folders/nodes", get(get_folder_nodes_handler))
        .route("/api/folders/{id}", get(get_folder_handler))
        .route("/api/folders/{id}", put(update_folder_handler))
        .route("/api/folders/{id}", delete(delete_folder_handler))
        .route("/api/folders/{id}/path", get(get_folder_path_handler))
        .route(
            "/api/folders/{id}/children",
            get(get_folder_children_handler),
        )
        .route("/api/folders/{id}/move", patch(move_folder_handler))
        .with_state(state)
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/folders
/// Create a new folder (auth required)
async fn create_folder_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Json(request): Json<CreateFolderRequest>,
) -> Result<(StatusCode, Json<FolderResponse>), FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!("Creating folder '{}' for user {}", request.name, user_id);

    // Validate name
    let name = request.name.trim();
    if name.is_empty() {
        return Err(FolderApiError::BadRequest(
            "Folder name cannot be empty".to_string(),
        ));
    }
    if name.len() > 255 {
        return Err(FolderApiError::BadRequest(
            "Folder name too long (max 255 characters)".to_string(),
        ));
    }

    // Create folder
    let create_dto = CreateFolder {
        owner_id: user_id,
        parent_id: request.parent_id,
        name: name.to_string(),
    };

    let folder = state.folder_repo.create(&create_dto).await?;

    tracing::info!("Folder created: {}", folder.id);

    Ok((StatusCode::CREATED, Json(folder.into())))
}

/// GET /api/folders
/// List user's folders
async fn list_folders_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Query(query): Query<ListFoldersQuery>,
) -> Result<Json<FolderListResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!(
        "Listing folders for user {}, parent: {:?}, all: {}",
        user_id,
        query.parent_id,
        query.all
    );

    let folders = if query.all {
        // Get all folders (flat list)
        state.folder_repo.list_all_by_owner(user_id).await?
    } else if let Some(parent_id) = query.parent_id {
        // Get children of specific folder
        state.folder_repo.list_children(parent_id, user_id).await?
    } else {
        // Get root folders
        state.folder_repo.list_root_folders(user_id).await?
    };

    let count = folders.len();
    let folders: Vec<FolderResponse> = folders.into_iter().map(Into::into).collect();

    Ok(Json(FolderListResponse { folders, count }))
}

/// GET /api/folders/tree
/// Get folder tree structure
async fn get_folder_tree_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
) -> Result<Json<FolderTreeResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!("Getting folder tree for user {}", user_id);

    let folders = state.folder_repo.get_folder_tree(user_id).await?;
    let count = folders.len();

    Ok(Json(FolderTreeResponse { folders, count }))
}

/// GET /api/folders/nodes
/// Get folder nodes with children and diagrams count
async fn get_folder_nodes_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Query(query): Query<ListFoldersQuery>,
) -> Result<Json<FolderNodesResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!(
        "Getting folder nodes for user {}, parent: {:?}",
        user_id,
        query.parent_id
    );

    let folders = state
        .folder_repo
        .get_folder_nodes(user_id, query.parent_id)
        .await?;
    let count = folders.len();

    Ok(Json(FolderNodesResponse { folders, count }))
}

/// GET /api/folders/:id
/// Get a folder by ID
async fn get_folder_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<FolderResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!("Getting folder {}, user: {}", id, user_id);

    let folder = state
        .folder_repo
        .find_by_id_and_owner(id, user_id)
        .await?
        .ok_or(FolderApiError::NotFound)?;

    Ok(Json(folder.into()))
}

/// GET /api/folders/:id/path
/// Get folder path (breadcrumb)
async fn get_folder_path_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<FolderPathResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!("Getting path for folder {}, user: {}", id, user_id);

    // Verify folder belongs to user
    let folder = state
        .folder_repo
        .find_by_id_and_owner(id, user_id)
        .await?
        .ok_or(FolderApiError::NotFound)?;

    let path = state.folder_repo.get_path(folder.id).await?;
    let path: Vec<FolderResponse> = path.into_iter().map(Into::into).collect();

    Ok(Json(FolderPathResponse { path }))
}

/// GET /api/folders/:id/children
/// Get folder children
async fn get_folder_children_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<FolderListResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!("Getting children for folder {}, user: {}", id, user_id);

    // Verify folder belongs to user
    state
        .folder_repo
        .find_by_id_and_owner(id, user_id)
        .await?
        .ok_or(FolderApiError::NotFound)?;

    let folders = state.folder_repo.list_children(id, user_id).await?;
    let count = folders.len();
    let folders: Vec<FolderResponse> = folders.into_iter().map(Into::into).collect();

    Ok(Json(FolderListResponse { folders, count }))
}

/// PUT /api/folders/:id
/// Update a folder (rename)
async fn update_folder_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateFolderRequest>,
) -> Result<Json<FolderResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!("Updating folder {} by user {}", id, user_id);

    // Validate name
    let name = request.name.trim();
    if name.is_empty() {
        return Err(FolderApiError::BadRequest(
            "Folder name cannot be empty".to_string(),
        ));
    }
    if name.len() > 255 {
        return Err(FolderApiError::BadRequest(
            "Folder name too long (max 255 characters)".to_string(),
        ));
    }

    let folder = state.folder_repo.rename(id, user_id, name).await?;

    tracing::info!("Folder renamed: {}", folder.id);

    Ok(Json(folder.into()))
}

/// PATCH /api/folders/:id/move
/// Move a folder to a new parent
async fn move_folder_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<MoveFolderRequest>,
) -> Result<Json<FolderResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!(
        "Moving folder {} to parent {:?} by user {}",
        id,
        request.parent_id,
        user_id
    );

    let folder = state
        .folder_repo
        .move_to_parent(id, user_id, request.parent_id)
        .await?;

    tracing::info!("Folder moved: {}", folder.id);

    Ok(Json(folder.into()))
}

/// DELETE /api/folders/:id
/// Delete a folder
async fn delete_folder_handler(
    State(state): State<Arc<FolderApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, FolderApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!("Deleting folder {} by user {}", id, user_id);

    let deleted = state.folder_repo.delete(id, user_id).await?;

    if deleted {
        tracing::info!("Folder deleted: {}", id);
    }

    Ok(Json(DeleteResponse { deleted, id }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract user ID from JWT token in Authorization header
fn extract_user_id(jwt_service: &JwtService, headers: &HeaderMap) -> Result<Uuid, FolderApiError> {
    let token = extract_bearer_token(headers)?;

    let claims = jwt_service
        .validate_access_token(&token)
        .map_err(|e| match e {
            crate::core::auth::JwtError::Expired => FolderApiError::TokenExpired,
            _ => FolderApiError::InvalidToken,
        })?;

    claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| FolderApiError::InvalidToken)
}

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Result<String, FolderApiError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(FolderApiError::Unauthorized)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(FolderApiError::InvalidToken);
    }

    let token = auth_header.trim_start_matches("Bearer ").to_string();

    if token.is_empty() {
        return Err(FolderApiError::InvalidToken);
    }

    Ok(token)
}

// ============================================================================
// Tests
// ============================================================================

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
        assert!(matches!(result, Err(FolderApiError::Unauthorized)));
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic base64credentials"),
        );

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(FolderApiError::InvalidToken)));
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(FolderApiError::InvalidToken)));
    }

    #[test]
    fn test_api_error_serialization() {
        let error = ApiError::new("Something went wrong", "ERROR_CODE");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("Something went wrong"));
        assert!(json.contains("ERROR_CODE"));
    }

    #[test]
    fn test_create_folder_request_deserialization() {
        let json = r#"{
            "name": "My Folder",
            "parent_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let request: CreateFolderRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "My Folder");
        assert!(request.parent_id.is_some());
    }

    #[test]
    fn test_create_folder_request_minimal() {
        let json = r#"{"name": "Simple"}"#;

        let request: CreateFolderRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "Simple");
        assert!(request.parent_id.is_none());
    }

    #[test]
    fn test_update_folder_request_deserialization() {
        let json = r#"{"name": "Updated Name"}"#;

        let request: UpdateFolderRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "Updated Name");
    }

    #[test]
    fn test_move_folder_request_to_root() {
        let json = r#"{"parent_id": null}"#;

        let request: MoveFolderRequest = serde_json::from_str(json).unwrap();

        assert!(request.parent_id.is_none());
    }

    #[test]
    fn test_move_folder_request_to_parent() {
        let json = r#"{"parent_id": "550e8400-e29b-41d4-a716-446655440000"}"#;

        let request: MoveFolderRequest = serde_json::from_str(json).unwrap();

        assert!(request.parent_id.is_some());
    }

    #[test]
    fn test_list_folders_query_defaults() {
        let query: ListFoldersQuery = serde_json::from_str("{}").unwrap();

        assert!(query.parent_id.is_none());
        assert!(!query.all);
    }

    #[test]
    fn test_list_folders_query_custom() {
        let json = r#"{
            "parent_id": "550e8400-e29b-41d4-a716-446655440000",
            "all": true
        }"#;

        let query: ListFoldersQuery = serde_json::from_str(json).unwrap();

        assert!(query.parent_id.is_some());
        assert!(query.all);
    }

    #[test]
    fn test_folder_response_serialization() {
        use chrono::Utc;

        let response = FolderResponse {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            parent_id: None,
            name: "Test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("Test"));
        assert!(json.contains("id"));
    }

    #[test]
    fn test_folder_list_response_serialization() {
        let response = FolderListResponse {
            folders: vec![],
            count: 0,
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("folders"));
        assert!(json.contains("count"));
    }

    #[test]
    fn test_folder_tree_response_serialization() {
        let response = FolderTreeResponse {
            folders: vec![],
            count: 0,
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("folders"));
        assert!(json.contains("count"));
    }

    #[test]
    fn test_folder_path_response_serialization() {
        let response = FolderPathResponse { path: vec![] };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("path"));
    }

    #[test]
    fn test_success_response_serialization() {
        let response = SuccessResponse {
            success: true,
            message: "Folder created".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("true"));
        assert!(json.contains("Folder created"));
    }

    #[test]
    fn test_delete_response_serialization() {
        let response = DeleteResponse {
            deleted: true,
            id: Uuid::nil(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("deleted"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_folder_api_error_display() {
        assert_eq!(FolderApiError::NotFound.to_string(), "Folder not found");
        assert_eq!(FolderApiError::AccessDenied.to_string(), "Access denied");
        assert_eq!(
            FolderApiError::Unauthorized.to_string(),
            "Authentication required"
        );
        assert_eq!(FolderApiError::InvalidToken.to_string(), "Invalid token");
        assert_eq!(FolderApiError::TokenExpired.to_string(), "Token expired");
        assert_eq!(
            FolderApiError::ParentNotFound.to_string(),
            "Parent folder not found"
        );
        assert_eq!(
            FolderApiError::CircularReference.to_string(),
            "Cannot move folder into itself or its descendants"
        );
        assert_eq!(
            FolderApiError::NameAlreadyExists.to_string(),
            "Folder name already exists in this location"
        );
        assert_eq!(
            FolderApiError::BadRequest("test".to_string()).to_string(),
            "Invalid request: test"
        );
        assert_eq!(
            FolderApiError::InternalError("db".to_string()).to_string(),
            "Internal error: db"
        );
    }

    #[test]
    fn test_folder_repository_error_conversion() {
        let err: FolderApiError = FolderRepositoryError::NotFound.into();
        assert!(matches!(err, FolderApiError::NotFound));

        let err: FolderApiError = FolderRepositoryError::AccessDenied.into();
        assert!(matches!(err, FolderApiError::AccessDenied));

        let err: FolderApiError = FolderRepositoryError::ParentNotFound.into();
        assert!(matches!(err, FolderApiError::ParentNotFound));

        let err: FolderApiError = FolderRepositoryError::CircularReference.into();
        assert!(matches!(err, FolderApiError::CircularReference));

        let err: FolderApiError = FolderRepositoryError::NameAlreadyExists.into();
        assert!(matches!(err, FolderApiError::NameAlreadyExists));
    }
}
