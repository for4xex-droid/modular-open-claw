-- Add prompt_embedding to llm_response_cache for semantic search
ALTER TABLE llm_response_cache ADD COLUMN prompt_embedding BLOB;
