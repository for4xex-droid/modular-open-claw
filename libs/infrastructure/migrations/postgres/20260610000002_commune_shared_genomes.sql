-- Migration: Create commune_shared_genomes table
CREATE TABLE IF NOT EXISTS commune_shared_genomes (
    id SERIAL PRIMARY KEY,
    topic_id VARCHAR(255) NOT NULL,
    blueprint_json TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
