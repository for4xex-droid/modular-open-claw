ALTER TABLE cortex_documents DROP CONSTRAINT cortex_documents_source_type_check;
ALTER TABLE cortex_documents ADD CONSTRAINT cortex_documents_source_type_check CHECK (source_type IN ('web','pdf','manual','github','rss','query'));
