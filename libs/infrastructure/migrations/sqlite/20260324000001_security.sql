-- Security Monitoring Tables
CREATE TABLE IF NOT EXISTS security_audit (
    agent_id TEXT NOT NULL PRIMARY KEY,
    request_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Initialize system-wide counter
INSERT OR IGNORE INTO security_audit (agent_id, request_count) VALUES ('SYSTEM', 0);
