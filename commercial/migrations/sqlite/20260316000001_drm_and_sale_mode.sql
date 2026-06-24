-- Phase 3: DRM Licenses
-- 2026-03-16

CREATE TABLE IF NOT EXISTS nurture_licenses (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    decryption_key TEXT NOT NULL,
    issued_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    revoked_at TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_licenses_owner ON nurture_licenses(owner_id);
CREATE INDEX IF NOT EXISTS idx_licenses_asset ON nurture_licenses(asset_id);

-- Item Sale Mode and DRM
ALTER TABLE nurture_items ADD COLUMN sale_mode TEXT NOT NULL DEFAULT 'Instant';
ALTER TABLE nurture_items ADD COLUMN drm_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE nurture_items ADD COLUMN subscription_interval_days INTEGER;
ALTER TABLE nurture_items ADD COLUMN subscription_price_coins BIGINT;
