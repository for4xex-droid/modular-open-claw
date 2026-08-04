ALTER TABLE prompt_evaluation_log ADD COLUMN IF NOT EXISTS route_tier TEXT;
ALTER TABLE prompt_evaluation_log ADD COLUMN IF NOT EXISTS route_reason TEXT;
ALTER TABLE prompt_evaluation_log ADD COLUMN IF NOT EXISTS route_mode TEXT;
