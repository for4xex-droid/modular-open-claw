-- SQLite Migration: harness_registry_v2
ALTER TABLE harness_registry ADD COLUMN version INTEGER DEFAULT 0;
ALTER TABLE harness_registry ADD COLUMN agent_id TEXT;
ALTER TABLE harness_registry ADD COLUMN fire_count BIGINT DEFAULT 0;
ALTER TABLE harness_registry ADD COLUMN false_positive_count BIGINT DEFAULT 0;
ALTER TABLE harness_registry ADD COLUMN last_fired_at DATETIME;
