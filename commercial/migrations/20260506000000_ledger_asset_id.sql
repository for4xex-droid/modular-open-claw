-- W-3: LedgerEntry に asset_id を追加 (DRM 判定用)
-- NULL 許容: 既存データは NULL のまま残る (後方互換)
ALTER TABLE nurture_ledger ADD COLUMN asset_id TEXT;
