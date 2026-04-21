CREATE TABLE IF NOT EXISTS quality_gate_history (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    job_id TEXT NOT NULL,
    score INTEGER NOT NULL,
    passed INTEGER DEFAULT 0,
    conductor TEXT DEFAULT 'GeoAuditConductor',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_qgh_created ON quality_gate_history(created_at);
CREATE INDEX IF NOT EXISTS idx_qgh_job ON quality_gate_history(job_id);
