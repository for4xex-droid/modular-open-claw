-- D-3: AI household ledger memo (not included in Merkle audit hash).
ALTER TABLE nurture_ledger ADD COLUMN memo TEXT;
