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
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(1000).expect("Capacity must be > 0"), // allow-anti-pattern
            )), // allow-anti-pattern
        }
    }

    /// テスト用: Master Key を直接注入して作成する。
    #[cfg(test)]
    pub fn new_with_master_key(pool: DatabasePool, master_key_bytes: Vec<u8>) -> Self {
        let _ = GLOBAL_MASTER_KEY.set(MlockedVec::new(master_key_bytes));
        Self {
            pool,
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(1000).expect("Capacity must be > 0"), // allow-anti-pattern
            )),
        }
    }

    /// キャッシュされた Master Key を取得
    fn get_cached_master_key(&self) -> Result<MlockedVec, AiomeError> {
        get_global_master_key()
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
    let password =
        std::env::var("VAULT_MASTER_PASSWORD").map_err(|_| AiomeError::SecurityViolation {
            reason: "VAULT_MASTER_PASSWORD environment variable is not set".into(),
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
