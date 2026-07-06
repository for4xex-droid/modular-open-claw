-- OP-012: agent_diagnoses was missing from the PostgreSQL schema (present in sqlite init).
-- Added as a separate migration (NOT edited into 20260324000000_init.sql) because init has
-- already been applied on existing DBs — editing it would trip sqlx checksum validation.
-- Must run before 20260602000000_support_incidents.sql (FK dependency).
CREATE TABLE IF NOT EXISTS agent_diagnoses (
    id SERIAL PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE REFERENCES jobs(id) ON DELETE CASCADE,
    critical_failure_step INTEGER NOT NULL,
    failure_category TEXT NOT NULL,
    root_cause TEXT NOT NULL,
    evidence TEXT NOT NULL,
    self_repair_hint TEXT NOT NULL,
    diagnosed_at TIMESTAMPTZ NOT NULL
);
