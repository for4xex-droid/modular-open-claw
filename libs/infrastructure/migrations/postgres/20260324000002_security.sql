-- Security Monitoring Tables
CREATE TABLE IF NOT EXISTS security_audit (
    agent_id TEXT PRIMARY KEY,
    request_count INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Initialize system-wide counter
INSERT INTO security_audit (agent_id, request_count) VALUES ('SYSTEM', 0)
ON CONFLICT (agent_id) DO NOTHING;
