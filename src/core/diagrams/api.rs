//! Diagram API endpoints
//!
//! Provides REST API endpoints for diagram management:
//! - POST /api/diagrams - Create a new diagram (auth required)
//! - GET /api/diagrams - List user's diagrams
//! - GET /api/diagrams/:id - Get diagram by ID
//! - PUT /api/diagrams/:id - Update diagram
//! - DELETE /api/diagrams/:id - Delete diagram
//! - PATCH /api/diagrams/:id/autosave - Autosave schema data

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

use crate::core::db::models::double_option;

use crate::core::auth::JwtService;
use crate::core::db::models::{
    CreateDiagram, Diagram, DiagramSummary, SharePermission, UpdateDiagram,
};
use crate::core::db::repositories::{DiagramAccess, DiagramRepository, DiagramRepositoryError};

/// Diagram API state containing the diagram repository and JWT service
#[derive(Clone)]
pub struct DiagramApiState {
    pub diagram_repo: DiagramRepository,
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

/// Diagram API error types
#[derive(Debug, thiserror::Error)]
pub enum DiagramApiError {
    #[error("Diagram not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("Authentication required")]
    Unauthorized,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Folder not found")]
    FolderNotFound,

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<DiagramRepositoryError> for DiagramApiError {
    fn from(err: DiagramRepositoryError) -> Self {
        match err {
            DiagramRepositoryError::NotFound => DiagramApiError::NotFound,
            DiagramRepositoryError::AccessDenied => DiagramApiError::AccessDenied,
            DiagramRepositoryError::FolderNotFound => DiagramApiError::FolderNotFound,
            DiagramRepositoryError::DatabaseError(e) => {
                DiagramApiError::InternalError(e.to_string())
            }
        }
    }
}

impl IntoResponse for DiagramApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            DiagramApiError::NotFound => (StatusCode::NOT_FOUND, "DIAGRAM_NOT_FOUND"),
            DiagramApiError::AccessDenied => (StatusCode::FORBIDDEN, "ACCESS_DENIED"),
            DiagramApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            DiagramApiError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN"),
            DiagramApiError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED"),
            DiagramApiError::FolderNotFound => (StatusCode::BAD_REQUEST, "FOLDER_NOT_FOUND"),
            DiagramApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            DiagramApiError::InternalError(_) => {
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

/// Request for creating a new diagram
#[derive(Debug, Deserialize)]
pub struct CreateDiagramRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    #[serde(default = "default_schema_data")]
    pub schema_data: serde_json::Value,
    #[serde(default)]
    pub is_public: bool,
}

fn default_schema_data() -> serde_json::Value {
    serde_json::json!({})
}

/// Request for updating a diagram
#[derive(Debug, Deserialize)]
pub struct UpdateDiagramRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub folder_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub schema_data: Option<serde_json::Value>,
    #[serde(default)]
    pub is_public: Option<bool>,
}

/// Request for autosaving diagram schema
#[derive(Debug, Deserialize)]
pub struct AutosaveRequest {
    pub schema_data: serde_json::Value,
}

/// Query parameters for listing diagrams
#[derive(Debug, Deserialize, Default)]
pub struct ListDiagramsQuery {
    /// Filter by folder (None = root level)
    pub folder_id: Option<Uuid>,
    /// Include diagrams shared with user
    #[serde(default)]
    pub include_shared: bool,
    /// Limit results (default 50, max 100)
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// Response for a single diagram
#[derive(Debug, Serialize)]
pub struct DiagramResponse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub schema_data: serde_json::Value,
    pub is_public: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub access: String,
}

impl DiagramResponse {
    pub fn from_diagram_with_access(diagram: Diagram, access: DiagramAccess) -> Self {
        Self {
            id: diagram.id,
            owner_id: diagram.owner_id,
            folder_id: diagram.folder_id,
            name: diagram.name,
            description: diagram.description,
            schema_data: diagram.schema_data.0,
            is_public: diagram.is_public,
            created_at: diagram.created_at,
            updated_at: diagram.updated_at,
            access: access_to_string(access),
        }
    }
}

fn access_to_string(access: DiagramAccess) -> String {
    match access {
        DiagramAccess::Owner => "owner".to_string(),
        DiagramAccess::Editor => "editor".to_string(),
        DiagramAccess::Viewer => "viewer".to_string(),
        DiagramAccess::Public => "public".to_string(),
        DiagramAccess::None => "none".to_string(),
    }
}

/// Response for diagram list
#[derive(Debug, Serialize)]
pub struct DiagramListResponse {
    pub diagrams: Vec<DiagramSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Shared diagram info for responses
#[derive(Debug, Serialize)]
pub struct SharedDiagramResponse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub permission: String,
}

impl SharedDiagramResponse {
    pub fn from_summary_with_permission(
        summary: DiagramSummary,
        permission: SharePermission,
    ) -> Self {
        Self {
            id: summary.id,
            owner_id: summary.owner_id,
            folder_id: summary.folder_id,
            name: summary.name,
            description: summary.description,
            is_public: summary.is_public,
            created_at: summary.created_at,
            updated_at: summary.updated_at,
            permission: permission.to_string(),
        }
    }
}

/// Response for shared diagrams list
#[derive(Debug, Serialize)]
pub struct SharedDiagramListResponse {
    pub diagrams: Vec<SharedDiagramResponse>,
    pub limit: i64,
    pub offset: i64,
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

/// Create the diagram API router
pub fn diagram_api_router(state: DiagramApiState) -> Router {
    let state = Arc::new(state);

    Router::new()
        .route("/api/diagrams", post(create_diagram_handler))
        .route("/api/diagrams", get(list_diagrams_handler))
        .route("/api/diagrams/shared", get(list_shared_diagrams_handler))
        .route("/api/diagrams/{id}", get(get_diagram_handler))
        .route("/api/diagrams/{id}", put(update_diagram_handler))
        .route("/api/diagrams/{id}", delete(delete_diagram_handler))
        .route(
            "/api/diagrams/{id}/autosave",
            patch(autosave_diagram_handler),
        )
        .with_state(state)
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/diagrams
/// Create a new diagram (auth required)
async fn create_diagram_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Json(request): Json<CreateDiagramRequest>,
) -> Result<(StatusCode, Json<DiagramResponse>), DiagramApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!("Creating diagram '{}' for user {}", request.name, user_id);

