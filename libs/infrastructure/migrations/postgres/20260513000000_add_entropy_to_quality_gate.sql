ALTER TABLE quality_gate_history ADD COLUMN entropy_score DOUBLE PRECISION;
ALTER TABLE quality_gate_history ADD COLUMN retry_count INTEGER;
ALTER TABLE quality_gate_history ADD COLUMN details TEXT;
