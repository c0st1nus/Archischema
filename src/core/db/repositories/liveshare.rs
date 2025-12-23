//! LiveShare repository for managing collaborative editing sessions
//!
//! Provides database operations for:
//! - Creating and managing LiveShare sessions
//! - Checking access permissions
//! - Managing session participants
//! - Saving and loading Yjs state snapshots

use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::fmt;
use uuid::Uuid;

use crate::core::db::models::{
    CreateLiveShareParticipant, CreateLiveShareSession, LiveShareParticipant, LiveShareSession,
    UpdateLiveShareSession,
};

/// Errors that can occur in LiveShare operations
#[derive(Debug)]
pub enum LiveShareRepositoryError {
    /// Session not found
    NotFound,
    /// Diagram not found
    DiagramNotFound,
    /// User not found
    UserNotFound,
    /// Access denied to diagram or session
    AccessDenied,
    /// Session is full (max users reached)
    SessionFull,
    /// Session is not active
    SessionInactive,
    /// Active session already exists for this diagram
    ActiveSessionExists,
    /// Database error
    DatabaseError(sqlx::Error),
}

impl fmt::Display for LiveShareRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "LiveShare session not found"),
            Self::DiagramNotFound => write!(f, "Diagram not found"),
            Self::UserNotFound => write!(f, "User not found"),
            Self::AccessDenied => write!(f, "Access denied"),
            Self::SessionFull => write!(f, "Session is full"),
            Self::SessionInactive => write!(f, "Session is not active"),
            Self::ActiveSessionExists => {
                write!(f, "Active session already exists for this diagram")
            }
            Self::DatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for LiveShareRepositoryError {}

impl From<sqlx::Error> for LiveShareRepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            _ => Self::DatabaseError(err),
        }
    }
}

/// Repository for LiveShare operations
pub struct LiveShareRepository {
    pool: PgPool,
}

impl LiveShareRepository {
    /// Create a new LiveShare repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new LiveShare session
    ///
    /// Checks that:
    /// - Diagram exists
    /// - User has edit access to the diagram (owner or shared with edit permission)
    /// - No active session exists for this diagram (enforced by DB constraint)
    pub async fn create_session(
        &self,
        data: CreateLiveShareSession,
    ) -> Result<LiveShareSession, LiveShareRepositoryError> {
        // Check if diagram exists
        let diagram_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM diagrams WHERE id = $1)")
                .bind(data.diagram_id)
                .fetch_one(&self.pool)
                .await?;

        if !diagram_exists {
            return Err(LiveShareRepositoryError::DiagramNotFound);
        }

