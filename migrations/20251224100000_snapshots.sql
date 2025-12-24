-- Snapshots for periodic state preservation
-- Migration: 20251224100000_snapshots

-- ============================================================================
-- LiveShare snapshots table
-- Stores periodic snapshots of session state for recovery and optimization
-- ============================================================================
CREATE TABLE liveshare_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES liveshare_sessions(id) ON DELETE CASCADE,
    snapshot_data BYTEA NOT NULL, -- Serialized SchemaGraph state
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Metadata for cleanup and statistics
    size_bytes INTEGER NOT NULL,
    element_count INTEGER NOT NULL, -- Number of tables + relationships
    CHECK (size_bytes >= 0 AND element_count >= 0)
);

-- Index for finding latest snapshots per session
CREATE INDEX idx_liveshare_snapshots_session_created
    ON liveshare_snapshots(session_id, created_at DESC);

-- Index for cleanup queries (older snapshots)
CREATE INDEX idx_liveshare_snapshots_created
    ON liveshare_snapshots(created_at);

-- ============================================================================
-- Constraints and cleanup policies
-- ============================================================================

-- Add comment for documentation
COMMENT ON TABLE liveshare_snapshots IS 'Periodic snapshots of session state for crash recovery and optimization';
COMMENT ON COLUMN liveshare_snapshots.snapshot_data IS 'Binary-serialized SchemaGraph state (Yrs/Yjs format or custom)';
COMMENT ON COLUMN liveshare_snapshots.size_bytes IS 'Size of snapshot_data in bytes for monitoring';
COMMENT ON COLUMN liveshare_snapshots.element_count IS 'Count of schema elements (tables + relationships) in snapshot';

-- ============================================================================
-- Function: Clean up old snapshots (keep last 10 per session)
-- ============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_snapshots()
RETURNS void AS $$
DECLARE
    v_session_id UUID;
BEGIN
    -- For each session, keep only the latest 10 snapshots
    FOR v_session_id IN
        SELECT DISTINCT session_id FROM liveshare_snapshots
    LOOP
        DELETE FROM liveshare_snapshots
        WHERE session_id = v_session_id
        AND id NOT IN (
            SELECT id FROM liveshare_snapshots
            WHERE session_id = v_session_id
            ORDER BY created_at DESC
            LIMIT 10
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Function: Auto-cleanup trigger
-- Runs after snapshot insert to maintain snapshot count
-- ============================================================================

CREATE OR REPLACE FUNCTION trigger_cleanup_snapshots()
RETURNS TRIGGER AS $$
BEGIN
    -- Clean up old snapshots asynchronously
    -- In production, this might be better as a scheduled job
    PERFORM cleanup_old_snapshots();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-cleanup old snapshots when new snapshot is created
CREATE TRIGGER trigger_liveshare_snapshots_cleanup
    AFTER INSERT ON liveshare_snapshots
    FOR EACH ROW
    EXECUTE FUNCTION trigger_cleanup_snapshots();
