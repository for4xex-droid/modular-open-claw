-- Core Infrastructure Tables
CREATE TABLE IF NOT EXISTS audit_ledger_global (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    operation TEXT NOT NULL,
    record_id TEXT NOT NULL,
    new_data TEXT NOT NULL,
    prev_hash TEXT NOT NULL,
    current_hash TEXT NOT NULL,
    timestamp TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_ledger_time ON audit_ledger_global(timestamp);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY, 
    category TEXT NOT NULL,
    topic TEXT NOT NULL,
    style_name TEXT NOT NULL, 
    karma_directives TEXT NOT NULL CHECK(json_valid(karma_directives)), 
    status TEXT NOT NULL CHECK(status IN ('Pending', 'Processing', 'Completed', 'Failed')),
    started_at TEXT, 
    last_heartbeat TEXT,
    tech_karma_extracted INTEGER NOT NULL DEFAULT 0, 
    creative_rating INTEGER CHECK(creative_rating IN (-1, 0, 1)), 
    execution_log TEXT,
    error_message TEXT,
    sns_platform TEXT,
    sns_content_id TEXT,
    published_at TEXT,
    output_artifacts TEXT,
    permission_manifest TEXT,
    agent_id TEXT,
    priority INTEGER NOT NULL DEFAULT 100,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS karma_logs (
    id TEXT PRIMARY KEY,
    job_id TEXT, 
    karma_type TEXT NOT NULL CHECK(karma_type IN ('Technical', 'Creative', 'Synthesized')),
    related_skill TEXT NOT NULL, 
    lesson TEXT NOT NULL,        
    weight INTEGER NOT NULL DEFAULT 100 CHECK(weight BETWEEN 0 AND 100), 
    soul_version_hash TEXT,
    karma_embedding BLOB,
    is_federated INTEGER NOT NULL DEFAULT 0,
    clone_origin_id TEXT,
    tier TEXT NOT NULL DEFAULT 'WARM',
    apply_count INTEGER NOT NULL DEFAULT 0,
    lamport_clock INTEGER NOT NULL DEFAULT 0,
    node_id TEXT NOT NULL DEFAULT '',
    signature TEXT,
    domain TEXT DEFAULT 'general',
    subtopic TEXT,
    is_archived INTEGER NOT NULL DEFAULT 0,
    last_applied_at TEXT DEFAULT (datetime('now')),
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_karma_logs_skill_weight ON karma_logs(related_skill, weight DESC);
CREATE INDEX IF NOT EXISTS idx_karma_tier ON karma_logs(tier);
CREATE INDEX IF NOT EXISTS idx_karma_logs_federated ON karma_logs(is_federated) WHERE is_federated = 0;
CREATE INDEX IF NOT EXISTS idx_karma_lamport ON karma_logs(lamport_clock, node_id);
CREATE INDEX IF NOT EXISTS idx_karma_logs_active ON karma_logs(is_archived) WHERE is_archived = 0;
CREATE INDEX IF NOT EXISTS idx_karma_taxonomy ON karma_logs(domain, related_skill);

CREATE VIRTUAL TABLE IF NOT EXISTS karma_fts USING fts5(lesson, content=karma_logs, content_rowid=rowid);

CREATE TABLE IF NOT EXISTS system_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ai_artifacts (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    category TEXT NOT NULL CHECK(category IN ('report', 'code', 'image', 'audio', 'expression', 'data', 'knowledge')),
    tags TEXT NOT NULL DEFAULT '[]', 
    created_by TEXT NOT NULL,
    dir_path TEXT NOT NULL,
    file_manifest TEXT NOT NULL, 
    karma_refs TEXT NOT NULL DEFAULT '[]', 
    job_ref TEXT,
    soul_version_hash TEXT,
    signature TEXT,
    embedding BLOB,
    text_content TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(job_ref) REFERENCES jobs(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS revenue_splits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_id TEXT NOT NULL,
    recipient_id TEXT NOT NULL,
    role TEXT NOT NULL,
    amount INTEGER NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS gig_intents (
    id TEXT PRIMARY KEY,
    requester_id TEXT NOT NULL,
    description TEXT NOT NULL,
    criteria TEXT NOT NULL,
    max_budget_coins INTEGER NOT NULL,
    category TEXT NOT NULL DEFAULT 'Other',
    deadline TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Open',
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS gig_bids (
    id TEXT PRIMARY KEY,
    intent_id TEXT NOT NULL,
    bidder_id TEXT NOT NULL,
    price_coins INTEGER NOT NULL,
    est_duration_sec INTEGER NOT NULL,
    deposit_amount INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending',
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(intent_id) REFERENCES gig_intents(id)
);

CREATE TABLE IF NOT EXISTS escrows (
    id TEXT PRIMARY KEY,
    payer_id TEXT NOT NULL,
    recipient_id TEXT,
    order_id TEXT NOT NULL,
    amount INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Locked',
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(order_id) REFERENCES gig_intents(id)
);

CREATE TABLE IF NOT EXISTS gig_deliveries (
    order_id TEXT PRIMARY KEY,
    deliverer_id TEXT NOT NULL,
    artifact_path TEXT NOT NULL,
    metadata TEXT NOT NULL,
    delivered_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(order_id) REFERENCES gig_intents(id)
);

CREATE TABLE IF NOT EXISTS sns_metrics_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    milestone_days INTEGER NOT NULL,
    views INTEGER NOT NULL,
    likes INTEGER NOT NULL,
    comments_count INTEGER NOT NULL,
    raw_comments_json TEXT,
    oracle_score_topic REAL,
    oracle_score_visual REAL,
    oracle_score_soul REAL,
    oracle_reason TEXT,
    hard_metric_score REAL,
    engagement_rate REAL,
    alignment_score REAL,
    growth_score REAL,
    lesson TEXT,
    should_evolve INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 0,
    is_finalized INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sns_metrics_job ON sns_metrics_history(job_id, milestone_days);

CREATE TABLE IF NOT EXISTS agent_stats (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    level INTEGER NOT NULL DEFAULT 1,
    exp INTEGER NOT NULL DEFAULT 0,
    resonance INTEGER NOT NULL DEFAULT 0,
    creativity INTEGER NOT NULL DEFAULT 0,
    fatigue INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS chat_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    is_distilled INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS chat_memory_summaries (
    channel_id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS soul_mutation_history (
    id TEXT PRIMARY KEY,
    parent_hash TEXT NOT NULL,
    mutation_diff TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS federation_peers (
    peer_url TEXT PRIMARY KEY,
    last_sync_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS immune_rules (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    severity INTEGER NOT NULL,
    action TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    is_federated INTEGER NOT NULL DEFAULT 0,
    lamport_clock INTEGER NOT NULL DEFAULT 0,
    node_id TEXT NOT NULL DEFAULT '',
    signature TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS arena_history (
    id TEXT PRIMARY KEY,
    skill_a TEXT NOT NULL,
    skill_b TEXT NOT NULL,
    topic TEXT NOT NULL,
    output_a TEXT,
    output_b TEXT,
    winner TEXT,
    reasoning TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS commune_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sender_pubkey TEXT NOT NULL,
    recipient_pubkey TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    content TEXT NOT NULL,
    karma_root_cid TEXT NOT NULL,
    signature TEXT NOT NULL,
    lamport_clock INTEGER NOT NULL,
    encryption TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS commune_peers (
    pubkey TEXT PRIMARY KEY,
    last_seen_at TEXT DEFAULT (datetime('now')),
    reputation_score INTEGER NOT NULL DEFAULT 100
);

CREATE TABLE IF NOT EXISTS commune_topics (
    topic_id TEXT PRIMARY KEY,
    peer_pubkey TEXT NOT NULL,
    summary TEXT,
    status TEXT NOT NULL,
    turn_count INTEGER NOT NULL DEFAULT 0,
    cooldown_until TEXT,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS evolution_chronicle (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level_at INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    description TEXT NOT NULL,
    inspiration_source TEXT,
    karma_snapshot TEXT,
    prev_record_hash TEXT NOT NULL,
    record_hash TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS timeline_checkpoints (
    id TEXT PRIMARY KEY,
    automerge_blob BLOB NOT NULL,
    last_seq INTEGER NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_souls (
    id TEXT PRIMARY KEY,
    generation INTEGER NOT NULL,
    soul_hash TEXT NOT NULL,
    somatic_markers_json TEXT NOT NULL,
    defenses_json TEXT NOT NULL,
    predictive_model_json TEXT NOT NULL,
    attachment_json TEXT NOT NULL,
    instinct_json TEXT NOT NULL,
    anamnesis_json TEXT NOT NULL,
    experience_buffer_json TEXT,  
    lora_adapter_path TEXT,      
    lora_base_model TEXT,        
    lora_hash TEXT,              
    last_begging_at TEXT,        
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS artifact_edges (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('Artifact', 'Karma')),
    relation TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'system',
    is_secret INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS expressions (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    emotion TEXT NOT NULL,
    karma_refs TEXT NOT NULL DEFAULT '[]',
    audio_path TEXT,
    duration_ms INTEGER,
    avatar_params TEXT,
    tts_status TEXT NOT NULL DEFAULT 'NotRequested',
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS resource_usage_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT,
    provider_name TEXT NOT NULL,
    model_name TEXT NOT NULL,
    usage_type TEXT NOT NULL,
    amount INTEGER NOT NULL,
    estimated_cost_usd REAL NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(job_id) REFERENCES jobs(id)
);

CREATE TABLE IF NOT EXISTS trajectory_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    step_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    tool_name TEXT,
    input_json TEXT,
    output_json TEXT,
    timestamp TEXT NOT NULL,
    constraint_violations TEXT,
    is_critical_failure INTEGER DEFAULT 0,
    failure_category TEXT,
    FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS agent_diagnoses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL UNIQUE,
    critical_failure_step INTEGER NOT NULL,
    failure_category TEXT NOT NULL,
    root_cause TEXT NOT NULL,
    evidence TEXT NOT NULL,
    self_repair_hint TEXT NOT NULL,
    diagnosed_at TEXT NOT NULL,
    FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS stripe_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    metadata TEXT,
    processed_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS polar_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    metadata TEXT,
    processed_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS vault_keys (
    asset_id TEXT PRIMARY KEY,
    encrypted_key BLOB NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS asset_registry (
    id TEXT PRIMARY KEY,
    creator_id TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    price_coins INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS licenses (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    original_event_id TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    granted_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ekyc_sessions (
    user_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    status TEXT DEFAULT 'requires_input',
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS verification_logs (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    criteria_type TEXT NOT NULL,
    passed INTEGER NOT NULL,
    score REAL NOT NULL,
    detail TEXT,
    verified_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(order_id) REFERENCES gig_intents(id)
);

CREATE TABLE IF NOT EXISTS llm_response_cache (
    prompt_hash TEXT PRIMARY KEY,
    response TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    model_name TEXT NOT NULL,
    ttl_seconds INTEGER NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS trend_cache (
    source_url TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Triggers 
CREATE TRIGGER IF NOT EXISTS audit_insert_jobs AFTER INSERT ON jobs BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('jobs', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'jobs:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_jobs AFTER UPDATE ON jobs BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('jobs', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'jobs:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_karma_logs AFTER INSERT ON karma_logs BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('karma_logs', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'karma_logs:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_karma_logs AFTER UPDATE ON karma_logs BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('karma_logs', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'karma_logs:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_system_state AFTER INSERT ON system_state BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('system_state', 'INSERT', COALESCE(CAST(NEW.key AS TEXT), 'UNKNOWN'), 'system_state:INSERT:' || COALESCE(CAST(NEW.key AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_system_state AFTER UPDATE ON system_state BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('system_state', 'UPDATE', COALESCE(CAST(NEW.key AS TEXT), 'UNKNOWN'), 'system_state:UPDATE:' || COALESCE(CAST(NEW.key AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_ai_artifacts AFTER INSERT ON ai_artifacts BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('ai_artifacts', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'ai_artifacts:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_ai_artifacts AFTER UPDATE ON ai_artifacts BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('ai_artifacts', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'ai_artifacts:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_revenue_splits AFTER INSERT ON revenue_splits BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('revenue_splits', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'revenue_splits:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_revenue_splits AFTER UPDATE ON revenue_splits BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('revenue_splits', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'revenue_splits:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_gig_intents AFTER INSERT ON gig_intents BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('gig_intents', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'gig_intents:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_gig_intents AFTER UPDATE ON gig_intents BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('gig_intents', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'gig_intents:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_gig_bids AFTER INSERT ON gig_bids BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('gig_bids', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'gig_bids:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_gig_bids AFTER UPDATE ON gig_bids BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('gig_bids', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'gig_bids:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_escrows AFTER INSERT ON escrows BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('escrows', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'escrows:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_escrows AFTER UPDATE ON escrows BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('escrows', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'escrows:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

CREATE TRIGGER IF NOT EXISTS audit_insert_gig_deliveries AFTER INSERT ON gig_deliveries BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('gig_deliveries', 'INSERT', COALESCE(CAST(NEW.order_id AS TEXT), 'UNKNOWN'), 'gig_deliveries:INSERT:' || COALESCE(CAST(NEW.order_id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_gig_deliveries AFTER UPDATE ON gig_deliveries BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('gig_deliveries', 'UPDATE', COALESCE(CAST(NEW.order_id AS TEXT), 'UNKNOWN'), 'gig_deliveries:UPDATE:' || COALESCE(CAST(NEW.order_id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;