    // Validate name
    let name = request.name.trim();
    if name.is_empty() {
        return Err(DiagramApiError::BadRequest(
            "Diagram name cannot be empty".to_string(),
        ));
    }
    if name.len() > 255 {
        return Err(DiagramApiError::BadRequest(
            "Diagram name too long (max 255 characters)".to_string(),
        ));
    }

    // Create diagram
    let create_dto = CreateDiagram {
        owner_id: user_id,
        folder_id: request.folder_id,
        name: name.to_string(),
        description: request.description,
        schema_data: request.schema_data,
        is_public: request.is_public,
    };

    let diagram = state.diagram_repo.create(&create_dto).await?;

    tracing::info!("Diagram created: {}", diagram.id);

    let response = DiagramResponse::from_diagram_with_access(diagram, DiagramAccess::Owner);

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/diagrams
/// List user's diagrams
async fn list_diagrams_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Query(query): Query<ListDiagramsQuery>,
) -> Result<Json<DiagramListResponse>, DiagramApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    let limit = query.limit.min(100).max(1);
    let offset = query.offset.max(0);

    tracing::debug!(
        "Listing diagrams for user {}, folder: {:?}, limit: {}, offset: {}",
        user_id,
        query.folder_id,
        limit,
        offset
    );

    // Get diagrams
    let diagrams = state
        .diagram_repo
        .list_by_owner(user_id, query.folder_id, limit, offset)
        .await?;

    // Get total count
    let total = if query.folder_id.is_some() {
        state
            .diagram_repo
            .count_in_folder(user_id, query.folder_id)
            .await?
    } else {
        state.diagram_repo.count_by_owner(user_id).await?
    };

    Ok(Json(DiagramListResponse {
        diagrams,
        total,
        limit,
        offset,
    }))
}

/// GET /api/diagrams/shared
/// List diagrams shared with the user
async fn list_shared_diagrams_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Query(query): Query<ListDiagramsQuery>,
) -> Result<Json<SharedDiagramListResponse>, DiagramApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    let limit = query.limit.min(100).max(1);
    let offset = query.offset.max(0);

    tracing::debug!(
        "Listing shared diagrams for user {}, limit: {}, offset: {}",
        user_id,
        limit,
        offset
    );

    let shared_diagrams = state
        .diagram_repo
        .list_shared_with(user_id, limit, offset)
        .await?;

    let diagrams: Vec<SharedDiagramResponse> = shared_diagrams
        .into_iter()
        .map(|(summary, permission)| {
            SharedDiagramResponse::from_summary_with_permission(summary, permission)
        })
        .collect();

    Ok(Json(SharedDiagramListResponse {
        diagrams,
        limit,
        offset,
    }))
}

/// GET /api/diagrams/:id
/// Get a diagram by ID (respects permissions)
async fn get_diagram_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<DiagramResponse>, DiagramApiError> {
    // Try to get user ID (optional - anonymous can view public diagrams)
    let user_id = extract_user_id_optional(&state.jwt_service, &headers);

    tracing::debug!("Getting diagram {}, user: {:?}", id, user_id);

    // Get diagram with access check
    let (diagram, access) = state
        .diagram_repo
        .find_by_id_with_access(id, user_id)
        .await?
        .ok_or(DiagramApiError::NotFound)?;

    Ok(Json(DiagramResponse::from_diagram_with_access(
        diagram, access,
    )))
}

