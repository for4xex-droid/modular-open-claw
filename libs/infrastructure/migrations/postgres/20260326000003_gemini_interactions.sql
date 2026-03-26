-- Aiome - The Autonomous AI Operating System
-- Phase 5: Gemini Interactions API 統合 (Postgres 版)

-- 1. 会話要約テーブルに Interaction ID を追加
ALTER TABLE chat_memory_summaries ADD COLUMN IF NOT EXISTS last_interaction_id TEXT;

-- 2. トピックテーブルに Interaction ID を追加
ALTER TABLE biome_topics ADD COLUMN IF NOT EXISTS interaction_id TEXT;

-- 3. 実行軌跡（Trajectory）に Interaction ID を追加
ALTER TABLE trajectory_steps ADD COLUMN IF NOT EXISTS interaction_id TEXT;
