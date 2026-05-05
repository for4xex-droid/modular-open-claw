/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::registry::RegistryManager;
use crate::security::mlock::MlockedVec;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::voice_vault::VoiceKeyVault;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use sqlx::SqlitePool;

use crate::db::DatabasePool;
use crate::security::sqlite_vault_backend::UniversalVaultBackend;
use aiome_core_contracts::vault_backend::VaultBackend;

/// 物理的に隔離されたキーストレージ (モック実装)
/// 将来的に Abyss Security Proxy 実ストレージまたは HSM と統合
pub struct AbyssVoiceVault {
    backend: Arc<dyn VaultBackend>,
    registry: Arc<RegistryManager>,
}

impl AbyssVoiceVault {
    /// 新しい Vault インスタンスを作成する
    pub fn new(registry: Arc<RegistryManager>, pool: DatabasePool) -> Self {
        Self {
            backend: Arc::new(UniversalVaultBackend::new(pool)),
            registry,
        }
    }

    /// テスト用: Master Key を直接注入して Vault を作成する。
    /// `std::env::set_var` を使わず安全にテストを実行するためのコンストラクタ。
    #[cfg(test)]
    pub fn new_with_master_key(
        registry: Arc<RegistryManager>,
        pool: DatabasePool,
        master_key_bytes: Vec<u8>,
    ) -> Self {
        Self {
            backend: Arc::new(UniversalVaultBackend::new_with_master_key(
                pool,
                master_key_bytes,
            )),
            registry,
        }
    }

    /// 起動時に永続化された鍵をリストアする (§CISO-1)
    /// Phase B 移行準備: オンデマンド取得化により、本メソッドの返り値は 0 となり、やがて廃止される。
    pub async fn restore_keys_from_db(&self) -> Result<usize, AiomeError> {
        tracing::info!("🔐 [Vault] restore_keys_from_db is now deprecated with VaultBackend.");
        Ok(0)
    }
}

#[async_trait]
impl VoiceKeyVault for AbyssVoiceVault {
    async fn fetch_decryption_key(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
    ) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        // 1. ライセンス検証 (Agent がこの Asset を所有しているか)
        if !self.verify_license(agent_id, asset_id).await? {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Agent {} does not have the license for asset {}",
                    agent_id, asset_id
                ),
            });
        }

        // 2. キーの取得 (Backend)
        let key = self.backend.get_dek(asset_id).await?;
        tracing::info!(
            asset_id = %asset_id,
            "🔓 [AuditLog] Decryption key accessed"
        );
        Ok(key)
    }

    async fn verify_license(&self, agent_id: Uuid, asset_id: Uuid) -> Result<bool, AiomeError> {
        // Phase 10.2: RegistryManager を通じて DB/Ledger の所有権を確認
        self.registry.check_ownership(agent_id, asset_id).await
    }

    async fn register_asset_key(
        &self,
        asset_id: Uuid,
        key: Zeroizing<Vec<u8>>,
    ) -> Result<(), AiomeError> {
        // Backend に登録を委譲
        self.backend.store_dek(asset_id, &key).await
    }

    async fn decrypt_stream(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
        encrypted_data: &[u8],
    ) -> Result<Vec<u8>, AiomeError> {
        let key = self.fetch_decryption_key(agent_id, asset_id).await?;

        // Phase 10.2: XChaCha20Poly1305 復号の実行 (§SEC-1)
        crate::security::crypto::decrypt_xchacha20poly1305(encrypted_data, &key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Master Key (32 bytes) のテスト用 hex 表現
    const TEST_MASTER_KEY_HEX: &str =
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    /// テスト用 master key のバイト列を返す
    fn test_master_key_bytes() -> Vec<u8> {
        hex::decode(TEST_MASTER_KEY_HEX).unwrap()
    }

    /// 共通セットアップ: メモリ DB + テーブル作成 + master key 注入済み Vault を返す
    async fn setup_vault_with_pool() -> (AbyssVoiceVault, SqlitePool) {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE vault_keys (asset_id TEXT PRIMARY KEY, encrypted_key BLOB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT, processed_at DATETIME DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE licenses (id TEXT PRIMARY KEY, agent_id TEXT NOT NULL, asset_id TEXT NOT NULL, original_event_id TEXT, status TEXT NOT NULL DEFAULT 'active', granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        let registry = Arc::new(RegistryManager::new(DatabasePool::Sqlite(pool.clone())));
        let vault = AbyssVoiceVault::new_with_master_key(
            registry,
            DatabasePool::Sqlite(pool.clone()),
            test_master_key_bytes(),
        );
        (vault, pool)
    }

    #[tokio::test]
    async fn test_abyss_voice_vault_persistence_roundtrip() {
        let (vault1, pool) = setup_vault_with_pool().await;
        let asset_id = Uuid::new_v4();
        let test_key = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]; // 32 bytes

        // 1. 鍵の登録
        vault1
            .register_asset_key(asset_id, Zeroizing::new(test_key.clone()))
            .await
            .unwrap();

        // 2. 同一プールを共有する別 Vault インスタンスを作成
        let registry2 = Arc::new(RegistryManager::new(DatabasePool::Sqlite(pool.clone())));
        let vault2 = AbyssVoiceVault::new_with_master_key(
            registry2.clone(),
            DatabasePool::Sqlite(pool.clone()),
            test_master_key_bytes(),
        );

        // 3. ライセンスの付与 (Wave 3: Registry を通じて正規に付与)
        let agent_id = Uuid::new_v4();
        registry2
            .grant_license(agent_id, asset_id, "evt_test_grant".to_string())
            .await
            .unwrap();

        // 4. 別 Vault から鍵を取得し、元と一致することを確認
        let fetched = vault2
            .fetch_decryption_key(agent_id, asset_id)
            .await
            .unwrap();
        assert_eq!(*fetched, test_key);
    }
}
