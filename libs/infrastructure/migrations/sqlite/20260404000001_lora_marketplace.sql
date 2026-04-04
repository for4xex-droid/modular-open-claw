-- LoRA Marketplace: Secure adapter trading tables

CREATE TABLE IF NOT EXISTS lora_listings (
    id TEXT PRIMARY KEY,
    seller_id TEXT NOT NULL,
    adapter_path TEXT NOT NULL,
    model_family TEXT NOT NULL,
    base_model TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    price_coins INTEGER NOT NULL,
    adapter_hash TEXT NOT NULL,
    adapter_size_bytes INTEGER NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'Open',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS lora_purchases (
    id TEXT PRIMARY KEY,
    listing_id TEXT NOT NULL REFERENCES lora_listings(id),
    buyer_id TEXT NOT NULL,
    escrow_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Escrowed',
    purchased_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_lora_listings_status ON lora_listings(status);
CREATE INDEX IF NOT EXISTS idx_lora_listings_family ON lora_listings(model_family);
CREATE INDEX IF NOT EXISTS idx_lora_listings_seller ON lora_listings(seller_id);
CREATE INDEX IF NOT EXISTS idx_lora_purchases_buyer ON lora_purchases(buyer_id);
CREATE INDEX IF NOT EXISTS idx_lora_purchases_listing ON lora_purchases(listing_id);

-- Audit triggers for tamper-evident ledger
CREATE TRIGGER IF NOT EXISTS audit_insert_lora_listings AFTER INSERT ON lora_listings BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('lora_listings', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'lora_listings:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_lora_listings AFTER UPDATE ON lora_listings BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('lora_listings', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'lora_listings:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_insert_lora_purchases AFTER INSERT ON lora_purchases BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('lora_purchases', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'lora_purchases:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_lora_purchases AFTER UPDATE ON lora_purchases BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('lora_purchases', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'lora_purchases:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
