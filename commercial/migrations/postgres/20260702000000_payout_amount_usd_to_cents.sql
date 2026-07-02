-- 金額を浮動小数点 (USD) から整数 (セント) に移行する。
-- 既適用マイグレーション (20260426142940) の書き換えは sqlx の
-- チェックサム検証 (VersionMismatch) を引き起こすため、新規マイグレーションで対応する。
ALTER TABLE nurture_payout_requests RENAME COLUMN amount_usd TO amount_usd_cents;

ALTER TABLE nurture_payout_requests
    ALTER COLUMN amount_usd_cents TYPE INTEGER
    USING ROUND(amount_usd_cents * 100)::INTEGER;
