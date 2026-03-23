/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use automerge::{transaction::Transactable, AutoCommit, ReadDoc};
use sqlx::Row;
use tracing::{error, info};

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

        let _row: Option<i64> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_scalar("SELECT last_lamport_clock FROM peers WHERE peer_id = ?")
                    .bind(hub_id)
                    .fetch_optional(p)
                    .await
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query_scalar("SELECT last_lamport_clock FROM peers WHERE peer_id = $1")
                    .bind(hub_id)
                    .fetch_optional(p)
                    .await
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(finalized_blob)
    }

    async fn get_timeline_blob(&self, hub_id: &str) -> Result<Option<Vec<u8>>, AiomeError> {
        let row: Option<Vec<u8>> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_scalar("SELECT automerge_blob FROM timeline_checkpoints WHERE id = ?")
                    .bind(hub_id)
                    .fetch_optional(p)
                    .await
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query_scalar("SELECT automerge_blob FROM timeline_checkpoints WHERE id = $1")
                    .bind(hub_id)
                    .fetch_optional(p)
                    .await
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(row)
    }
}
