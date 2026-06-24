CREATE TABLE IF NOT EXISTS nurture_payout_requests (
    id TEXT PRIMARY KEY NOT NULL,
    actor_id TEXT NOT NULL,
    amount_usd REAL NOT NULL,
    points_burned INTEGER NOT NULL,
    status TEXT NOT NULL,
    provider_reference_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_nurture_payout_requests_actor ON nurture_payout_requests(actor_id);
