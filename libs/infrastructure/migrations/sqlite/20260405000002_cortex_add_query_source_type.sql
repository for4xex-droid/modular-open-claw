CREATE TABLE cortex_documents_new (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source_url TEXT,
    content_md TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('web','pdf','manual','github','rss','query')),
    tags TEXT DEFAULT '[]',
    summary TEXT,
    wiki_article_refs TEXT DEFAULT '[]',
    ingested_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    compiled BOOLEAN DEFAULT 0
);

INSERT INTO cortex_documents_new SELECT * FROM cortex_documents;
DROP TABLE cortex_documents;
ALTER TABLE cortex_documents_new RENAME TO cortex_documents;
