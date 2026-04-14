CREATE TABLE IF NOT EXISTS prompt_evaluation_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prompt_hash TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    latency_ms INTEGER,
    token_count_in INTEGER,
    token_count_out INTEGER,
    cost_usd DOUBLE PRECISION,
    cache_hit INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_pel_created ON prompt_evaluation_log(created_at);
CREATE INDEX IF NOT EXISTS idx_pel_provider ON prompt_evaluation_log(provider);
