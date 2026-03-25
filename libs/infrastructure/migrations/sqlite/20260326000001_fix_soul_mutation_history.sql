-- Fix soul_mutation_history schema mismatch
DROP TABLE IF EXISTS soul_mutation_history;
CREATE TABLE soul_mutation_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    old_hash TEXT NOT NULL,
    new_hash TEXT NOT NULL,
    mutation_reason TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);
