-- D-5: Items the agent wanted but could not afford (InsufficientBalance signal).
CREATE TABLE IF NOT EXISTS nurture_wishlist (
    agent_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, item_id)
);
