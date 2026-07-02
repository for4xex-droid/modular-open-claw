-- 金額を浮動小数点 (USD) から整数 (セント) に移行する。
-- 既適用マイグレーション (20260426142940) の書き換えは sqlx の
-- チェックサム検証 (VersionMismatch) を引き起こすため、新規マイグレーションで対応する。
-- SQLite は動的型付けのため、宣言型が REAL のままでも整数値を格納できる。
ALTER TABLE nurture_payout_requests RENAME COLUMN amount_usd TO amount_usd_cents;

-- 既存レコードの USD 値をセントに換算する（四捨五入して整数化）
UPDATE nurture_payout_requests
SET amount_usd_cents = CAST(ROUND(amount_usd_cents * 100) AS INTEGER);
