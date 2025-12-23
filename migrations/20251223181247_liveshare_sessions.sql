-- LiveShare sessions and participants migration
-- Migration: 20251223181247_liveshare_sessions

-- ============================================================================
-- LiveShare sessions table
-- Stores persistent LiveShare session data for collaborative editing
-- ============================================================================
CREATE TABLE liveshare_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    diagram_id UUID NOT NULL REFERENCES diagrams(id) ON DELETE CASCADE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255), -- NULL if no password protection
    max_users INTEGER NOT NULL DEFAULT 10 CHECK (max_users >= 2 AND max_users <= 100),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    yjs_state BYTEA, -- Yrs/Yjs document state snapshot for persistence
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ -- NULL while active, set when session ends
);

CREATE INDEX idx_liveshare_sessions_diagram ON liveshare_sessions(diagram_id);
CREATE INDEX idx_liveshare_sessions_owner ON liveshare_sessions(owner_id);
CREATE INDEX idx_liveshare_sessions_active ON liveshare_sessions(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_liveshare_sessions_ended ON liveshare_sessions(ended_at) WHERE ended_at IS NOT NULL;

-- ============================================================================
-- LiveShare participants table
-- Tracks who joined/left each session for audit and analytics
-- ============================================================================
CREATE TABLE liveshare_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES liveshare_sessions(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL, -- NULL if guest/anonymous
    username VARCHAR(100) NOT NULL, -- Stored denormalized for historical records
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at TIMESTAMPTZ -- NULL while still connected
);

CREATE INDEX idx_liveshare_participants_session ON liveshare_participants(session_id);
CREATE INDEX idx_liveshare_participants_user ON liveshare_participants(user_id);
CREATE INDEX idx_liveshare_participants_joined ON liveshare_participants(joined_at);
CREATE INDEX idx_liveshare_participants_active ON liveshare_participants(session_id, left_at) WHERE left_at IS NULL;

-- ============================================================================
-- Apply updated_at trigger to liveshare_sessions
-- ============================================================================
CREATE TRIGGER trigger_liveshare_sessions_updated_at
    BEFORE UPDATE ON liveshare_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Constraints and business logic
-- ============================================================================

-- Ensure only one active session per diagram (optional, can be removed if multiple sessions per diagram are allowed)
CREATE UNIQUE INDEX idx_liveshare_sessions_diagram_active
    ON liveshare_sessions(diagram_id)
    WHERE is_active = TRUE AND ended_at IS NULL;

-- Add comment for documentation
COMMENT ON TABLE liveshare_sessions IS 'Persistent storage for LiveShare collaborative editing sessions';
COMMENT ON COLUMN liveshare_sessions.yjs_state IS 'Serialized Yrs/Yjs document state for session recovery and persistence';
COMMENT ON COLUMN liveshare_sessions.is_active IS 'TRUE if session is currently accepting connections, FALSE if archived';
COMMENT ON COLUMN liveshare_sessions.ended_at IS 'Timestamp when session was explicitly ended by owner or expired';

COMMENT ON TABLE liveshare_participants IS 'Audit log of users who participated in LiveShare sessions';
COMMENT ON COLUMN liveshare_participants.username IS 'Denormalized username at time of joining for historical accuracy';
COMMENT ON COLUMN liveshare_participants.left_at IS 'NULL while user is still in session, set when they disconnect';
