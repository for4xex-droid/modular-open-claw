CREATE TABLE IF NOT EXISTS cortex_typed_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_article_id TEXT NOT NULL,
    target_article_id TEXT NOT NULL,
    link_type TEXT NOT NULL DEFAULT 'references',
    confidence REAL DEFAULT 1.0,
    evidence_text TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_article_id) REFERENCES cortex_wiki_articles(id),
    FOREIGN KEY (target_article_id) REFERENCES cortex_wiki_articles(id),
    UNIQUE(source_article_id, target_article_id, link_type)
);

CREATE INDEX idx_typed_links_source ON cortex_typed_links(source_article_id);
CREATE INDEX idx_typed_links_target ON cortex_typed_links(target_article_id);
CREATE INDEX idx_typed_links_type ON cortex_typed_links(link_type);

CREATE TRIGGER IF NOT EXISTS audit_insert_typed_links AFTER INSERT ON cortex_typed_links BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('cortex_typed_links', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'cortex_typed_links:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_typed_links AFTER UPDATE ON cortex_typed_links BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('cortex_typed_links', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'cortex_typed_links:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
