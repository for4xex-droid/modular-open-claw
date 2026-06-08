/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::abyss_voice_vault::AbyssVoiceVault;
use crate::db::DatabasePool;
use crate::registry::RegistryManager;
use aiome_core::error::AiomeError;
use aiome_core_contracts::voice_vault::VoiceKeyVault;
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

/// Phase 9: Voice Core DRM (Digital Rights Management)
///
/// ボイスアセット（TTSモデル、LoRA等）の正当な所有権を管理し、
/// Abyss Vault 経由でのキー取得と復号を行う基盤。
pub struct VoiceCoreDrm {
    /// Abyss Vault のベースURL
    pub vault_url: String,
    vault: AbyssVoiceVault,
    #[allow(dead_code)]
    registry: Arc<RegistryManager>,
}

impl VoiceCoreDrm {
    /// 新しい DRM インスタンスを生成する
    pub async fn new(
        vault_url: String,
        registry: Arc<RegistryManager>,
        pool: DatabasePool,
    ) -> Self {
        let vault = AbyssVoiceVault::new(registry.clone(), pool);
        // 起動時に永続化された鍵をリストア (§CISO-1)
        match vault.restore_keys_from_db().await {
            Ok(n) => tracing::info!("🔐 [DRM] {} vault keys restored on startup", n),
            Err(e) => tracing::error!("🚨 [DRM] Failed to restore vault keys: {:?}", e),
        }
        Self {
            vault_url,
            vault,
            registry,
        }
    }

    /// アセットの復号キーを取得する (Abyss Vault 連携)
    pub async fn fetch_decryption_key(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
    ) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        self.vault.fetch_decryption_key(agent_id, asset_id).await
    }

    /// ライセンスを検証する
    pub async fn verify_license(&self, agent_id: Uuid, asset_id: Uuid) -> Result<bool, AiomeError> {
        self.vault.verify_license(agent_id, asset_id).await
    }

    /// アセットキーを登録する
    pub async fn register_asset_key(
        &self,
        asset_id: Uuid,
        key: Zeroizing<Vec<u8>>,
    ) -> Result<(), AiomeError> {
        self.vault.register_asset_key(asset_id, key).await
    }
}
