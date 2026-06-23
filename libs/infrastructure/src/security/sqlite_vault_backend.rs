/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use crate::sql_exec;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::vault_backend::VaultBackend;
use async_trait::async_trait;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::security::mlock::MlockedVec;
use once_cell::sync::OnceCell;

/// Universal (SQLite/PostgreSQL) implementation for VaultBackend
#[derive(Debug)]
pub struct UniversalVaultBackend {
    pool: DatabasePool,
    /// デコード済み DEK の LRU キャッシュ (メモリ保護付き)
    cache: Mutex<LruCache<Uuid, MlockedVec>>,
}

impl UniversalVaultBackend {
    /// 新しい UniversalVaultBackend を作成する。
    pub fn new(pool: DatabasePool) -> Self {
        Self {
            pool,
            cache: Mutex::new(LruCache::new(match NonZeroUsize::new(1000) {
                Some(cap) => cap,
                None => {
                    tracing::error!("FATAL: Invalid LRU capacity");
                    std::process::exit(1);
                }
            })),
        }
    }

    /// テスト用: Master Key を直接注入して作成する。
    pub fn new_with_master_key(pool: DatabasePool, master_key_bytes: Vec<u8>) -> Self {
        let _ = GLOBAL_MASTER_KEY.set(MlockedVec::new(master_key_bytes));
        Self {
            pool,
            cache: Mutex::new(LruCache::new(match NonZeroUsize::new(1000) {
                Some(cap) => cap,
                None => {
                    tracing::error!("FATAL: Invalid LRU capacity");
                    std::process::exit(1);
                }
            })),
        }
    }

    /// キャッシュされた Master Key を取得
    fn get_cached_master_key(&self) -> Result<MlockedVec, AiomeError> {
        get_global_master_key()
    }

    pub async fn get_secret(&self, key: &str) -> Result<zeroize::Zeroizing<String>, AiomeError> {
        let master = self.get_cached_master_key()?;
        let q = format!(
            "SELECT encrypted_value FROM vault_secrets WHERE key = {}",
            self.pool.ph(0)
        );

        let encrypted: Option<Vec<u8>> = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(key)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(key)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?,
        };

        let encrypted = encrypted.ok_or_else(|| AiomeError::ArtifactNotFound {
            path: format!("Secret key '{}'", key),
        })?;

        let decrypted =
            crate::security::crypto::decrypt_xchacha20poly1305(&encrypted, &master.to_zeroizing())?;

        let val_str = String::from_utf8(decrypted).map_err(|e| AiomeError::SecurityViolation {
            reason: format!("Secret value is not valid UTF-8: {}", e),
        })?;

        Ok(zeroize::Zeroizing::new(val_str))
    }

    pub async fn store_secret(&self, key: &str, value: &str) -> Result<(), AiomeError> {
        let master = self.get_cached_master_key()?;
        let encrypted = crate::security::crypto::encrypt_xchacha20poly1305(
            value.as_bytes(),
            &master.to_zeroizing(),
        )?;

        let q = self
            .pool
            .upsert_query("vault_secrets", "key", &["key", "encrypted_value"], 0);

        sql_exec!(&self.pool, &q, key, &encrypted).map_err(|e| AiomeError::Infrastructure {
            reason: format!("vault_secrets INSERT: {}", e),
        })?;

        Ok(())
    }

    /// vault_secrets テーブルの全キー名を取得（値は返さない）
    pub async fn list_secret_keys(&self) -> Result<Vec<String>, AiomeError> {
        let q = "SELECT key FROM vault_secrets ORDER BY key";
        let keys: Vec<String> = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query_scalar(q).fetch_all(p).await.map_err(|e| {
                AiomeError::Infrastructure {
                    reason: format!("vault_secrets SELECT keys: {}", e),
                }
            })?,
            DatabasePool::Postgres(p) => sqlx::query_scalar(q).fetch_all(p).await.map_err(|e| {
                AiomeError::Infrastructure {
                    reason: format!("vault_secrets SELECT keys: {}", e),
                }
            })?,
        };
        Ok(keys)
    }

    /// 指定キーのシークレットを削除
    pub async fn delete_secret(&self, key: &str) -> Result<bool, AiomeError> {
        let q = format!("DELETE FROM vault_secrets WHERE key = {}", self.pool.ph(0));
        let rows_affected = sql_exec!(&self.pool, &q, key)?;
        Ok(rows_affected > 0)
    }
}

pub(crate) static GLOBAL_MASTER_KEY: OnceCell<MlockedVec> = OnceCell::new();

/// Globally retrieve the cached master key (useful for setting encryption)
pub fn get_global_master_key() -> Result<MlockedVec, AiomeError> {
    let key = GLOBAL_MASTER_KEY.get_or_try_init(get_master_key)?;
    Ok(key.clone())
}

/// Master Key 導出 (§CISO-1)
fn get_master_key() -> Result<MlockedVec, AiomeError> {
    let password = shared::security::get_keychain_secret("com.aiome.vault-master-password")
        .or_else(|| std::env::var("VAULT_MASTER_PASSWORD").ok())
        .ok_or_else(|| AiomeError::SecurityViolation {
            reason: "VAULT_MASTER_PASSWORD must be set in macOS Keychain or environment".into(),
        })?;
    shared::security::scrub_env("VAULT_MASTER_PASSWORD");

    // Salt は固定値を使用するか、コンフィグから読み込む。ここではハードコードの System Salt を使用
    let salt = b"aiome-system-vault-salt-v1-xchacha";
    let key = crate::security::crypto::derive_master_key_argon2id(&password, salt)?;

    Ok(MlockedVec::new(key.to_vec()))
}

