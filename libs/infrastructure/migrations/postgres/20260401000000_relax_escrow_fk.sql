-- Relaxing FK constraint to allow API usage escrows
ALTER TABLE escrows DROP CONSTRAINT IF EXISTS escrows_order_id_fkey;
