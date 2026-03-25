-- Add metadata column to asset_registry
ALTER TABLE asset_registry ADD COLUMN metadata JSONB;
