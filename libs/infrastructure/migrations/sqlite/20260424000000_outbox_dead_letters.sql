CREATE TABLE IF NOT EXISTS outbox_dead_letters (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    error_reason TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_dlq_created_at ON outbox_dead_letters (created_at);
CREATE INDEX IF NOT EXISTS idx_dlq_event_type ON outbox_dead_letters (event_type);
