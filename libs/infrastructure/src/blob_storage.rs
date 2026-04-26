/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core_contracts::contracts::SystemEvent;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

#[async_trait]
pub trait BlobStorageOps: Send + Sync {
    async fn purge_actor_assets(&self, agent_id: Uuid) -> Result<(), AiomeError>;
}

pub struct BlobStorageAdapter {
    local_base_dir: PathBuf,
}

impl BlobStorageAdapter {
    pub fn new(local_base_dir: PathBuf) -> Self {
        Self { local_base_dir }
    }

    /// SystemEvent::ActorForgotten イベントを監視し、該当アクターの物理ファイルを削除する
    pub async fn start_event_listener(
        self: std::sync::Arc<Self>,
        mut rx: broadcast::Receiver<SystemEvent>,
    ) {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(SystemEvent::ActorForgotten(agent_id)) => {
                        info!("🧹 [BlobStorage] Received ActorForgotten event for {}, starting physical purge...", agent_id);
                        if let Err(e) = self.purge_actor_assets(agent_id).await {
                            warn!(
                                "⚠️ [BlobStorage] Failed to purge physical assets for actor {}: {}",
                                agent_id, e
                            );
                        } else {
                            info!(
                                "✅ [BlobStorage] Successfully purged physical assets for actor {}",
                                agent_id
                            );
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("⚠️ [BlobStorage] Event channel closed, stopping listener.");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        warn!(
                            "⚠️ [BlobStorage] Event listener lagged, missed {} events",
                            missed
                        );
                    }
                }
            }
        });
    }
}

#[async_trait]
impl BlobStorageOps for BlobStorageAdapter {
    async fn purge_actor_assets(&self, agent_id: Uuid) -> Result<(), AiomeError> {
        // ローカルストレージ（/tmp や指定された base_dir）の削除
        let actor_dir = self
            .local_base_dir
            .join("actors")
            .join(agent_id.to_string());

        if actor_dir.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(&actor_dir).await {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Failed to delete local actor dir {:?}: {}", actor_dir, e),
                });
            }
        }

        // TODO: S3 や R2 などの外部ストレージパージロジックを追加 (将来拡張用)
        // let prefix = format!("actors/{}/", agent_id);
        // s3_client.delete_objects_with_prefix(bucket, prefix).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_blob_storage_purge_actor_assets() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agent_id = Uuid::new_v4();

        let actor_dir = temp_dir.path().join("actors").join(agent_id.to_string());
        std::fs::create_dir_all(&actor_dir).unwrap();
        std::fs::write(actor_dir.join("profile.png"), b"fake image").unwrap();

        let adapter = BlobStorageAdapter::new(temp_dir.path().to_path_buf());

        assert!(actor_dir.exists());

        adapter.purge_actor_assets(agent_id).await.unwrap();

        // ディレクトリごと削除されていることを確認
        assert!(!actor_dir.exists());
    }
}
