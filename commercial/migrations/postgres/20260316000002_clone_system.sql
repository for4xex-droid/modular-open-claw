-- Phase 3: Shadow Clone System
CREATE TABLE IF NOT EXISTS nurture_clone_instances (
    id TEXT PRIMARY KEY,
    parent_actor_id TEXT NOT NULL,
    pid INTEGER,
    public_key TEXT,
    specialization TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Spawning',
    karma_snapshot_count INTEGER NOT NULL DEFAULT 0,
    karma_merged_count INTEGER DEFAULT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    coins_consumed INTEGER NOT NULL DEFAULT 0,
    escrow_coins INTEGER NOT NULL DEFAULT 0,
    escrow_tx_id TEXT,
    FOREIGN KEY (parent_actor_id) REFERENCES nurture_wallets(actor_id)
);

-- V-29: Karma tracking for clones
CREATE TABLE IF NOT EXISTS karma_logs (
    id TEXT PRIMARY KEY,
    actor_id TEXT NOT NULL,
    delta INTEGER NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    clone_origin_id TEXT
);

CREATE INDEX idx_clone_parent ON nurture_clone_instances(parent_actor_id);
CREATE INDEX idx_clone_status ON nurture_clone_instances(status);

CREATE TABLE IF NOT EXISTS nurture_promoted_clones (
    id TEXT PRIMARY KEY,
    parent_actor_id TEXT NOT NULL,
    promoted_actor_id TEXT NOT NULL UNIQUE,
    specialization TEXT NOT NULL,
    karma_count INTEGER NOT NULL,
    promoted_at TEXT NOT NULL,
    FOREIGN KEY (parent_actor_id) REFERENCES nurture_wallets(actor_id)
);

-- Meditation Cache (Phase SC-2)
CREATE TABLE IF NOT EXISTS nurture_meditation_cache (
    actor_id TEXT NOT NULL,
    soul_hash TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    result_karma_json TEXT NOT NULL,
    PRIMARY KEY (actor_id, soul_hash)
);
