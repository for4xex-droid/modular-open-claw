-- 20260602000000_support_incidents.sql
CREATE TABLE IF NOT EXISTS support_incidents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'Medium',
    user_hash TEXT NOT NULL,
    channel_id TEXT,
    system_context TEXT,
    suggested_fix TEXT,
    related_diagnosis_id INTEGER,
    status TEXT NOT NULL DEFAULT 'Open',
    resolved_at TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(related_diagnosis_id) REFERENCES agent_diagnoses(id)
);

CREATE INDEX IF NOT EXISTS idx_support_incidents_status ON support_incidents(status);
CREATE INDEX IF NOT EXISTS idx_support_incidents_created_at ON support_incidents(created_at);
CREATE INDEX IF NOT EXISTS idx_support_incidents_severity ON support_incidents(severity);
