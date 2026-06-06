-- Phase 2 Updates: Optimistic Locking, Audit Trail, and Saga Support

-- Update Wallets
ALTER TABLE nurture_wallets ADD COLUMN version BIGINT NOT NULL DEFAULT 0;
ALTER TABLE nurture_wallets ADD COLUMN last_transaction_at TIMESTAMP;

-- Update Ledger
ALTER TABLE nurture_ledger ADD COLUMN audit_hash TEXT;

-- Saga Logs
CREATE TABLE IF NOT EXISTS nurture_saga_logs (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    status TEXT NOT NULL,
    payload TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Idempotency
CREATE TABLE IF NOT EXISTS nurture_idempotency (
    key TEXT PRIMARY KEY,
    status_code INTEGER,
    response_body TEXT,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Customers
CREATE TABLE IF NOT EXISTS nurture_customers (
    actor_id TEXT PRIMARY KEY,
    stripe_customer_id TEXT NOT NULL UNIQUE,
    email TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
