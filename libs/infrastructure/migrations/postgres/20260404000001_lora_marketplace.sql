-- LoRA Marketplace: Secure adapter trading tables (PostgreSQL)

CREATE TABLE IF NOT EXISTS lora_listings (
    id TEXT PRIMARY KEY,
    seller_id TEXT NOT NULL,
    adapter_path TEXT NOT NULL,
    model_family TEXT NOT NULL,
    base_model TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    price_coins BIGINT NOT NULL,
    adapter_hash TEXT NOT NULL,
    adapter_size_bytes BIGINT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'Open',
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS lora_purchases (
    id TEXT PRIMARY KEY,
    listing_id TEXT NOT NULL REFERENCES lora_listings(id),
    buyer_id TEXT NOT NULL,
    escrow_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Escrowed',
    purchased_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lora_listings_status ON lora_listings(status);
CREATE INDEX IF NOT EXISTS idx_lora_listings_family ON lora_listings(model_family);
CREATE INDEX IF NOT EXISTS idx_lora_listings_seller ON lora_listings(seller_id);
CREATE INDEX IF NOT EXISTS idx_lora_purchases_buyer ON lora_purchases(buyer_id);
CREATE INDEX IF NOT EXISTS idx_lora_purchases_listing ON lora_purchases(listing_id);

-- Audit triggers (reuse existing process_audit function)
DROP TRIGGER IF EXISTS audit_trigger_lora_listings ON lora_listings;
CREATE TRIGGER audit_trigger_lora_listings AFTER INSERT OR UPDATE ON lora_listings FOR EACH ROW EXECUTE FUNCTION process_audit();

DROP TRIGGER IF EXISTS audit_trigger_lora_purchases ON lora_purchases;
CREATE TRIGGER audit_trigger_lora_purchases AFTER INSERT OR UPDATE ON lora_purchases FOR EACH ROW EXECUTE FUNCTION process_audit();
