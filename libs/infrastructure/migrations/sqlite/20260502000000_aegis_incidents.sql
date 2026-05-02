CREATE TABLE IF NOT EXISTS aegis_incidents (
    id TEXT PRIMARY KEY,
    skill_name TEXT NOT NULL,
    wasm_hash TEXT NOT NULL,
    input_payload TEXT NOT NULL,
    stack_trace TEXT NOT NULL,
    status TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);


CREATE INDEX IF NOT EXISTS idx_aegis_incidents_skill_name ON aegis_incidents(skill_name);
CREATE INDEX IF NOT EXISTS idx_aegis_incidents_status ON aegis_incidents(status);
CREATE INDEX IF NOT EXISTS idx_aegis_incidents_created_at ON aegis_incidents(created_at);
