-- Migration: Add payload_type to commune_messages
ALTER TABLE commune_messages ADD COLUMN payload_type TEXT DEFAULT NULL;
