CREATE TABLE IF NOT EXISTS score_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_date TEXT NOT NULL,
    metric_name TEXT NOT NULL,  -- 'exp', 'resonance', 'karma_count', 'job_success_rate', 'creativity'
    metric_value REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_score_snapshot_date_metric 
    ON score_snapshots(snapshot_date, metric_name);
