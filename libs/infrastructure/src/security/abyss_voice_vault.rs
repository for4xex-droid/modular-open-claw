/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::registry::RegistryManager;
use crate::security::mlock::MlockedVec;
use aiome_contracts::error::AiomeError;
use aiome_contracts::voice_vault::VoiceKeyVault;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use sqlx::SqlitePool;

use crate::security::sqlite_vault_backend::SqliteVaultBackend;
use aiome_contracts::vault_backend::VaultBackend;

/// 物理的に隔離されたキーストレージ (モック実装)
/// 将来的に Abyss Security Proxy 実ストレージまたは HSM と統合
pub struct AbyssVoiceVault {
    backend: Arc<dyn VaultBackend>,
    registry: Arc<RegistryManager>,
}

impl AbyssVoiceVault {
    /// 新しい Vault インスタンスを作成する
    pub fn new(registry: Arc<RegistryManager>, pool: SqlitePool) -> Self {
        Self {
            backend: Arc::new(SqliteVaultBackend::new(pool)),
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

        // Phase 10.2: AES-256-GCM 復号の実行 (§SEC-1)
        crate::security::crypto::decrypt_aes256gcm(encrypted_data, &key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_vault() -> AbyssVoiceVault {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // 必要テーブル作成
        sqlx::query(
            "CREATE TABLE vault_keys (asset_id TEXT PRIMARY KEY, encrypted_key BLOB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT, processed_at DATETIME DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        let registry = Arc::new(RegistryManager::new(pool.clone()));
        AbyssVoiceVault::new(registry, pool)
    }

    #[tokio::test]
    async fn test_abyss_voice_vault_persistence_roundtrip() {
        // RED test: VAULT_MASTER_KEY is not set, this should fail.
        let vault = setup_vault().await;
        let asset_id = Uuid::new_v4();
        let test_key = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 1, 2,
        ]; // 32 bytes

        // 環境変数をセットしてテストを通す準備
        let master_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        std::env::set_var("VAULT_MASTER_KEY", master_hex);

        let pool_backup = vault.backend.clone(); // (Used strictly for pool simulation down below if needed, but not heavily accessible now that backend is hidden)

        // 1. 鍵の登録
        vault
            .register_asset_key(asset_id, Zeroizing::new(test_key.clone()))
            .await
            .unwrap();

        // 2. 新しい Vault インスタンスを作成（復元の確認）
        // VaultBackend により、restore_keys_from_db を呼ばずとも on-demand fetch で取得可能。
        let registry_clone = Arc::new(RegistryManager::new(
            // Recovering pool from setup for test purpose
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(),
        )); // In real test, pass the same pool. Let's just create a secondary setup that points to same db.

        let vault2 = setup_vault().await;

        // Let's actually share the pool for the test validation since sqlite::memory is per-pool.
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // 必要テーブル作成
        sqlx::query(
            "CREATE TABLE vault_keys (asset_id TEXT PRIMARY KEY, encrypted_key BLOB NOT NULL, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT, processed_at DATETIME DEFAULT CURRENT_TIMESTAMP)"
        ).execute(&pool).await.unwrap();

        let registry = Arc::new(RegistryManager::new(pool.clone()));
        let shared_vault1 = AbyssVoiceVault::new(registry.clone(), pool.clone());
        let shared_vault2 = AbyssVoiceVault::new(registry, pool.clone());

        shared_vault1
            .register_asset_key(asset_id, Zeroizing::new(test_key.clone()))
            .await
            .unwrap();

        // 3. ライセンスチェックをモックするためにテーブル挿入
        let agent_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)",
        )
        .bind("evt_test")
        .bind("checkout.session.completed")
        .bind(format!(
            r#"{{"agent_id": "{}", "asset_id": "{}"}}"#,
            agent_id, asset_id
        ))
        .execute(&pool)
        .await
        .unwrap();

        let fetched = shared_vault2
            .fetch_decryption_key(agent_id, asset_id)
            .await
            .unwrap();
        assert_eq!(*fetched, test_key);
    }
}
