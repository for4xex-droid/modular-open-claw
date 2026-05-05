ALTER TABLE node_reputation ADD COLUMN last_seen_lamport_clock INTEGER NOT NULL DEFAULT 0;
