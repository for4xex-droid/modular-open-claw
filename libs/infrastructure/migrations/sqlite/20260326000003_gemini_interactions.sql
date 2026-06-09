-- Gemini Interactions support (Gate 4 Patch)
-- 1. Chat memory session management
ALTER TABLE chat_memory_summaries ADD COLUMN last_interaction_id TEXT;
ALTER TABLE commune_topics ADD COLUMN last_interaction_id TEXT;

-- 2. Trajectory expansion for reasoning and server-side state
ALTER TABLE trajectory_steps ADD COLUMN interaction_id TEXT;
