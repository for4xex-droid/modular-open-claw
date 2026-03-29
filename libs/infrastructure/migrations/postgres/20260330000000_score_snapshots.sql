CREATE TABLE IF NOT EXISTS score_snapshots (
    id SERIAL PRIMARY KEY,
    snapshot_date TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_score_snapshot_date_metric 
    ON score_snapshots(snapshot_date, metric_name);
