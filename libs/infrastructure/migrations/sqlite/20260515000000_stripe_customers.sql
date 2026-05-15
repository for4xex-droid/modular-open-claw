-- Stripe Customer Registry for Checkout Sessions
CREATE TABLE IF NOT EXISTS stripe_customers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    customer_id TEXT UNIQUE NOT NULL,
    agent_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stripe_customers_agent
    ON stripe_customers(agent_id);

-- Audit trigger (Aiome convention)
CREATE TRIGGER IF NOT EXISTS audit_insert_stripe_customers
    AFTER INSERT ON stripe_customers
BEGIN
    INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash)
    VALUES ('stripe_customers', 'INSERT',
        COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'),
        'stripe_customers:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'),
        COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'),
        hex(randomblob(16)));
END;