/// PUT /api/diagrams/:id
/// Update a diagram (requires edit permission)
async fn update_diagram_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateDiagramRequest>,
) -> Result<Json<DiagramResponse>, DiagramApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!("Updating diagram {} by user {}", id, user_id);

    // Check access
    let access = state
        .diagram_repo
        .get_access_level(id, Some(user_id))
        .await?;
    if !access.can_edit() {
        return Err(DiagramApiError::AccessDenied);
    }

    // Validate name if provided
    if let Some(ref name) = request.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(DiagramApiError::BadRequest(
                "Diagram name cannot be empty".to_string(),
            ));
        }
        if name.len() > 255 {
            return Err(DiagramApiError::BadRequest(
                "Diagram name too long (max 255 characters)".to_string(),
            ));
        }
    }

    // Build update DTO
    let update_dto = UpdateDiagram {
        folder_id: request.folder_id.clone(),
        name: request.name.map(|n| n.trim().to_string()),
        description: request.description,
        schema_data: request.schema_data,
        is_public: request.is_public,
    };

    let diagram = state.diagram_repo.update(id, &update_dto).await?;

    tracing::info!("Diagram updated: {}", diagram.id);

    Ok(Json(DiagramResponse::from_diagram_with_access(
        diagram, access,
    )))
}

/// DELETE /api/diagrams/:id
/// Delete a diagram (owner only)
async fn delete_diagram_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, DiagramApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::info!("Deleting diagram {} by user {}", id, user_id);

    // Check access - only owner can delete
    let access = state
        .diagram_repo
        .get_access_level(id, Some(user_id))
        .await?;
    if !access.can_delete() {
        return Err(DiagramApiError::AccessDenied);
    }

    // Delete diagram
    let deleted = state.diagram_repo.delete_by_owner(id, user_id).await?;

    if deleted {
        tracing::info!("Diagram deleted: {}", id);
    }

    Ok(Json(DeleteResponse { deleted, id }))
}

/// PATCH /api/diagrams/:id/autosave
/// Autosave diagram schema data (requires edit permission)
async fn autosave_diagram_handler(
    State(state): State<Arc<DiagramApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<AutosaveRequest>,
) -> Result<Json<SuccessResponse>, DiagramApiError> {
    // Require authentication
    let user_id = extract_user_id(&state.jwt_service, &headers)?;

    tracing::debug!("Autosaving diagram {} by user {}", id, user_id);

    // Check access
    let access = state
        .diagram_repo
        .get_access_level(id, Some(user_id))
        .await?;
    if !access.can_edit() {
        return Err(DiagramApiError::AccessDenied);
    }

    // Update schema data
    state
        .diagram_repo
        .update_schema(id, &request.schema_data)
        .await?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Diagram saved".to_string(),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract user ID from JWT token in Authorization header
fn extract_user_id(jwt_service: &JwtService, headers: &HeaderMap) -> Result<Uuid, DiagramApiError> {
    let token = extract_bearer_token(headers)?;

    let claims = jwt_service
        .validate_access_token(&token)
        .map_err(|e| match e {
            crate::core::auth::JwtError::Expired => DiagramApiError::TokenExpired,
            _ => DiagramApiError::InvalidToken,
        })?;

    claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| DiagramApiError::InvalidToken)
}

