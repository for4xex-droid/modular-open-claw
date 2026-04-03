CREATE TABLE IF NOT EXISTS cortex_documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source_url TEXT,
    content_md TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('web','pdf','manual','github','rss')),
    tags TEXT DEFAULT '[]',
    summary TEXT,
    wiki_article_refs TEXT DEFAULT '[]',
    ingested_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
