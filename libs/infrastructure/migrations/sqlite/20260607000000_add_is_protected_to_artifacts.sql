-- Migration: Add is_protected column to ai_artifacts
ALTER TABLE ai_artifacts ADD COLUMN is_protected INTEGER NOT NULL DEFAULT 0;
