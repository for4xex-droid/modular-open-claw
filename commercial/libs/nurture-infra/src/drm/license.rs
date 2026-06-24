/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_bridge::db::{DatabasePool, DatabaseTransaction};
use nurture_bridge::error::AiomeError;
use nurture_bridge::{sql_exec, sql_fetch_optional_map, sql_tx_exec};
use nurture_core::license::{AssetLicense, LicenseStore};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use secrecy::{ExposeSecret, Secret};

pub struct SQLiteLicenseStore {
    pool: DatabasePool,
    master_key: Secret<[u8; 32]>,
}

impl SQLiteLicenseStore {
    pub fn new(pool: DatabasePool, master_key_seed: &secrecy::SecretString) -> Self {
        let master_key: [u8; 32] =
            Sha256::digest(master_key_seed.expose_secret().as_bytes()).into();
        Self {
            pool,
            master_key: Secret::new(master_key),
        }
    }

    fn encrypt_key(&self, plaintext: &str) -> Result<String, NurtureError> {
        let cipher = ChaCha20Poly1305::new(&(*self.master_key.expose_secret()).into());
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| NurtureError::Infrastructure(format!("Encryption failed: {}", e)))?;

        // Format: b64_nonce:b64_ciphertext
        let b64_nonce = BASE64_STANDARD.encode(nonce_bytes);
        let b64_cipher = BASE64_STANDARD.encode(ciphertext);

        Ok(format!("{}:{}", b64_nonce, b64_cipher))
    }

    fn decrypt_key(&self, encrypted: &str) -> Result<String, NurtureError> {
        let parts: Vec<&str> = encrypted.split(':').collect();
        if parts.len() != 2 {
            // Legacy plaintext format migration (or error)
            return Ok(encrypted.to_string());
        }

        let nonce_bytes = BASE64_STANDARD
            .decode(parts[0])
            .map_err(|_| NurtureError::Infrastructure("Invalid nonce b64".into()))?;
        let ciphertext = BASE64_STANDARD
            .decode(parts[1])
            .map_err(|_| NurtureError::Infrastructure("Invalid cipher b64".into()))?;

        if nonce_bytes.len() != 12 {
            return Err(NurtureError::Infrastructure("Invalid nonce len".into()));
        }

        let cipher = ChaCha20Poly1305::new(&(*self.master_key.expose_secret()).into());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| NurtureError::Infrastructure("Decryption failed".into()))?;

        String::from_utf8(plaintext)
            .map_err(|_| NurtureError::Infrastructure("Invalid UTF-8 in key".into()))
    }
}

#[async_trait]
impl LicenseStore for SQLiteLicenseStore {
    async fn issue_license(&self, license: &AssetLicense) -> Result<(), NurtureError> {
        let encrypted_key = self.encrypt_key(&license.decryption_key)?;

        sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO nurture_licenses (id, transaction_id, asset_id, owner_id, decryption_key, issued_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            pg: "INSERT INTO nurture_licenses (id, transaction_id, asset_id, owner_id, decryption_key, issued_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            license.id.to_string(),
            license.transaction_id.to_string(),
            license.asset_id.to_string(),
            license.owner_id.0.to_string(),
            encrypted_key,
            license.issued_at,
            license.expires_at
        )
        .map(|_| ())
        .map_err(|e| NurtureError::Infrastructure(format!("ライセンス発行失敗: {}", e)))
    }

