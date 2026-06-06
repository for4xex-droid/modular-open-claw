CREATE TABLE IF NOT EXISTS system_state (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO system_state (key, value) 
VALUES ('csam_toxicity_forbidden_words', '["dangerous","illegal","exploit","abuse","trafficking","csam","child exploitation"]')
ON CONFLICT(key) DO NOTHING;
