-- ADR-024: Trajectory 拡張フィールドおよび Task Contract の追加
ALTER TABLE trajectory_steps ADD COLUMN IF NOT EXISTS reasoning TEXT;
ALTER TABLE trajectory_steps ADD COLUMN IF NOT EXISTS parent_step_id TEXT;
ALTER TABLE trajectory_steps ADD COLUMN IF NOT EXISTS step_category TEXT NOT NULL DEFAULT 'General';
ALTER TABLE trajectory_steps ADD COLUMN IF NOT EXISTS completion_criteria TEXT;
