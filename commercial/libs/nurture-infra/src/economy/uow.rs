use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use nurture_core::ledger::LedgerEntry;
use nurture_core::license::AssetLicense;
use nurture_core::uow::{CommerceUow, UowManager};
use secrecy::Secret;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteUowManager {
    pool: SqlitePool,
    master_key: Secret<[u8; 32]>,
}

impl SqliteUowManager {
    pub fn new(pool: SqlitePool, master_key_seed: &secrecy::SecretString) -> Self {
        use secrecy::ExposeSecret;
        use sha2::{Digest, Sha256};
        let master_key: [u8; 32] =
            Sha256::digest(master_key_seed.expose_secret().as_bytes()).into();
        Self {
            pool,
            master_key: Secret::new(master_key),
        }
    }
}

#[async_trait]
impl UowManager for SqliteUowManager {
    async fn begin_uow(&self) -> Result<Box<dyn CommerceUow>, NurtureError> {
        let tx = self.pool.begin().await.map_err(|e| {
            NurtureError::Infrastructure(format!("Transaction begin failed: {}", e))
        })?;
        Ok(Box::new(SqliteCommerceUow {
            tx: Some(tx),
            master_key: self.master_key.clone(),
        }))
    }
}

pub struct SqliteCommerceUow {
    tx: Option<sqlx::Transaction<'static, sqlx::Sqlite>>,
    master_key: Secret<[u8; 32]>,
}

#[async_trait]
impl CommerceUow for SqliteCommerceUow {
    async fn transfer_license(
        &mut self,
        old_license_id: &Uuid,
        new_license: &AssetLicense,
    ) -> Result<(), NurtureError> {
        let tx = self.tx.as_mut().ok_or_else(|| {
            NurtureError::Infrastructure("Transaction already committed or rolled back".to_string())
        })?;

        crate::drm::license::SQLiteLicenseStore::transfer_license_internal(
            tx,
            &self.master_key,
            old_license_id,
            new_license,
        )
        .await
    }

    async fn record_batch(&mut self, entries: &[LedgerEntry]) -> Result<(), NurtureError> {
        let tx = self.tx.as_mut().ok_or_else(|| {
            NurtureError::Infrastructure("Transaction already committed or rolled back".to_string())
        })?;

        crate::economy::ledger::SQLiteEconomyLedger::record_batch_internal(tx, entries).await
    }

    async fn commit(mut self: Box<Self>) -> Result<(), NurtureError> {
        if let Some(tx) = self.tx.take() {
            tx.commit().await.map_err(|e| {
                NurtureError::Infrastructure(format!("Transaction commit failed: {}", e))
            })?;
        }
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), NurtureError> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await.map_err(|e| {
                NurtureError::Infrastructure(format!("Transaction rollback failed: {}", e))
            })?;
        }
        Ok(())
    }
}
