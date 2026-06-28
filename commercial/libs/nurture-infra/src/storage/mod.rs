/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use uuid::Uuid;

#[cfg(feature = "cloud-storage")]
pub mod s3;
#[cfg(feature = "cloud-storage")]
pub use s3::S3AssetStorage;

#[async_trait]
pub trait AssetStorage: Send + Sync {
    async fn put_asset(
        &self,
        actor_id: &ActorId,
        asset_id: &Uuid,
        data: &[u8],
    ) -> Result<(), NurtureError>;
    async fn get_asset(&self, actor_id: &ActorId, asset_id: &Uuid)
        -> Result<Vec<u8>, NurtureError>;
    async fn delete_assets_for_actor(&self, actor_id: &ActorId) -> Result<(), NurtureError>;
}

pub struct MockAssetStorage {
    // Allows us to track if it was called in tests
    pub called_actor: std::sync::Mutex<Option<ActorId>>,
    pub assets: dashmap::DashMap<String, Vec<u8>>, // key: "actor_id/asset_id"
}

impl MockAssetStorage {
    pub fn new() -> Self {
        Self {
            called_actor: std::sync::Mutex::new(None),
            assets: dashmap::DashMap::new(),
        }
    }
}

impl Default for MockAssetStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AssetStorage for MockAssetStorage {
    async fn put_asset(
        &self,
        actor_id: &ActorId,
        asset_id: &Uuid,
        data: &[u8],
    ) -> Result<(), NurtureError> {
        let key = format!("{}/{}", actor_id.0, asset_id);
        self.assets.insert(key, data.to_vec());
        Ok(())
    }

    async fn get_asset(
        &self,
        actor_id: &ActorId,
        asset_id: &Uuid,
    ) -> Result<Vec<u8>, NurtureError> {
        let key = format!("{}/{}", actor_id.0, asset_id);
        if let Some(entry) = self.assets.get(&key) {
            Ok(entry.value().clone())
        } else {
            Err(NurtureError::Infrastructure("Asset not found".to_string()))
        }
    }

    #[allow(clippy::unwrap_used)]
    async fn delete_assets_for_actor(&self, actor_id: &ActorId) -> Result<(), NurtureError> {
        // allow-anti-pattern: Test mock uses unwrap
        let mut guard = self.called_actor.lock().unwrap();
        *guard = Some(*actor_id);
        let prefix = format!("{}/", actor_id.0);
        self.assets.retain(|k, _| !k.starts_with(&prefix));
        Ok(())
    }
}
