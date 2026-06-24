-- Project NURTURE: Escrow Table
-- Sprint C Step 1
-- 2026-04-17
--
-- エスクロー（一時保留）決済を管理するテーブル。
-- aiome 側 DockerConductor, LoraMarketplace, GigEngine から呼び出される。

CREATE TABLE IF NOT EXISTS nurture_escrows (
    escrow_id   TEXT PRIMARY KEY,
    agent_id    TEXT NOT NULL,
    amount      BIGINT NOT NULL CHECK(amount > 0),
    status      TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'released', 'refunded')),
    recipient_id TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_nurture_escrows_agent ON nurture_escrows(agent_id);
CREATE INDEX IF NOT EXISTS idx_nurture_escrows_status ON nurture_escrows(status);
