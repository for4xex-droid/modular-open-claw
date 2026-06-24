-- Project NURTURE: Marketplace Migration
-- Table for Digital Items (Metadata)
CREATE TABLE IF NOT EXISTS nurture_items (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    price_coins BIGINT NOT NULL,
    creator_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT NOT NULL -- JSON
);

-- Table for Active Offers in Marketplace
CREATE TABLE IF NOT EXISTS nurture_offers (
    id TEXT PRIMARY KEY,
    seller_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    price_coins BIGINT NOT NULL,
    status TEXT NOT NULL, -- Active, SoldOut, etc.
    stock INTEGER,
    listed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    FOREIGN KEY(item_id) REFERENCES nurture_items(id)
);

CREATE INDEX IF NOT EXISTS idx_items_creator ON nurture_items(creator_id);
CREATE INDEX IF NOT EXISTS idx_offers_seller ON nurture_offers(seller_id);
CREATE INDEX IF NOT EXISTS idx_offers_item ON nurture_offers(item_id);
