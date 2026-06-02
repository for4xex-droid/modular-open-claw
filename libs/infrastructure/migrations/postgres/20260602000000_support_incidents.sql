-- 20260602000000_support_incidents.sql
CREATE TABLE IF NOT EXISTS support_incidents (
    id VARCHAR(255) PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    severity VARCHAR(50) NOT NULL DEFAULT 'Medium',
    user_hash VARCHAR(255) NOT NULL,
    channel_id VARCHAR(255),
    system_context TEXT,
    suggested_fix TEXT,
    related_diagnosis_id INTEGER,
    status VARCHAR(50) NOT NULL DEFAULT 'Open',
    resolved_at VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(related_diagnosis_id) REFERENCES agent_diagnoses(id)
);

CREATE INDEX IF NOT EXISTS idx_support_incidents_status ON support_incidents(status);
CREATE INDEX IF NOT EXISTS idx_support_incidents_created_at ON support_incidents(created_at);
CREATE INDEX IF NOT EXISTS idx_support_incidents_severity ON support_incidents(severity);
