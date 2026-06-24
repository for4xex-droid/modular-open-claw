CREATE TABLE IF NOT EXISTS nurture_ncmec_reports (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    evidence_metadata TEXT NOT NULL,
    reported_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ncmec_status ON nurture_ncmec_reports(status);
