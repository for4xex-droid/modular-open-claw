-- Phase 52 Protection: Federation Privacy Filtering
ALTER TABLE karma_logs ADD COLUMN is_private INTEGER DEFAULT 0;
