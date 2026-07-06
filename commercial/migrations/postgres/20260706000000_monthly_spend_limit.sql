-- OP-059 R2-3: Monthly spend limit columns
ALTER TABLE nurture_wallets ADD COLUMN IF NOT EXISTS monthly_limit BIGINT NOT NULL DEFAULT 0;
ALTER TABLE nurture_wallets ADD COLUMN IF NOT EXISTS spent_this_month BIGINT NOT NULL DEFAULT 0;
