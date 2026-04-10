-- Add Cortex Concept FTS5 support for edge knowledge search

-- Create FTS5 Virtual Table linked to the external content table 'cortex_concept_index'
CREATE VIRTUAL TABLE IF NOT EXISTS cortex_concept_fts USING fts5(concept, content=cortex_concept_index, content_rowid=rowid);

-- Create synchronization triggers (3-way)

-- Type: INSERT
CREATE TRIGGER IF NOT EXISTS cortex_fts_insert AFTER INSERT ON cortex_concept_index BEGIN
  INSERT INTO cortex_concept_fts(rowid, concept) VALUES (new.rowid, new.concept);
END;

-- Type: UPDATE
CREATE TRIGGER IF NOT EXISTS cortex_fts_update AFTER UPDATE ON cortex_concept_index BEGIN
  INSERT INTO cortex_concept_fts(cortex_concept_fts, rowid, concept) VALUES('delete', old.rowid, old.concept);
  INSERT INTO cortex_concept_fts(rowid, concept) VALUES (new.rowid, new.concept);
END;

-- Type: DELETE
CREATE TRIGGER IF NOT EXISTS cortex_fts_delete AFTER DELETE ON cortex_concept_index BEGIN
  INSERT INTO cortex_concept_fts(cortex_concept_fts, rowid, concept) VALUES('delete', old.rowid, old.concept);
END;

-- Sync existing data
INSERT INTO cortex_concept_fts(rowid, concept) SELECT rowid, concept FROM cortex_concept_index;
