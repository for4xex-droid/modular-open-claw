-- Migration: Create commune_shared_genomes table
CREATE TABLE IF NOT EXISTS commune_shared_genomes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic_id TEXT NOT NULL,
    blueprint_json TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);
