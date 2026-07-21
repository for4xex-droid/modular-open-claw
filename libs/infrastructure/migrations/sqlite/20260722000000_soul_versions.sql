-- OP-020-F5 S-3: soul_versions was present on Postgres init but missing on SQLite.
CREATE TABLE IF NOT EXISTS soul_versions (
    hash TEXT PRIMARY KEY,
    soul_id TEXT NOT NULL REFERENCES agent_souls(id) ON DELETE CASCADE,
    parent_hash TEXT,
    somatic_markers_json TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_soul_versions_soul_id ON soul_versions(soul_id);
