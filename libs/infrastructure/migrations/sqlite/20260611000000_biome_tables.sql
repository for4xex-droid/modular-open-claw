-- Biome Releases v8.3 - SQLite Schema
CREATE TABLE IF NOT EXISTS biome_runs (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    score REAL NOT NULL,
    max_generation INTEGER NOT NULL,
    cell_count INTEGER NOT NULL,
    is_dendou INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS biome_specimens (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    specimen_name TEXT NOT NULL,
    genome_data TEXT NOT NULL,
    rarity TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES biome_runs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS biome_analytics (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    active_cells INTEGER NOT NULL,
    frozen_cells INTEGER NOT NULL,
    element_imbalance REAL NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES biome_runs(id) ON DELETE CASCADE
);