        // Check if user has edit access (owner or shared with edit permission)
        let has_access = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM diagrams WHERE id = $1 AND owner_id = $2
                UNION
                SELECT 1 FROM diagram_shares
                WHERE diagram_id = $1 AND user_id = $2 AND permission = 'edit'
            )
            "#,
        )
        .bind(data.diagram_id)
        .bind(data.owner_id)
        .fetch_one(&self.pool)
        .await?;

        if !has_access {
            return Err(LiveShareRepositoryError::AccessDenied);
        }

        // Create session
        let session = sqlx::query_as::<_, LiveShareSession>(
            r#"
            INSERT INTO liveshare_sessions (diagram_id, owner_id, name, password_hash, max_users)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.diagram_id)
        .bind(data.owner_id)
        .bind(data.name)
        .bind(data.password_hash)
        .bind(data.max_users)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("idx_liveshare_sessions_diagram_active") =>
            {
                LiveShareRepositoryError::ActiveSessionExists
            }
            _ => LiveShareRepositoryError::from(e),
        })?;

        Ok(session)
    }

    /// Get active session for a diagram
    pub async fn get_active_session_for_diagram(
        &self,
        diagram_id: Uuid,
    ) -> Result<Option<LiveShareSession>, LiveShareRepositoryError> {
        let session = sqlx::query_as::<_, LiveShareSession>(
            r#"
            SELECT * FROM liveshare_sessions
            WHERE diagram_id = $1 AND is_active = TRUE AND ended_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(diagram_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<LiveShareSession, LiveShareRepositoryError> {
        let session =
            sqlx::query_as::<_, LiveShareSession>("SELECT * FROM liveshare_sessions WHERE id = $1")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(session)
    }

    /// Check if user has access to a session
    ///
    /// User has access if:
    /// - They own the diagram
    /// - They have edit permission on the diagram
    /// - They have view permission on the diagram (read-only access)
    pub async fn check_session_access(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, LiveShareRepositoryError> {
        let has_access = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM liveshare_sessions ls
                JOIN diagrams d ON d.id = ls.diagram_id
                WHERE ls.id = $1 AND d.owner_id = $2
                UNION
                SELECT 1 FROM liveshare_sessions ls
                JOIN diagram_shares ds ON ds.diagram_id = ls.diagram_id
                WHERE ls.id = $1 AND ds.user_id = $2
            )
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(has_access)
    }

    /// Check if user has edit access to a session
    ///
    /// User has edit access if:
    /// - They own the diagram
    /// - They have edit permission on the diagram
    pub async fn check_session_edit_access(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, LiveShareRepositoryError> {
        let has_edit_access = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM liveshare_sessions ls
                JOIN diagrams d ON d.id = ls.diagram_id
                WHERE ls.id = $1 AND d.owner_id = $2
                UNION
                SELECT 1 FROM liveshare_sessions ls
                JOIN diagram_shares ds ON ds.diagram_id = ls.diagram_id
                WHERE ls.id = $1 AND ds.user_id = $2 AND ds.permission = 'edit'
            )
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(has_edit_access)
    }

    /// Save Yjs state snapshot for a session
    pub async fn save_snapshot(
        &self,
        session_id: Uuid,
        yjs_state: Vec<u8>,
    ) -> Result<(), LiveShareRepositoryError> {
        sqlx::query(
            r#"
            UPDATE liveshare_sessions
            SET yjs_state = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(yjs_state)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update session configuration
    pub async fn update_session(
        &self,
        session_id: Uuid,
        data: UpdateLiveShareSession,
    ) -> Result<LiveShareSession, LiveShareRepositoryError> {
        let updated_session = sqlx::query_as::<_, LiveShareSession>(
            r#"
            UPDATE liveshare_sessions
            SET name = COALESCE($1, name),
                password_hash = CASE
                    WHEN $2::boolean THEN $3
                    ELSE password_hash
                END,
                max_users = COALESCE($4, max_users),
                is_active = COALESCE($5, is_active),
                yjs_state = COALESCE($6, yjs_state),
                ended_at = CASE
                    WHEN $7::boolean THEN $8
                    ELSE ended_at
                END,
                updated_at = NOW()
            WHERE id = $9
            RETURNING *
            "#,
        )
        .bind(data.name)
        .bind(data.password_hash.is_some())
        .bind(data.password_hash.flatten())
        .bind(data.max_users)
        .bind(data.is_active)
        .bind(data.yjs_state)
        .bind(data.ended_at.is_some())
        .bind(data.ended_at.flatten())
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated_session)
    }

    /// End a session (set ended_at and is_active = false)
    pub async fn end_session(
        &self,
        session_id: Uuid,
    ) -> Result<LiveShareSession, LiveShareRepositoryError> {
        let session = sqlx::query_as::<_, LiveShareSession>(
            r#"
            UPDATE liveshare_sessions
            SET is_active = FALSE, ended_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Delete a session (hard delete)
    pub async fn delete_session(&self, session_id: Uuid) -> Result<(), LiveShareRepositoryError> {
        sqlx::query("DELETE FROM liveshare_sessions WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add a participant to a session
    pub async fn add_participant(
        &self,
        data: CreateLiveShareParticipant,
    ) -> Result<LiveShareParticipant, LiveShareRepositoryError> {
        // Check if session exists and is active
        let session = self.get_session_by_id(data.session_id).await?;

        if !session.is_active {
            return Err(LiveShareRepositoryError::SessionInactive);
        }

        // Check if session is full
        let current_count = self.get_active_participant_count(data.session_id).await?;
        if current_count >= session.max_users as i64 {
            return Err(LiveShareRepositoryError::SessionFull);
        }

        // Add participant
        let participant = sqlx::query_as::<_, LiveShareParticipant>(
            r#"
            INSERT INTO liveshare_participants (session_id, user_id, username)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(data.session_id)
        .bind(data.user_id)
        .bind(data.username)
        .fetch_one(&self.pool)
        .await?;

        Ok(participant)
    }

    /// Remove a participant from a session (set left_at)
    pub async fn remove_participant(
        &self,
        participant_id: Uuid,
    ) -> Result<LiveShareParticipant, LiveShareRepositoryError> {
        let participant = sqlx::query_as::<_, LiveShareParticipant>(
            r#"
            UPDATE liveshare_participants
            SET left_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(participant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(participant)
    }

    /// Remove participant by user_id from session
    pub async fn remove_participant_by_user(
        &self,
        session_id: Uuid,
        user_id: Option<Uuid>,
        username: &str,
    ) -> Result<(), LiveShareRepositoryError> {
        sqlx::query(
            r#"
            UPDATE liveshare_participants
            SET left_at = NOW()
            WHERE session_id = $1
              AND ((user_id = $2 AND $2 IS NOT NULL) OR (username = $3 AND user_id IS NULL))
              AND left_at IS NULL
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .bind(username)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get active participants for a session
    pub async fn get_active_participants(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<LiveShareParticipant>, LiveShareRepositoryError> {
        let participants = sqlx::query_as::<_, LiveShareParticipant>(
            r#"
            SELECT * FROM liveshare_participants
            WHERE session_id = $1 AND left_at IS NULL
            ORDER BY joined_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(participants)
    }

    /// Get all participants for a session (including those who left)
    pub async fn get_all_participants(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<LiveShareParticipant>, LiveShareRepositoryError> {
        let participants = sqlx::query_as::<_, LiveShareParticipant>(
            r#"
            SELECT * FROM liveshare_participants
            WHERE session_id = $1
            ORDER BY joined_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(participants)
    }

    /// Get active participant count for a session
    pub async fn get_active_participant_count(
        &self,
        session_id: Uuid,
    ) -> Result<i64, LiveShareRepositoryError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM liveshare_participants
            WHERE session_id = $1 AND left_at IS NULL
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// List all sessions for a diagram
    pub async fn list_sessions_for_diagram(
        &self,
        diagram_id: Uuid,
    ) -> Result<Vec<LiveShareSession>, LiveShareRepositoryError> {
        let sessions = sqlx::query_as::<_, LiveShareSession>(
            r#"
            SELECT * FROM liveshare_sessions
            WHERE diagram_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(diagram_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// List all active sessions for a user (where they have access)
    pub async fn list_active_sessions_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<LiveShareSession>, LiveShareRepositoryError> {
        let sessions = sqlx::query_as::<_, LiveShareSession>(
            r#"
            SELECT DISTINCT ls.* FROM liveshare_sessions ls
            JOIN diagrams d ON d.id = ls.diagram_id
            LEFT JOIN diagram_shares ds ON ds.diagram_id = ls.diagram_id
            WHERE ls.is_active = TRUE
              AND ls.ended_at IS NULL
              AND (d.owner_id = $1 OR ds.user_id = $1)
            ORDER BY ls.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Cleanup expired sessions (ended_at is older than retention period)
    pub async fn cleanup_old_sessions(
        &self,
        retention_days: i64,
    ) -> Result<u64, LiveShareRepositoryError> {
        let cutoff_date = Utc::now() - Duration::days(retention_days);

        let result = sqlx::query(
            r#"
            DELETE FROM liveshare_sessions
            WHERE ended_at IS NOT NULL AND ended_at < $1
            "#,
        )
        .bind(cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Auto-end inactive sessions (no active participants for X minutes)
    pub async fn auto_end_inactive_sessions(
        &self,
        inactivity_minutes: i64,
    ) -> Result<Vec<Uuid>, LiveShareRepositoryError> {
        let cutoff_time = Utc::now() - Duration::minutes(inactivity_minutes);

        let session_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE liveshare_sessions ls
            SET is_active = FALSE, ended_at = NOW(), updated_at = NOW()
            WHERE ls.is_active = TRUE
              AND ls.ended_at IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM liveshare_participants lp
                  WHERE lp.session_id = ls.id
                    AND lp.left_at IS NULL
                    AND lp.joined_at > $1
              )
            RETURNING ls.id
            "#,
        )
        .bind(cutoff_time)
        .fetch_all(&self.pool)
        .await?;

        Ok(session_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a test database to run
    // Run with: cargo test --features ssr -- --test-threads=1

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_liveshare_repository_error_display() {
        assert_eq!(
            LiveShareRepositoryError::NotFound.to_string(),
            "LiveShare session not found"
        );
        assert_eq!(
            LiveShareRepositoryError::DiagramNotFound.to_string(),
            "Diagram not found"
        );
        assert_eq!(
            LiveShareRepositoryError::AccessDenied.to_string(),
            "Access denied"
        );
        assert_eq!(
            LiveShareRepositoryError::SessionFull.to_string(),
            "Session is full"
        );
    }
}
