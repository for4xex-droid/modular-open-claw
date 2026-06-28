/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use async_trait::async_trait;
use aws_sdk_s3::Client;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use uuid::Uuid;

use super::AssetStorage;

pub struct S3AssetStorage {
    client: Client,
    bucket: String,
}

impl S3AssetStorage {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }
}

#[async_trait]
impl AssetStorage for S3AssetStorage {
    async fn put_asset(
        &self,
        actor_id: &ActorId,
        asset_id: &Uuid,
        data: &[u8],
    ) -> Result<(), NurtureError> {
        let key = format!("actors/{}/{}", actor_id.0, asset_id);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.to_vec().into())
            .send()
            .await
            .map_err(|e| NurtureError::Infrastructure(format!("Failed to upload to S3: {}", e)))?;
        Ok(())
    }

    async fn get_asset(
        &self,
        actor_id: &ActorId,
        asset_id: &Uuid,
    ) -> Result<Vec<u8>, NurtureError> {
        let key = format!("actors/{}/{}", actor_id.0, asset_id);
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| NurtureError::Infrastructure(format!("Failed to get from S3: {}", e)))?;

        let data =
            resp.body.collect().await.map_err(|e| {
                NurtureError::Infrastructure(format!("Failed to read S3 body: {}", e))
            })?;
        Ok(data.into_bytes().to_vec())
    }

    async fn delete_assets_for_actor(&self, actor_id: &ActorId) -> Result<(), NurtureError> {
        let prefix = format!("actors/{}/", actor_id.0);

        // List all objects with the actor's prefix
        let mut continuation_token = None;
        loop {
            let mut list_request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix);
            if let Some(token) = continuation_token {
                list_request = list_request.continuation_token(token);
            }

            let response = list_request.send().await.map_err(|e| {
                let err_msg = format!("Failed to list S3 objects: {}", e);
                tracing::error!(
                    "❌ [S3AssetStorage] Error listing objects for actor {}: {}",
                    actor_id.0,
                    err_msg
                );
                NurtureError::Infrastructure(err_msg)
            })?;

            let contents = response.contents();
            if contents.is_empty() {
                break;
            }

            let mut delete_objects = vec![];
            for obj in contents {
                if let Some(key) = obj.key() {
                    delete_objects.push(
                        aws_sdk_s3::types::ObjectIdentifier::builder()
                            .key(key)
                            .build()
                            .map_err(|e| {
                                NurtureError::Infrastructure(format!("Invalid object key: {}", e))
                            })?,
                    );
                }
            }

            if !delete_objects.is_empty() {
                let delete = aws_sdk_s3::types::Delete::builder()
                    .set_objects(Some(delete_objects))
                    .build()
                    .map_err(|e| {
                        NurtureError::Infrastructure(format!(
                            "Failed to build delete request: {}",
                            e
                        ))
                    })?;
                self.client
                    .delete_objects()
                    .bucket(&self.bucket)
                    .delete(delete)
                    .send()
                    .await
                    .map_err(|e| {
                        let err_msg = format!("Failed to delete S3 objects: {}", e);
                        tracing::error!(
                            "❌ [S3AssetStorage] Error deleting objects for actor {}: {}",
                            actor_id.0,
                            err_msg
                        );
                        NurtureError::Infrastructure(err_msg)
                    })?;
            }
            continuation_token = response.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        }

        tracing::info!(
            "🗑️ [S3AssetStorage] Purged all physical assets for actor {}",
            actor_id.0
        );
        Ok(())
    }
}
