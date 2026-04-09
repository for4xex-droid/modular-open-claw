-- Relax job status constraint to allow 'AwaitingInput'
-- SQLite migration to recreate the jobs table with updated CHECK constraint

PRAGMA foreign_keys=OFF;

-- 1. Create new table
CREATE TABLE jobs_new (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    topic TEXT NOT NULL,
    style_name TEXT NOT NULL,
    karma_directives TEXT NOT NULL CHECK(json_valid(karma_directives)),
    status TEXT NOT NULL CHECK(status IN ('Pending', 'Processing', 'Completed', 'Failed', 'AwaitingInput')),
    started_at TEXT,
    last_heartbeat TEXT,
    tech_karma_extracted INTEGER NOT NULL DEFAULT 0,
    creative_rating INTEGER CHECK(creative_rating IN (-1, 0, 1)),
    execution_log TEXT,
    error_message TEXT,
    sns_platform TEXT,
    sns_content_id TEXT,
    published_at TEXT,
    output_artifacts TEXT,
    permission_manifest TEXT,
    agent_id TEXT,
    priority INTEGER NOT NULL DEFAULT 100,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    requires_review INTEGER DEFAULT 0
);

-- 2. Copy data
INSERT INTO jobs_new SELECT * FROM jobs;

-- 3. Drop old table and rename
DROP TABLE jobs;
ALTER TABLE jobs_new RENAME TO jobs;

-- 4. Re-create indexes
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);

-- 5. Re-create triggers (as defined in 20260324000000_init.sql)
CREATE TRIGGER IF NOT EXISTS audit_insert_jobs AFTER INSERT ON jobs 
BEGIN 
    INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) 
    VALUES ('jobs', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'jobs:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); 
END;

CREATE TRIGGER IF NOT EXISTS audit_update_jobs AFTER UPDATE ON jobs 
BEGIN 
    INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) 
    VALUES ('jobs', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'jobs:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); 
END;

PRAGMA foreign_keys=ON;
