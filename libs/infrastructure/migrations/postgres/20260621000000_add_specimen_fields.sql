-- Add new specimen detail fields to biome_specimens
ALTER TABLE biome_specimens ADD COLUMN element_balance TEXT DEFAULT '{}';
ALTER TABLE biome_specimens ADD COLUMN morphology_distribution TEXT DEFAULT '{}';
ALTER TABLE biome_specimens ADD COLUMN discovered_reactions TEXT DEFAULT '[]';
ALTER TABLE biome_specimens ADD COLUMN active_cell_count INTEGER DEFAULT 0;
