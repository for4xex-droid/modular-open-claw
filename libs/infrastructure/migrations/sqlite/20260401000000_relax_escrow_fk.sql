PRAGMA foreign_keys=OFF;

CREATE TABLE IF NOT EXISTS new_escrows (
    id TEXT PRIMARY KEY,
    payer_id TEXT NOT NULL,
    recipient_id TEXT,
    order_id TEXT NOT NULL,
    amount INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Locked',
    created_at TEXT DEFAULT (datetime('now'))
);

INSERT INTO new_escrows SELECT id, payer_id, recipient_id, order_id, amount, status, created_at FROM escrows;

DROP TABLE escrows;

ALTER TABLE new_escrows RENAME TO escrows;

PRAGMA foreign_keys=ON;
