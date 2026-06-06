-- Migration: Add is_protected column to ai_artifacts
ALTER TABLE ai_artifacts ADD COLUMN is_protected BOOLEAN NOT NULL DEFAULT FALSE;
