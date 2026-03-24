use aiome_contracts::error::AiomeError;
use aiome_contracts::vault_backend::VaultBackend;
use async_trait::async_trait;
use lru::LruCache;
use sqlx::SqlitePool;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::security::mlock::MlockedVec;

use once_cell::sync::OnceCell;

/// SQLite をストレージとして使用し、環境変数の Master Key で DEK を保護する Backend 実装。
/// Phase A 移行用。
#[derive(Debug)]
pub struct SqliteVaultBackend {
    pool: SqlitePool,
    master_key: OnceCell<MlockedVec>,
    /// デコード済み DEK の LRU キャッシュ (メモリ保護付き)
    cache: Mutex<LruCache<Uuid, MlockedVec>>,
}

impl SqliteVaultBackend {
    /// 新しい SqliteVaultBackend を作成する。
    /// Master Key は `VAULT_MASTER_KEY` 環境変数から遅延取得される。
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            master_key: OnceCell::new(),
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
        }
    }

    /// テスト用: Master Key を直接注入して SqliteVaultBackend を作成する。
    /// `std::env::set_var` を使わずにテストを実行するためのコンストラクタ。
    #[cfg(test)]
    pub fn new_with_master_key(pool: SqlitePool, master_key_bytes: Vec<u8>) -> Self {
        let cell = OnceCell::new();
        let _ = cell.set(MlockedVec::new(master_key_bytes));
        Self {
            pool,
            master_key: cell,
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
        }
    }

    /// キャッシュされた Master Key を取得 (最初の1回目のみパース)
    fn get_cached_master_key(&self) -> Result<MlockedVec, AiomeError> {
        let key = self.master_key.get_or_try_init(get_master_key)?;
        Ok(key.clone())
    }
}

/// Master Key 導出 (§CISO-1)
fn get_master_key() -> Result<MlockedVec, AiomeError> {
    let key_hex = std::env::var("VAULT_MASTER_KEY").map_err(|_| AiomeError::SecurityViolation {
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
    Ok(MlockedVec::new(key))
}

#[async_trait]
impl VaultBackend for SqliteVaultBackend {
    async fn get_dek(&self, asset_id: Uuid) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        // 1. キャッシュから取得を試みる (§CISO-1)
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(mlocked_key) = cache.get(&asset_id) {
                return Ok(mlocked_key.to_zeroizing());
            }
        }

        // 2. DB から取得
        let master = self.get_cached_master_key()?;
        let (encrypted,): (Vec<u8>,) =
            sqlx::query_as("SELECT encrypted_key FROM vault_keys WHERE asset_id = ?")
                .bind(asset_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("vault_keys SELECT: {}", e),
                })?
                .ok_or_else(|| AiomeError::ArtifactNotFound {
                    path: format!("Decryption key for asset {}", asset_id),
                })?;

        let decrypted =
            crate::security::crypto::decrypt_aes256gcm(&encrypted, &master.to_zeroizing())?;

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
        let encrypted = crate::security::crypto::encrypt_aes256gcm(dek, &master.to_zeroizing())?;

        sqlx::query("INSERT OR REPLACE INTO vault_keys (asset_id, encrypted_key) VALUES (?, ?)")
            .bind(asset_id.to_string())
            .bind(&encrypted)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("vault_keys INSERT: {}", e),
            })?;

        // キャッシュに保存/更新 (§CISO-1)
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(asset_id, MlockedVec::new(dek.to_vec()));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<(), AiomeError> {
        // Master Key が読み込めるか、DBが生きているか確認
        self.get_cached_master_key()?;
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("vault_keys Health Check: {}", e),
            })?;
        Ok(())
    }
}
