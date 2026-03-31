-- SQLite Migration: harness_registry
CREATE TABLE IF NOT EXISTS harness_registry (
    id TEXT PRIMARY KEY,
    domain TEXT NOT NULL,
    description TEXT NOT NULL,
    code_payload TEXT NOT NULL,
    status TEXT NOT NULL,
    severity INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
