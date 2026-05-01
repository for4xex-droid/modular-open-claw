-- peer_sync_times: Federation クライアント用
CREATE TABLE IF NOT EXISTS peer_sync_times (
    peer_url TEXT PRIMARY KEY,
    last_sync_at TEXT NOT NULL
);
