/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::voice_vault::VoiceKeyVault;
use async_trait::async_trait;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};
use std::collections::HashMap;
use std::sync::Mutex;

/// 物理的に隔離されたキーストレージ (モック実装)
/// 将来的に Abyss Security Proxy 実ストレージまたは HSM と統合
pub struct AbyssVoiceVault {
    // FIXME: MVP 用のインメモリ保管庫。実運用時は安全な外部 Vault に移譲する
    keys: Mutex<HashMap<Uuid, Zeroizing<Vec<u8>>>>,
}

impl Default for AbyssVoiceVault {
    fn default() -> Self {
        Self::new()
    }
}

impl AbyssVoiceVault {
    pub fn new() -> Self {
        Self {
            keys: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl VoiceKeyVault for AbyssVoiceVault {
    async fn fetch_decryption_key(&self, agent_id: Uuid, asset_id: Uuid) -> Result<Vec<u8>, AiomeError> {
        // 1. ライセンス検証 (Agent がこの Asset を所有しているか)
        if !self.verify_license(agent_id, asset_id).await? {
            return Err(AiomeError::SecurityViolation {
                reason: format!("Agent {} does not have the license for asset {}", agent_id, asset_id),
            });
        }

        // 2. キーの取得
        let guard = self.keys.lock().unwrap();
        if let Some(key) = guard.get(&asset_id) {
            // zeroize 対応のためクローンしてから返す
            Ok((**key).to_vec())
        } else {
            Err(AiomeError::ArtifactNotFound {
                path: format!("Decryption key for asset {}", asset_id),
            })
        }
    }

    async fn verify_license(&self, _agent_id: Uuid, _asset_id: Uuid) -> Result<bool, AiomeError> {
        // TODO: (Phase 10.2) 実際の Ledger/DB と照合して所有権を確認
        // MVP では常に true
        Ok(true)
    }

    async fn register_asset_key(&self, asset_id: Uuid, mut key: Vec<u8>) -> Result<(), AiomeError> {
        let mut guard = self.keys.lock().unwrap();

        // ゼロ埋めを保証するラッパーに包んで保管
        let zeroizing_key = Zeroizing::new(key.clone());
        guard.insert(asset_id, zeroizing_key);
        
        // 元の変数も zeroize
        key.zeroize();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_abyss_voice_vault_register_and_fetch() {
        let vault = AbyssVoiceVault::new();
        let agent_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();
        let test_key = vec![1, 2, 3, 4, 5];

        assert!(vault.register_asset_key(asset_id, test_key.clone()).await.is_ok());

        let fetched = vault.fetch_decryption_key(agent_id, asset_id).await;
        assert!(fetched.is_ok());
        assert_eq!(fetched.unwrap(), test_key);
    }

    #[tokio::test]
    async fn test_abyss_voice_vault_fetch_missing() {
        let vault = AbyssVoiceVault::new();
        let agent_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        let fetched = vault.fetch_decryption_key(agent_id, asset_id).await;
        assert!(fetched.is_err());
        if let Err(AiomeError::ArtifactNotFound { path }) = fetched {
            assert!(path.contains(&asset_id.to_string()));
        } else {
            panic!("Expected ArtifactNotFound error");
        }
    }
}