#[async_trait]
impl VaultBackend for UniversalVaultBackend {
    async fn get_dek(&self, asset_id: Uuid) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        // 1. キャッシュから取得を試みる (§CISO-1)
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(mlocked_key) = cache.get(&asset_id) {
                return Ok(mlocked_key.to_zeroizing());
            }
        }

        // 2. DB から取得
        let master = self.get_cached_master_key()?;
        let asset_id_str = asset_id.to_string();
        let q = format!(
            "SELECT encrypted_key FROM vault_keys WHERE asset_id = {}",
            self.pool.ph(0)
        );

        let encrypted: Option<Vec<u8>> = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(&asset_id_str)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(&asset_id_str)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };

        let encrypted = encrypted.ok_or_else(|| AiomeError::ArtifactNotFound {
            path: format!("Decryption key for asset {}", asset_id),
        })?;

        let decrypted =
            crate::security::crypto::decrypt_xchacha20poly1305(&encrypted, &master.to_zeroizing())?;

        // 3. キャッシュに保存
        let mlocked_key = MlockedVec::new(decrypted);
        let result = mlocked_key.to_zeroizing();

        if let Ok(mut cache) = self.cache.lock() {
            cache.put(asset_id, mlocked_key);
        }

        Ok(result)
    }

    async fn store_dek(&self, asset_id: Uuid, dek: &[u8]) -> Result<(), AiomeError> {
        let master = self.get_cached_master_key()?;
        let encrypted =
            crate::security::crypto::encrypt_xchacha20poly1305(dek, &master.to_zeroizing())?;

        let q = self
            .pool
            .upsert_query("vault_keys", "asset_id", &["asset_id", "encrypted_key"], 0);

        sql_exec!(&self.pool, &q, asset_id.to_string(), &encrypted).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("vault_keys INSERT: {}", e),
            }
        })?;

        // キャッシュに保存/更新 (§CISO-1)
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(asset_id, MlockedVec::new(dek.to_vec()));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<(), AiomeError> {
        self.get_cached_master_key()?;
        let q = "SELECT 1";
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(q)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(q)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_exec;

    async fn setup_db() -> DatabasePool {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let schema =
            "CREATE TABLE vault_keys (asset_id TEXT PRIMARY KEY, encrypted_key BLOB NOT NULL)";
        sql_exec!(&pool, schema).unwrap();
        pool
    }

    #[tokio::test]
    async fn test_store_and_get_dek() {
        let pool = setup_db().await;
        let backend = UniversalVaultBackend::new_with_master_key(pool, vec![0u8; 32]);

        let asset_id = Uuid::new_v4();
        let dek = vec![1, 2, 3, 4, 5];

        let result = backend.get_dek(asset_id).await;
        assert!(matches!(result, Err(AiomeError::ArtifactNotFound { .. })));

        backend.store_dek(asset_id, &dek).await.unwrap();

        let retrieved = backend.get_dek(asset_id).await.unwrap();
        assert_eq!(*retrieved, dek);

        backend.cache.lock().unwrap().clear();
        let retrieved_db = backend.get_dek(asset_id).await.unwrap();
        assert_eq!(*retrieved_db, dek);
    }

    async fn setup_secrets_db() -> DatabasePool {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let schema =
            "CREATE TABLE vault_secrets (key TEXT PRIMARY KEY, encrypted_value BLOB NOT NULL)";
        sql_exec!(&pool, schema).unwrap();
        pool
    }

    #[tokio::test]
    async fn test_store_and_get_secret() {
        let pool = setup_secrets_db().await;
        let backend = UniversalVaultBackend::new_with_master_key(pool, vec![0u8; 32]);

        let key = "TEST_SECRET_KEY";
        let val = "super_secret_value_12345";

        backend.store_secret(key, val).await.unwrap();

        let retrieved = backend.get_secret(key).await.unwrap();
        assert_eq!(&*retrieved, val);
    }

    #[tokio::test]
    async fn test_list_secret_keys() {
        let pool = setup_secrets_db().await;
        let backend = UniversalVaultBackend::new_with_master_key(pool, vec![0u8; 32]);

        backend.store_secret("KEY_A", "valA").await.unwrap();
        backend.store_secret("KEY_B", "valB").await.unwrap();

        let keys = backend.list_secret_keys().await.unwrap();
        assert_eq!(keys, vec!["KEY_A".to_string(), "KEY_B".to_string()]);
    }

    #[tokio::test]
    async fn test_delete_secret() {
        let pool = setup_secrets_db().await;
        let backend = UniversalVaultBackend::new_with_master_key(pool, vec![0u8; 32]);

        backend.store_secret("KEY_DELETE", "val").await.unwrap();

        // 削除成功を確認
        let deleted = backend.delete_secret("KEY_DELETE").await.unwrap();
        assert!(deleted);

        // 存在しないキーの削除を確認
        let deleted_nonexistent = backend.delete_secret("KEY_DELETE").await.unwrap();
        assert!(!deleted_nonexistent);
    }
}
