-- Add Phase G Trajectory Extensions
ALTER TABLE trajectory_steps ADD COLUMN reward_signal REAL;
ALTER TABLE trajectory_steps ADD COLUMN llm_prompt_hash TEXT;