/// Extract user ID from JWT token, returning None if not authenticated
fn extract_user_id_optional(jwt_service: &JwtService, headers: &HeaderMap) -> Option<Uuid> {
    extract_user_id(jwt_service, headers).ok()
}

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Result<String, DiagramApiError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(DiagramApiError::Unauthorized)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(DiagramApiError::InvalidToken);
    }

    let token = auth_header.trim_start_matches("Bearer ").to_string();

    if token.is_empty() {
        return Err(DiagramApiError::InvalidToken);
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
        assert!(matches!(result, Err(DiagramApiError::Unauthorized)));
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic base64credentials"),
        );

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(DiagramApiError::InvalidToken)));
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(DiagramApiError::InvalidToken)));
    }

    #[test]
    fn test_api_error_serialization() {
        let error = ApiError::new("Something went wrong", "ERROR_CODE");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("Something went wrong"));
        assert!(json.contains("ERROR_CODE"));
    }

    #[test]
    fn test_create_diagram_request_deserialization() {
        let json = r#"{
            "name": "My Diagram",
            "description": "A test diagram",
            "is_public": true
        }"#;

        let request: CreateDiagramRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "My Diagram");
        assert_eq!(request.description, Some("A test diagram".to_string()));
        assert!(request.is_public);
        assert!(request.folder_id.is_none());
        assert_eq!(request.schema_data, serde_json::json!({}));
    }

    #[test]
    fn test_create_diagram_request_minimal() {
        let json = r#"{"name": "Simple"}"#;

        let request: CreateDiagramRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "Simple");
        assert!(request.description.is_none());
        assert!(!request.is_public);
        assert!(request.folder_id.is_none());
    }

    #[test]
    fn test_create_diagram_request_with_schema() {
        let json = r#"{
            "name": "With Schema",
            "schema_data": {"tables": [], "relations": []}
        }"#;

        let request: CreateDiagramRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "With Schema");
        assert_eq!(
            request.schema_data,
            serde_json::json!({"tables": [], "relations": []})
        );
    }

    #[test]
    fn test_update_diagram_request_deserialization() {
        let json = r#"{
            "name": "Updated Name",
            "is_public": false
        }"#;

        let request: UpdateDiagramRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.is_public, Some(false));
        assert!(request.description.is_none());
        assert!(request.folder_id.is_none());
        assert!(request.schema_data.is_none());
    }

    #[test]
    fn test_autosave_request_deserialization() {
        let json = r#"{"schema_data": {"tables": [{"name": "users"}]}}"#;

        let request: AutosaveRequest = serde_json::from_str(json).unwrap();

        assert!(request.schema_data.get("tables").is_some());
    }

    #[test]
    fn test_list_diagrams_query_defaults() {
        let query: ListDiagramsQuery = serde_json::from_str("{}").unwrap();

        assert!(query.folder_id.is_none());
        assert!(!query.include_shared);
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
    }

    #[test]
    fn test_list_diagrams_query_custom() {
        let json = r#"{
            "folder_id": "550e8400-e29b-41d4-a716-446655440000",
            "include_shared": true,
            "limit": 20,
            "offset": 10
        }"#;

        let query: ListDiagramsQuery = serde_json::from_str(json).unwrap();

        assert!(query.folder_id.is_some());
        assert!(query.include_shared);
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 10);
    }

    #[test]
    fn test_access_to_string() {
        assert_eq!(access_to_string(DiagramAccess::Owner), "owner");
        assert_eq!(access_to_string(DiagramAccess::Editor), "editor");
        assert_eq!(access_to_string(DiagramAccess::Viewer), "viewer");
        assert_eq!(access_to_string(DiagramAccess::Public), "public");
        assert_eq!(access_to_string(DiagramAccess::None), "none");
    }

    #[test]
    fn test_diagram_response_serialization() {
        use chrono::Utc;

        let response = DiagramResponse {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            folder_id: None,
            name: "Test".to_string(),
            description: Some("Description".to_string()),
            schema_data: serde_json::json!({}),
            is_public: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            access: "owner".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("Test"));
        assert!(json.contains("owner"));
    }

    #[test]
    fn test_diagram_list_response_serialization() {
        let response = DiagramListResponse {
            diagrams: vec![],
            total: 0,
            limit: 50,
            offset: 0,
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("diagrams"));
        assert!(json.contains("total"));
        assert!(json.contains("limit"));
        assert!(json.contains("offset"));
    }

    #[test]
    fn test_success_response_serialization() {
        let response = SuccessResponse {
            success: true,
            message: "Diagram saved".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("true"));
        assert!(json.contains("Diagram saved"));
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
    fn test_diagram_api_error_display() {
        assert_eq!(DiagramApiError::NotFound.to_string(), "Diagram not found");
        assert_eq!(DiagramApiError::AccessDenied.to_string(), "Access denied");
        assert_eq!(
            DiagramApiError::Unauthorized.to_string(),
            "Authentication required"
        );
        assert_eq!(DiagramApiError::InvalidToken.to_string(), "Invalid token");
        assert_eq!(DiagramApiError::TokenExpired.to_string(), "Token expired");
        assert_eq!(
            DiagramApiError::FolderNotFound.to_string(),
            "Folder not found"
        );
        assert_eq!(
            DiagramApiError::BadRequest("test".to_string()).to_string(),
            "Invalid request: test"
        );
        assert_eq!(
            DiagramApiError::InternalError("db".to_string()).to_string(),
            "Internal error: db"
        );
    }

    #[test]
    fn test_diagram_repository_error_conversion() {
        let err: DiagramApiError = DiagramRepositoryError::NotFound.into();
        assert!(matches!(err, DiagramApiError::NotFound));

        let err: DiagramApiError = DiagramRepositoryError::AccessDenied.into();
        assert!(matches!(err, DiagramApiError::AccessDenied));

        let err: DiagramApiError = DiagramRepositoryError::FolderNotFound.into();
        assert!(matches!(err, DiagramApiError::FolderNotFound));
    }
}
