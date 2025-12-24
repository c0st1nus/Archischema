-- Phase 8: Permission checks and security for LiveShare
-- Migration: 20251225080000_liveshare_permissions

-- ============================================================================
-- Add rate limiting tracking table for DDoS prevention
-- Stores connection attempts to detect and prevent abuse
-- ============================================================================
CREATE TABLE liveshare_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES liveshare_sessions(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    window_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    window_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for cleaning old rate limit records
CREATE INDEX idx_liveshare_rate_limits_window_end
    ON liveshare_rate_limits(window_end);

-- Index for finding rate limit records by session
CREATE INDEX idx_liveshare_rate_limits_session
    ON liveshare_rate_limits(session_id);

-- ============================================================================
-- Trigger: Auto-close session when diagram is deleted
-- ============================================================================

CREATE OR REPLACE FUNCTION close_liveshare_sessions_on_diagram_delete()
RETURNS TRIGGER AS $$
BEGIN
    -- Update all active sessions for the deleted diagram to closed
    UPDATE liveshare_sessions
    SET is_active = FALSE, ended_at = NOW()
    WHERE diagram_id = OLD.id AND is_active = TRUE;
    
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_close_liveshare_on_diagram_delete
    BEFORE DELETE ON diagrams
    FOR EACH ROW
    EXECUTE FUNCTION close_liveshare_sessions_on_diagram_delete();

-- ============================================================================
-- Function: Clean up old rate limit records (keep last 24 hours)
-- ============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_rate_limits()
RETURNS void AS $$
BEGIN
    DELETE FROM liveshare_rate_limits
    WHERE window_end < NOW() - INTERVAL '24 hours';
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Add comments for documentation
-- ============================================================================

COMMENT ON TABLE liveshare_rate_limits IS 'Rate limiting tracking for DDoS prevention and abuse detection';
COMMENT ON COLUMN liveshare_rate_limits.message_count IS 'Number of messages sent in this time window';
COMMENT ON COLUMN liveshare_rate_limits.window_start IS 'Start time of the rate limit window';
COMMENT ON COLUMN liveshare_rate_limits.window_end IS 'End time of the rate limit window';