    async fn get_license(
        &self,
        owner: &ActorId,
        asset_id: &Uuid,
    ) -> Result<Option<AssetLicense>, NurtureError> {
        struct RowData {
            id: String,
            tx_id: String,
            asset_id_str: String,
            owner_id_str: String,
            encrypted_key: String,
            issued_at: chrono::DateTime<chrono::Utc>,
            expires_at: Option<chrono::DateTime<chrono::Utc>>,
            revoked_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let res: Result<Option<RowData>, AiomeError> = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT id, transaction_id, asset_id, owner_id, decryption_key, issued_at, expires_at, revoked_at \
                     FROM nurture_licenses \
                     WHERE owner_id = ? AND asset_id = ? AND revoked_at IS NULL \
                     AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP) \
                     ORDER BY issued_at DESC LIMIT 1",
            |row| {
                Ok::<RowData, AiomeError>(RowData {
                    id: row.get("id"),
                    tx_id: row.get("transaction_id"),
                    asset_id_str: row.get("asset_id"),
                    owner_id_str: row.get("owner_id"),
                    encrypted_key: row.get("decryption_key"),
                    issued_at: row.get("issued_at"),
                    expires_at: row.get("expires_at"),
                    revoked_at: row.get("revoked_at"),
                })
            },
            pg: "SELECT id, transaction_id, asset_id, owner_id, decryption_key, issued_at, expires_at, revoked_at \
                 FROM nurture_licenses \
                 WHERE owner_id = $1 AND asset_id = $2 AND revoked_at IS NULL \
                 AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP) \
                 ORDER BY issued_at DESC LIMIT 1",
            |row| {
                Ok::<RowData, AiomeError>(RowData {
                    id: row.get("id"),
                    tx_id: row.get("transaction_id"),
                    asset_id_str: row.get("asset_id"),
                    owner_id_str: row.get("owner_id"),
                    encrypted_key: row.get("decryption_key"),
                    issued_at: row.get("issued_at"),
                    expires_at: row.get("expires_at"),
                    revoked_at: row.get("revoked_at"),
                })
            },
            owner.0.to_string(),
            asset_id.to_string()
        );

        let row_opt =
            res.map_err(|e| NurtureError::Infrastructure(format!("ライセンス取得失敗: {}", e)))?;

        match row_opt {
            Some(row) => {
                let plaintext_key = self.decrypt_key(&row.encrypted_key)?;

                Ok(Some(AssetLicense {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|_| NurtureError::Infrastructure("ID parse error".into()))?,
                    transaction_id: Uuid::parse_str(&row.tx_id)
                        .map_err(|_| NurtureError::Infrastructure("TX ID parse error".into()))?,
                    asset_id: Uuid::parse_str(&row.asset_id_str)
                        .map_err(|_| NurtureError::Infrastructure("Asset ID parse error".into()))?,
                    owner_id: ActorId(Uuid::parse_str(&row.owner_id_str).map_err(|_| {
                        NurtureError::Infrastructure("Owner ID parse error".into())
                    })?),
                    decryption_key: plaintext_key,
                    issued_at: row.issued_at,
                    expires_at: row.expires_at,
                    revoked_at: row.revoked_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn revoke_license(&self, license_id: &Uuid) -> Result<(), NurtureError> {
        sql_exec!(
            &self.pool,
            sqlite: "UPDATE nurture_licenses SET revoked_at = CURRENT_TIMESTAMP WHERE id = ?",
            pg: "UPDATE nurture_licenses SET revoked_at = CURRENT_TIMESTAMP WHERE id = $1",
            license_id.to_string()
        )
        .map(|_| ())
        .map_err(|e| NurtureError::Infrastructure(format!("ライセンス取消失敗: {}", e)))
    }

    async fn transfer_license(
        &self,
        old_license_id: &Uuid,
        new_license: &AssetLicense,
    ) -> Result<(), NurtureError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            NurtureError::Infrastructure(format!("Transaction begin failed: {}", e))
        })?;

        Self::transfer_license_internal(&mut tx, &self.master_key, old_license_id, new_license)
            .await?;

        tx.commit().await.map_err(|e| {
            NurtureError::Infrastructure(format!("Transaction commit failed: {}", e))
        })?;

        Ok(())
    }
    async fn purge_expired_licenses(&self) -> Result<u64, NurtureError> {
        sql_exec!(
            &self.pool,
            sqlite: "DELETE FROM nurture_licenses WHERE (expires_at IS NOT NULL AND expires_at < CURRENT_TIMESTAMP) OR revoked_at IS NOT NULL",
            pg: "DELETE FROM nurture_licenses WHERE (expires_at IS NOT NULL AND expires_at < CURRENT_TIMESTAMP) OR revoked_at IS NOT NULL"
        )
        .map_err(|e| {
            NurtureError::Infrastructure(format!("期限切れライセンスのパージ失敗: {}", e))
        })
    }
}

impl SQLiteLicenseStore {
    /// 内部利用・トランザクション(UoW)用の移転ロジック
    pub(crate) async fn transfer_license_internal(
        tx: &mut DatabaseTransaction<'_>,
        master_key: &Secret<[u8; 32]>,
        old_license_id: &Uuid,
        new_license: &AssetLicense,
    ) -> Result<(), NurtureError> {
        let query_revoke = "UPDATE nurture_licenses SET revoked_at = CURRENT_TIMESTAMP WHERE id = ? AND revoked_at IS NULL";
        let query_revoke_pg = "UPDATE nurture_licenses SET revoked_at = CURRENT_TIMESTAMP WHERE id = $1 AND revoked_at IS NULL";

        // 1. 古いライセンスを revoke (UPDATE)
        let rows_affected = sql_tx_exec!(
            tx,
            sqlite: query_revoke,
            pg: query_revoke_pg,
            old_license_id.to_string()
        )
        .map_err(|e| {
            NurtureError::Infrastructure(format!("ライセンス取消クエリ実行失敗: {}", e))
        })?;

        if rows_affected == 0 {
            return Err(NurtureError::Infrastructure(format!(
                "ライセンス無効化に失敗しました。対象のライセンスが存在しないか、既に無効化されています: {}",
                old_license_id
            )));
        }

        // 2. 新しいライセンスを発行 (INSERT)
        // ⚠️ encrypt_key / decrypt_key と同一フォーマット (b64_nonce:b64_ciphertext) を使用すること
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            ChaCha20Poly1305, Nonce,
        };
        use rand::rngs::OsRng;
        use rand::RngCore;
        use secrecy::ExposeSecret;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = ChaCha20Poly1305::new(&(*master_key.expose_secret()).into());
        let ciphertext = cipher
            .encrypt(nonce, new_license.decryption_key.as_bytes())
            .map_err(|e| {
                NurtureError::Infrastructure(format!("鍵の暗号化に失敗しました: {}", e))
            })?;

