-- Project NURTURE: Initial Economy Migration
-- Table for User Wallets (Coin balance and daily limits)
CREATE TABLE IF NOT EXISTS nurture_wallets (
    actor_id TEXT PRIMARY KEY,
    balance BIGINT NOT NULL DEFAULT 0,
    lifetime_charged BIGINT NOT NULL DEFAULT 0,
    lifetime_spent BIGINT NOT NULL DEFAULT 0,
    daily_limit BIGINT NOT NULL DEFAULT 10000,
    spent_today BIGINT NOT NULL DEFAULT 0,
    last_reset TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Table for Creator Points
CREATE TABLE IF NOT EXISTS nurture_points (
    actor_id TEXT PRIMARY KEY,
    balance BIGINT NOT NULL DEFAULT 0,
    lifetime_earned BIGINT NOT NULL DEFAULT 0,
    lifetime_withdrawn BIGINT NOT NULL DEFAULT 0,
    conversion_rate REAL NOT NULL DEFAULT 1.0
);

-- Table for Economic Ledger (History of all transactions)
CREATE TABLE IF NOT EXISTS nurture_ledger (
    rowid BIGSERIAL UNIQUE,
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    debit_account TEXT NOT NULL,
    credit_account TEXT NOT NULL,
    coin_amount BIGINT NOT NULL,
    points_amount BIGINT NOT NULL,
    entry_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indices for faster querying
CREATE INDEX IF NOT EXISTS idx_ledger_debit ON nurture_ledger(debit_account);
CREATE INDEX IF NOT EXISTS idx_ledger_credit ON nurture_ledger(credit_account);
CREATE INDEX IF NOT EXISTS idx_ledger_transaction ON nurture_ledger(transaction_id);
