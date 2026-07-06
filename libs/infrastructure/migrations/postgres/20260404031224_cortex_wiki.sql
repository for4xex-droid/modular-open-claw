CREATE TABLE IF NOT EXISTS cortex_wiki_articles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content_md TEXT NOT NULL,
    concepts TEXT DEFAULT '[]',
    backlinks TEXT DEFAULT '[]',
    source_refs TEXT DEFAULT '[]',
    content_hash TEXT NOT NULL,
    version BIGINT DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cortex_concept_index (
    concept TEXT PRIMARY KEY,
    article_ids TEXT DEFAULT '[]',
    document_ids TEXT DEFAULT '[]',
    summary TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE cortex_documents ADD COLUMN compiled BOOLEAN DEFAULT FALSE;
CREATE UNIQUE INDEX IF NOT EXISTS idx_wiki_title ON cortex_wiki_articles(title);

CREATE TRIGGER audit_insert_wiki_articles AFTER INSERT ON cortex_wiki_articles FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_update_wiki_articles AFTER UPDATE ON cortex_wiki_articles FOR EACH ROW EXECUTE FUNCTION process_audit();