        // Format: b64_nonce:b64_ciphertext (encrypt_key と同一形式)
        let b64_nonce = BASE64.encode(nonce_bytes);
        let b64_cipher = BASE64.encode(&ciphertext);
        let encrypted_key = format!("{}:{}", b64_nonce, b64_cipher);

        let query_insert = "INSERT INTO nurture_licenses (id, transaction_id, asset_id, owner_id, decryption_key, issued_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)";
        let query_insert_pg = "INSERT INTO nurture_licenses (id, transaction_id, asset_id, owner_id, decryption_key, issued_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)";

        sql_tx_exec!(
            tx,
            sqlite: query_insert,
            pg: query_insert_pg,
            new_license.id.to_string(),
            new_license.transaction_id.to_string(),
            new_license.asset_id.to_string(),
            new_license.owner_id.0.to_string(),
            encrypted_key,
            new_license.issued_at,
            new_license.expires_at
        )
        .map_err(|e| NurtureError::Infrastructure(format!("ライセンス発行失敗: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use nurture_core::license::AssetLicense;

    #[tokio::test]
    async fn test_license_key_encryption() {
        let sqlite_pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Setup schema
        sqlx::query(
            "CREATE TABLE nurture_licenses (
                id TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                decryption_key TEXT NOT NULL,
                issued_at TIMESTAMP NOT NULL,
                expires_at TIMESTAMP,
                revoked_at TIMESTAMP
            )",
        )
        .execute(&sqlite_pool)
        .await
        .unwrap();

        let pool = DatabasePool::Sqlite(sqlite_pool.clone());
        let master_key_seed =
            secrecy::SecretString::from("test_super_secret_seed_phrase".to_string());
        let store = SQLiteLicenseStore::new(pool, &master_key_seed);

        let license = AssetLicense {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: Uuid::new_v4(),
            owner_id: ActorId(Uuid::new_v4()),
            decryption_key: "super_secret_drm_key_12345".to_string(),
            issued_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };

        // Act - Issue license
        store.issue_license(&license).await.unwrap();

        // Assert - Check DB to ensure it's NOT plaintext
        let row = sqlx::query("SELECT decryption_key FROM nurture_licenses WHERE id = ?")
            .bind(license.id.to_string())
            .fetch_one(&sqlite_pool)
            .await
            .unwrap();
        let db_key: String = row.get("decryption_key");
        assert_ne!(
            db_key, "super_secret_drm_key_12345",
            "DRM KEY IS STORED IN PLAINTEXT!"
        );
        assert!(
            db_key.contains(':'),
            "Expected b64_nonce:b64_ciphertext format"
        );

        // Act - Get license and verify decryption
        let fetched = store
            .get_license(&license.owner_id, &license.asset_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            fetched.decryption_key, "super_secret_drm_key_12345",
            "Failed to decrypt DRM key"
        );
    }
}
