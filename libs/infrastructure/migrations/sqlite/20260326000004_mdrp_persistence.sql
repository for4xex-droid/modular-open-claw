-- Phase 12.7 & 12.8: MDRP Persistence
ALTER TABLE agent_souls ADD COLUMN semantic_index_json TEXT;
ALTER TABLE agent_souls ADD COLUMN persona_boundaries_json TEXT;
