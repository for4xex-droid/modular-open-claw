-- Add expires_at to nurture_escrows if not exists
-- To support automated TTL (Time-To-Live) and un-locking for stuck escrows

ALTER TABLE nurture_escrows ADD COLUMN expires_at TIMESTAMP;

-- Populate existing rows safely
UPDATE nurture_escrows SET expires_at = datetime(created_at, '+1 day') WHERE expires_at IS NULL;
