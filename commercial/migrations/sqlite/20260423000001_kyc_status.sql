CREATE TABLE IF NOT EXISTS nurture_kyc_status (
    actor_id TEXT PRIMARY KEY,
    status TEXT NOT NULL, -- 'pending', 'verified', 'rejected'
    verified_at TIMESTAMP,
    stripe_session_id TEXT
);
