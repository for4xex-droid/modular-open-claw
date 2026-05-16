-- Add X specific metrics for Buzz Protocol
ALTER TABLE sns_metrics_history ADD COLUMN repost_count INTEGER DEFAULT 0;
ALTER TABLE sns_metrics_history ADD COLUMN quote_count INTEGER DEFAULT 0;
ALTER TABLE sns_metrics_history ADD COLUMN reply_count INTEGER DEFAULT 0;
ALTER TABLE sns_metrics_history ADD COLUMN impression_count INTEGER DEFAULT 0;
