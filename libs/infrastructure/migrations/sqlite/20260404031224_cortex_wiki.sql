CREATE TABLE IF NOT EXISTS cortex_wiki_articles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content_md TEXT NOT NULL,
    concepts TEXT DEFAULT '[]',
    backlinks TEXT DEFAULT '[]',
    source_refs TEXT DEFAULT '[]',
    content_hash TEXT NOT NULL,
    version INTEGER DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cortex_concept_index (
    concept TEXT PRIMARY KEY,
    article_ids TEXT DEFAULT '[]',
    document_ids TEXT DEFAULT '[]',
    summary TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Phase B: Add compiled tracking to existing cortex_documents
ALTER TABLE cortex_documents ADD COLUMN compiled BOOLEAN DEFAULT 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_wiki_title ON cortex_wiki_articles(title);

CREATE TRIGGER IF NOT EXISTS audit_insert_wiki_articles AFTER INSERT ON cortex_wiki_articles BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('cortex_wiki_articles', 'INSERT', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'cortex_wiki_articles:INSERT:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
CREATE TRIGGER IF NOT EXISTS audit_update_wiki_articles AFTER UPDATE ON cortex_wiki_articles BEGIN INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('cortex_wiki_articles', 'UPDATE', COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), 'cortex_wiki_articles:UPDATE:' || COALESCE(CAST(NEW.id AS TEXT), 'UNKNOWN'), COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16))); END;
