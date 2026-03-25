-- ADR-024: Trajectory 拡張フィールドの追加
ALTER TABLE trajectory_steps ADD COLUMN reasoning TEXT;
ALTER TABLE trajectory_steps ADD COLUMN parent_step_id TEXT;
ALTER TABLE trajectory_steps ADD COLUMN step_category TEXT NOT NULL DEFAULT 'General';
