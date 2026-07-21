-- OP-020-F5 S-2: Soul Sync pairing registry (pubkeys only — no Soul plaintext)
CREATE TABLE IF NOT EXISTS paired_devices (
    session_id TEXT PRIMARY KEY NOT NULL,
    device_a_pubkey TEXT NOT NULL,
    device_b_pubkey TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_paired_devices_a ON paired_devices(device_a_pubkey);
CREATE INDEX IF NOT EXISTS idx_paired_devices_b ON paired_devices(device_b_pubkey);
