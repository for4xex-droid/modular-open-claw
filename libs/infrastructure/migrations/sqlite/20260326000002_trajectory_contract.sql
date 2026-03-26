-- ADR-024: Trajectory Task Contract (completion_criteria) の追加
ALTER TABLE trajectory_steps ADD COLUMN completion_criteria TEXT;
