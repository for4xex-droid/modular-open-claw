CREATE TABLE IF NOT EXISTS cortex_activity_log (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail_json TEXT DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
