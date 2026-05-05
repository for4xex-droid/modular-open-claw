CREATE TABLE IF NOT EXISTS approved_karma (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    karma_type TEXT NOT NULL,
    related_skill TEXT NOT NULL,
    lesson TEXT NOT NULL,
    weight INTEGER NOT NULL,
    soul_version_hash TEXT,
    lamport_clock BIGINT NOT NULL DEFAULT 0,
    signature TEXT,
    approved_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL,
    clone_origin_id TEXT,
    generation INTEGER,
    somatic_valence REAL
);

CREATE TABLE IF NOT EXISTS quarantined_karma (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    karma_type TEXT NOT NULL,
    related_skill TEXT NOT NULL,
    lesson TEXT NOT NULL,
    weight INTEGER NOT NULL,
    soul_version_hash TEXT,
    lamport_clock BIGINT NOT NULL DEFAULT 0,
    signature TEXT,
    received_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL,
    clone_origin_id TEXT,
    generation INTEGER,
    somatic_valence REAL
);

CREATE TABLE IF NOT EXISTS approved_rules (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    severity INTEGER NOT NULL,
    action TEXT NOT NULL,
    node_id TEXT NOT NULL,
    lamport_clock BIGINT NOT NULL DEFAULT 0,
    signature TEXT,
    approved_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS quarantined_rules (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    pattern TEXT NOT NULL,
    severity INTEGER NOT NULL,
    action TEXT NOT NULL,
    lamport_clock BIGINT NOT NULL DEFAULT 0,
    signature TEXT,
    received_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_approved_karma_at ON approved_karma(approved_at);
CREATE INDEX IF NOT EXISTS idx_approved_rules_at ON approved_rules(approved_at);
CREATE INDEX IF NOT EXISTS idx_q_karma_node_clock ON quarantined_karma(node_id, lamport_clock);
CREATE INDEX IF NOT EXISTS idx_q_rules_node_clock ON quarantined_rules(node_id, lamport_clock);

CREATE TABLE IF NOT EXISTS approved_arena_matches (
    id TEXT PRIMARY KEY,
    skill_a TEXT NOT NULL,
    skill_b TEXT NOT NULL,
    topic TEXT NOT NULL,
    output_a TEXT,
    output_b TEXT,
    winner TEXT,
    reasoning TEXT,
    approved_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS quarantined_arena_matches (
    id TEXT PRIMARY KEY,
    skill_a TEXT NOT NULL,
    skill_b TEXT NOT NULL,
    topic TEXT NOT NULL,
    output_a TEXT,
    output_b TEXT,
    winner TEXT,
    reasoning TEXT,
    received_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_approved_arena_at ON approved_arena_matches(approved_at);
CREATE INDEX IF NOT EXISTS idx_a_karma_node_clock ON approved_karma(node_id, lamport_clock);
CREATE INDEX IF NOT EXISTS idx_a_rules_node_clock ON approved_rules(node_id, lamport_clock);

CREATE TABLE IF NOT EXISTS node_reputation (
    node_id TEXT PRIMARY KEY,
    reputation_score INTEGER NOT NULL DEFAULT 100,
    is_banned INTEGER NOT NULL DEFAULT 0,
    last_seen_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS biome_topics (
    topic_id TEXT PRIMARY KEY,
    peer_pubkey TEXT NOT NULL,
    summary TEXT,
    status TEXT NOT NULL DEFAULT 'Active',
    turn_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS biome_relay_queue (
    id SERIAL PRIMARY KEY,
    recipient_pubkey TEXT NOT NULL,
    payload TEXT NOT NULL,
    is_delivered INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_biome_relay_recipient ON biome_relay_queue(recipient_pubkey) WHERE is_delivered = 0;

CREATE TABLE IF NOT EXISTS hub_timeline (
    id TEXT PRIMARY KEY, 
    automerge_blob BYTEA NOT NULL, 
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS federated_metrics (
    node_id TEXT NOT NULL, 
    metrics_json TEXT NOT NULL, 
    received_at TEXT NOT NULL, 
    PRIMARY KEY (node_id, received_at)
);

CREATE TABLE IF NOT EXISTS timeline_snapshots (
    node_id TEXT PRIMARY KEY, 
    snapshot_blob BYTEA NOT NULL, 
    received_at TEXT NOT NULL
);
