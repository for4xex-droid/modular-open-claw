/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::SqlitePool;

/// アセット種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    /// TTS向けボイスモデル
    VoiceModel,
    /// LoRA等の追加学習モデル
    LoRA,
    /// その他のスキル・プラグイン
    Plugin,
}

impl AsRef<str> for AssetType {
    fn as_ref(&self) -> &str {
        match self {
            AssetType::VoiceModel => "voice",
            AssetType::LoRA => "lora",
            AssetType::Plugin => "plugin",
        }
    }
}

/// Registry 用の Asset メタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub asset_type: AssetType,
    pub name: String,
    pub description: String,
    pub price_coins: u64,
}

/// Phase 10: Registry
///
/// クリエイターがアップロードしたアセット（ボイス、LoRA等）のインデックスと
/// マニフェスト（SkillManifestの上位層）を管理するシステム。
pub struct RegistryManager {
    pool: SqlitePool,
}

impl RegistryManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// アセットのメタデータを登録する
    pub async fn register_asset(&self, manifest: AssetManifest) -> Result<(), AiomeError> {
        let type_str = manifest.asset_type.as_ref();
        
        let result = sqlx::query(
            r#"
            INSERT INTO asset_registry (id, creator_id, asset_type, name, description, price_coins)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(manifest.id.to_string())
        .bind(manifest.creator_id.to_string())
        .bind(type_str)
        .bind(&manifest.name)
        .bind(&manifest.description)
        .bind(manifest.price_coins as i64)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                tracing::info!("📦 [Registry] Successfully registered asset: {}", manifest.id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ [Registry] Failed to register asset: {}", e);
                Err(AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }

    /// アセットのメタデータを取得する
    pub async fn get_asset(&self, asset_id: Uuid) -> Result<AssetManifest, AiomeError> {
        // FIXME: MVP なので簡易的なモック実装をフォールバックに使うか、DBから引くか
        // 現時点では DB 用意前につきスタブ
        Ok(AssetManifest {
            id: asset_id,
            creator_id: Default::default(),
            asset_type: AssetType::VoiceModel,
            name: "Dummy Asset".into(),
            description: "Not Implemented".into(),
            price_coins: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db_for_registry() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE asset_registry (
                id TEXT PRIMARY KEY,
                creator_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                price_coins INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_registry_register_asset() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        
        let manifest = AssetManifest {
            id: Uuid::new_v4(),
            creator_id: Uuid::new_v4(),
            asset_type: AssetType::VoiceModel,
            name: "Premium Voice".into(),
            description: "High quality voice model".into(),
            price_coins: 500,
        };

        assert!(registry.register_asset(manifest).await.is_ok());
    }
}
