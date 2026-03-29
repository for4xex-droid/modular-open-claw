-- Add somatic_valence to track emotional state during lesson extraction
ALTER TABLE karma_logs ADD COLUMN somatic_valence REAL;
