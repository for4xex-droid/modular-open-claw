/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use automerge::{transaction::Transactable, AutoCommit, ReadDoc};
use sqlx::Row;
use tracing::error;

#[async_trait]
/// `CrdtOps` トレイト
pub trait CrdtOps {
    /// リモートノードとCRDTタイムラインを同期し、マージ後のBlobを返す
    async fn sync_timeline(
        &self,
        hub_id: &str,
        remote_blob: Option<&[u8]>,
    ) -> Result<Vec<u8>, AiomeError>;
    /// 指定ハブIDのCRDTタイムラインBlobを取得する
    async fn get_timeline_blob(&self, hub_id: &str) -> Result<Option<Vec<u8>>, AiomeError>;
}

#[async_trait]
impl CrdtOps for UniversalJobQueue {
    /// [A-4] CRDT Timeline Sync
    /// Merges local timeline with remote timeline using Automerge.
    async fn sync_timeline(
        &self,
        hub_id: &str,
        remote_blob: Option<&[u8]>,
    ) -> Result<Vec<u8>, AiomeError> {
        let mut local_doc = match self.get_timeline_blob(hub_id).await? {
            Some(blob) => AutoCommit::load(&blob).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Automerge load error: {}", e),
            })?,
            None => AutoCommit::new(),
        };

        if let Some(rb) = remote_blob {
            // SEC: Protect against CRDT OOM Bomb (Size limit: 1MB, aligned with Samsara Hub)
            if rb.len() > 1024 * 1024 {
                return Err(AiomeError::SecurityViolation {
                    reason: "CRDT remote blob exceeds maximum allowed size of 1MB".into(),
                });
            }

            let mut remote_doc = AutoCommit::load(rb).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Remote Automerge load error: {}", e),
            })?;
            local_doc
                .merge(&mut remote_doc)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Automerge merge error: {}", e),
                })?;
        }

        // Add local marker if needed
        let now = chrono::Utc::now().to_rfc3339();
        if let Err(e) = local_doc.put(automerge::ROOT, "last_sync", now) {
            error!("Failed to update CRDT last_sync: {}", e);
        }

        let finalized_blob = local_doc.save();

        let q = format!(
            "INSERT INTO timeline_checkpoints (id, automerge_blob, updated_at) VALUES ({0}, {1}, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET automerge_blob = excluded.automerge_blob, updated_at = CURRENT_TIMESTAMP",
            self.pool.ph(0), self.pool.ph(1)
        );

        crate::sql_exec!(&self.pool, &q, hub_id.to_string(), finalized_blob.clone())?;

        Ok(finalized_blob)
    }

    async fn get_timeline_blob(&self, hub_id: &str) -> Result<Option<Vec<u8>>, AiomeError> {
        let q = format!(
            "SELECT automerge_blob FROM timeline_checkpoints WHERE id = {}",
            self.pool.ph(0)
        );
        let row: Option<Vec<u8>> =
            crate::sql_fetch_optional!(&self.pool, (Vec<u8>,), &q, hub_id)?.map(|r| r.0);

        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use crate::job_queue::trajectory_store::SqliteTrajectoryStore;
    use crate::job_queue::UniversalJobQueue;
    use automerge::{transaction::Transactable, AutoCommit, Value};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;

    async fn setup_test_queue() -> UniversalJobQueue {
        let sql_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Run migrations
        sqlx::query("CREATE TABLE timeline_checkpoints (id TEXT PRIMARY KEY, automerge_blob BLOB NOT NULL, last_seq INTEGER NOT NULL DEFAULT 0, updated_at TEXT DEFAULT (datetime('now')));")
            .execute(&sql_pool)
            .await
            .unwrap();

        let db_pool = DatabasePool::Sqlite(sql_pool.clone());
        let ts = Arc::new(SqliteTrajectoryStore::new(db_pool.clone()));
        UniversalJobQueue::from_pool(db_pool, ts)
    }

    #[tokio::test]
    async fn test_crdt_sync_empty_remote() {
        let q = setup_test_queue().await;
        let blob = q.sync_timeline("hub_1", None).await.unwrap();
        assert!(!blob.is_empty());

        let saved = q.get_timeline_blob("hub_1").await.unwrap().unwrap();
        assert_eq!(blob, saved);
    }

    #[tokio::test]
    async fn test_crdt_sync_with_remote() {
        let q = setup_test_queue().await;

        let mut remote_doc = AutoCommit::new();
        remote_doc
            .put(automerge::ROOT, "remote_key", "remote_value")
            .unwrap();
        let remote_blob = remote_doc.save();

        let merged_blob = q.sync_timeline("hub_2", Some(&remote_blob)).await.unwrap();

        let mut merged_doc = AutoCommit::load(&merged_blob).unwrap();
        let val = merged_doc
            .get(automerge::ROOT, "remote_key")
            .unwrap()
            .unwrap();
        match val.0 {
            Value::Scalar(s) => assert_eq!(s.to_string().trim_matches('"'), "remote_value"),
            _ => panic!("Expected scalar string"),
        }
    }

    #[tokio::test]
    async fn test_crdt_sync_remote_too_large() {
        let q = setup_test_queue().await;
        let giant_blob = vec![0u8; 1024 * 1024 + 1]; // 1MB + 1 byte

        let err = q
            .sync_timeline("hub_3", Some(&giant_blob))
            .await
            .unwrap_err();
        match err {
            AiomeError::SecurityViolation { reason } => {
                assert!(reason.contains("1MB"));
            }
            _ => panic!("Expected SecurityViolation"),
        }
    }
}
