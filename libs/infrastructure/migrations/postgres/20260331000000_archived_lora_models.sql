CREATE TABLE IF NOT EXISTS archived_lora_models (
    id SERIAL PRIMARY KEY,
    soul_id TEXT NOT NULL REFERENCES agent_souls(id) ON DELETE CASCADE,
    generation INTEGER NOT NULL,
    lora_hash TEXT NOT NULL,
    adapter_path TEXT NOT NULL,
    base_model TEXT NOT NULL,
    archived_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
