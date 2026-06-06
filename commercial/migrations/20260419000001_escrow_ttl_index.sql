-- Project NURTURE: Escrow TTL Composite Index
--
-- process_expired_escrows の検索クエリ (status = 'pending' AND expires_at < ?)
-- を最適化するための複合インデックス。

CREATE INDEX IF NOT EXISTS idx_nurture_escrows_status_expires 
ON nurture_escrows(status, expires_at);
