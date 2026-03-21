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
use std::sync::{Arc, Mutex};
use crate::registry::RegistryManager;

use sqlx::SqlitePool;

use once_cell::sync::OnceCell;

/// 物理的に隔離されたキーストレージ (モック実装)
/// 将来的に Abyss Security Proxy 実ストレージまたは HSM と統合
pub struct AbyssVoiceVault {
    // FIXME: MVP 用のインメモリ保管庫。実運用時は安全な外部 Vault に移譲する
    keys: Mutex<HashMap<Uuid, Zeroizing<Vec<u8>>>>,
    master_key: OnceCell<Zeroizing<Vec<u8>>>,
    registry: Arc<RegistryManager>,
    pool: SqlitePool,
}

impl AbyssVoiceVault {
    /// 新しい Vault インスタンスを作成する
    pub fn new(registry: Arc<RegistryManager>, pool: SqlitePool) -> Self {
        Self {
            keys: Mutex::new(HashMap::new()),
            master_key: OnceCell::new(),
            registry,
            pool,
        }
    }

    /// キャッシュされた Master Key を取得 (最初の1回目のみパース)
    fn get_cached_master_key(&self) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        let key = self.master_key.get_or_try_init(|| get_master_key())?;
        Ok(key.clone())
    }

    /// 起動時に永続化された鍵をリストアする (§CISO-1)
    pub async fn restore_keys_from_db(&self) -> Result<usize, AiomeError> {
        let master = self.get_cached_master_key()?;
        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT asset_id, encrypted_key FROM vault_keys"
        ).fetch_all(&self.pool).await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("vault_keys SELECT: {}", e) })?;

        let mut guard = self.keys.lock().unwrap_or_else(|p| p.into_inner());
        let mut count = 0;
        for (id_str, encrypted) in &rows {
            let asset_id = Uuid::parse_str(&id_str).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Invalid UUID in vault_keys: {}", e),
            })?;
            let decrypted = crate::security::crypto::decrypt_aes256gcm(encrypted, &master)?;
            guard.insert(asset_id, Zeroizing::new(decrypted));
            count += 1;
        }
        tracing::info!("🔐 [Vault] Restored {} keys from persistent storage", count);
        Ok(count)
    }
}

/// Master Key 導出 (§CISO-1)
fn get_master_key() -> Result<Zeroizing<Vec<u8>>, AiomeError> {
    let key_hex = std::env::var("VAULT_MASTER_KEY")
        .map_err(|_| AiomeError::SecurityViolation {
            reason: "VAULT_MASTER_KEY environment variable is not set".into(),
        })?;
    let key = hex::decode(&key_hex).map_err(|_| AiomeError::SecurityViolation {
        reason: "VAULT_MASTER_KEY is not valid hex".into(),
    })?;
    if key.len() != 32 {
        return Err(AiomeError::SecurityViolation {
            reason: "VAULT_MASTER_KEY must be 32 bytes (64 hex chars)".into(),
        });
    }
    Ok(Zeroizing::new(key))
}

#[async_trait]
impl VoiceKeyVault for AbyssVoiceVault {
    async fn fetch_decryption_key(&self, agent_id: Uuid, asset_id: Uuid) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        // 1. ライセンス検証 (Agent がこの Asset を所有しているか)
        if !self.verify_license(agent_id, asset_id).await? {
            return Err(AiomeError::SecurityViolation {
                reason: format!("Agent {} does not have the license for asset {}", agent_id, asset_id),
            });
        }

        // 2. キーの取得
        let guard = self.keys.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(key) = guard.get(&asset_id) {
            tracing::info!(
                agent_id = %agent_id,
                asset_id = %asset_id,
                "🔓 [AuditLog] Decryption key accessed"
            );
            Ok(key.clone())
        } else {
            Err(AiomeError::ArtifactNotFound {
                path: format!("Decryption key for asset {}", asset_id),
            })
        }
    }

    async fn verify_license(&self, agent_id: Uuid, asset_id: Uuid) -> Result<bool, AiomeError> {
        // Phase 10.2: RegistryManager を通を通じて DB/Ledger の所有権を確認
        self.registry.check_ownership(agent_id, asset_id).await
    }

    async fn register_asset_key(&self, asset_id: Uuid, key: Zeroizing<Vec<u8>>) -> Result<(), AiomeError> {
        // 1. Master Key で暗号化して DB に永続化 (§CISO-1)
        let master = self.get_cached_master_key()?;
        let encrypted = crate::security::crypto::encrypt_aes256gcm(&key, &master)?;
        sqlx::query("INSERT OR REPLACE INTO vault_keys (asset_id, encrypted_key) VALUES (?, ?)")
            .bind(asset_id.to_string())
            .bind(&encrypted)
            .execute(&self.pool).await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("vault_keys INSERT: {}", e) })?;

        // 2. メモリキャッシュにも保持（高速パス）
        let mut guard = self.keys.lock().unwrap_or_else(|p| p.into_inner());
        guard.insert(asset_id, key);
        Ok(())
    }

    async fn decrypt_stream(&self, agent_id: Uuid, asset_id: Uuid, encrypted_data: &[u8]) -> Result<Vec<u8>, AiomeError> {
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
        let test_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2]; // 32 bytes

        // 環境変数をセットしてテストを通す準備
        let master_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        std::env::set_var("VAULT_MASTER_KEY", master_hex);

        // 1. 鍵の登録
        vault.register_asset_key(asset_id, Zeroizing::new(test_key.clone())).await.unwrap();

        // 2. 新しい Vault インスタンスを作成（メモリは空）
        let registry = Arc::new(RegistryManager::new(vault.pool.clone()));
        let vault2 = AbyssVoiceVault::new(registry, vault.pool.clone());
        
        // 3. 永続化ストレージからリストア
        let count = vault2.restore_keys_from_db().await.unwrap();
        assert_eq!(count, 1);

        // 4. メモリから鍵が引けることを確認（ライセンスチェックをモックするためにテーブル挿入）
        let agent_id = Uuid::new_v4();
        sqlx::query("INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)")
            .bind("evt_test")
            .bind("checkout.session.completed")
            .bind(format!(r#"{{"agent_id": "{}", "asset_id": "{}"}}"#, agent_id, asset_id))
            .execute(&vault2.pool).await.unwrap();

        let fetched = vault2.fetch_decryption_key(agent_id, asset_id).await.unwrap();
        assert_eq!(*fetched, test_key);
    }
}
