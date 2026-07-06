CREATE TABLE IF NOT EXISTS cortex_typed_links (
    id BIGSERIAL PRIMARY KEY,
    source_article_id TEXT NOT NULL,
    target_article_id TEXT NOT NULL,
    link_type TEXT NOT NULL DEFAULT 'references',
    confidence DOUBLE PRECISION DEFAULT 1.0,
    evidence_text TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_article_id) REFERENCES cortex_wiki_articles(id),
    FOREIGN KEY (target_article_id) REFERENCES cortex_wiki_articles(id),
    UNIQUE(source_article_id, target_article_id, link_type)
);

CREATE INDEX idx_typed_links_source ON cortex_typed_links(source_article_id);
CREATE INDEX idx_typed_links_target ON cortex_typed_links(target_article_id);
CREATE INDEX idx_typed_links_type ON cortex_typed_links(link_type);

CREATE TRIGGER audit_insert_typed_links AFTER INSERT ON cortex_typed_links FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_update_typed_links AFTER UPDATE ON cortex_typed_links FOR EACH ROW EXECUTE FUNCTION process_audit();
