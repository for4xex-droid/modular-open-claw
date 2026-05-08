/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! LoRA Marketplace — 安全なアダプター取引基盤
//!
//! エスクロー決済・SHA-256 完全性検証・PathSandbox によるファイル隔離を
//! 組み合わせ、LoRA アダプターの出品・購入・転送を安全に仲介する。

use aiome_core_contracts::commerce::CommerceEngine;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::lora_marketplace::{
    ListingFilter, ListingStatus, LoraListing, LoraMarketplace, LoraPurchase, PurchaseStatus,
};
use async_trait::async_trait;
use shared::db::DatabasePool;
use shared::sql_exec;
use sqlx::Row;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// LoRA Marketplace の Universal (SQLite/PostgreSQL) 実装
pub struct UniversalLoraMarketplace {
    pool: DatabasePool,
    commerce_engine: Arc<dyn CommerceEngine>,
    vault_root: PathBuf,
}

impl UniversalLoraMarketplace {
    /// 新規インスタンスを生成する
    pub fn new(
        pool: DatabasePool,
        commerce_engine: Arc<dyn CommerceEngine>,
        vault_root: PathBuf,
    ) -> Self {
        Self {
            pool,
            commerce_engine,
            vault_root,
        }
    }

    fn compute_file_hash(path: &std::path::Path) -> Result<String, AiomeError> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let mut file = std::fs::File::open(path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to open adapter file for hashing: {}", e),
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = [0; 65536]; // 64KB バッファ

        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to read adapter file chunk: {}", e),
                })?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

#[async_trait]
impl LoraMarketplace for UniversalLoraMarketplace {
    async fn publish_listing(&self, listing: LoraListing) -> Result<Uuid, AiomeError> {
        // SEC: アダプターファイルの存在確認
        let adapter_full_path = self.vault_root.join(&listing.adapter_path);
        if !adapter_full_path.exists() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Adapter file not found at: {}", adapter_full_path.display()),
            });
        }

        // SEC: PathSandbox — vault_root 外へのトラバーサルを遮断
        let sandbox = shared::sandbox::PathSandbox::new(&self.vault_root).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to create PathSandbox: {}", e),
            }
        })?;
        sandbox
            .validate_path(&listing.adapter_path)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Insecure adapter path: {}. Traversal blocked.", e),
            })?;

        // SEC: モデル名のパストラバーサル防止
        if listing.model_family.contains('/')
            || listing.model_family.contains('\\')
            || listing.model_family.contains('.')
        {
            return Err(AiomeError::Infrastructure {
                reason: "Invalid model_family format. Contains prohibited path characters."
                    .to_string(),
            });
        }
        if listing.base_model.contains('/')
            || listing.base_model.contains('\\')
            || listing.base_model.contains('.')
        {
            return Err(AiomeError::Infrastructure {
                reason: "Invalid base_model format. Contains prohibited path characters."
                    .to_string(),
            });
        }

        // SEC: サイズ制限 (500MB)
        let metadata =
            std::fs::metadata(&adapter_full_path).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to read adapter metadata: {}", e),
            })?;
        if metadata.len() > 500 * 1024 * 1024 {
            return Err(AiomeError::Infrastructure {
                reason: "Adapter file exceeds 500MB size limit".to_string(),
            });
        }

        // SEC: ハッシュ検証 — 出品者が申告したハッシュと実ファイルのハッシュが一致するか
        let actual_hash = Self::compute_file_hash(&adapter_full_path)?;
        if actual_hash != listing.adapter_hash {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Adapter hash mismatch: declared={}, actual={}",
                    listing.adapter_hash, actual_hash
                ),
            });
        }

        let tags_json =
            serde_json::to_string(&listing.tags).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Tags serialization failed: {}", e),
            })?;

        let q = format!(
            "INSERT INTO lora_listings (id, seller_id, adapter_path, model_family, base_model, title, description, price_coins, adapter_hash, adapter_size_bytes, tags, status) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, 'Open')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3),
            self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7),
            self.pool.ph(8), self.pool.ph(9), self.pool.ph(10)
        );

        sql_exec!(
            &self.pool,
            &q,
            listing.id.to_string(),
            listing.seller_id.to_string(),
            listing.adapter_path,
            listing.model_family,
            listing.base_model,
            listing.title,
            listing.description,
            listing.price_coins as i64,
            listing.adapter_hash,
            listing.adapter_size_bytes as i64,
            tags_json
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Listing database insertion failed: {}", e),
        })?;

        Ok(listing.id)
    }

    async fn list_listings(&self, filter: ListingFilter) -> Result<Vec<LoraListing>, AiomeError> {
        let mut conditions = vec!["1=1".to_string()];
        let mut bind_values: Vec<String> = Vec::new();

        if let Some(ref family) = filter.model_family {
            bind_values.push(family.clone());
            conditions.push(format!(
                "model_family = {}",
                self.pool.ph(bind_values.len() - 1)
            ));
        }
        if let Some(status) = filter.status {
            bind_values.push(status.to_string());
            conditions.push(format!("status = {}", self.pool.ph(bind_values.len() - 1)));
        }
        if let Some(seller) = filter.seller_id {
            bind_values.push(seller.to_string());
            conditions.push(format!(
                "seller_id = {}",
                self.pool.ph(bind_values.len() - 1)
            ));
        }

        let limit = filter.limit.unwrap_or(50).min(100);
        let q = format!(
            "SELECT id, seller_id, adapter_path, model_family, base_model, title, description, price_coins, adapter_hash, adapter_size_bytes, tags, status, created_at FROM lora_listings WHERE {} ORDER BY created_at DESC LIMIT {}",
            conditions.join(" AND "),
            limit
        );

        // Helper to extract listing from a row
        fn parse_listing_row(
            id: String,
            seller_id: String,
            adapter_path: String,
            model_family: String,
            base_model: String,
            title: String,
            description: String,
            price_coins: i64,
            adapter_hash: String,
            adapter_size_bytes: i64,
            tags_json: String,
            status_str: String,
        ) -> Result<LoraListing, AiomeError> {
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let status = match status_str.as_str() {
                "Sold" => ListingStatus::Sold,
                "Delisted" => ListingStatus::Delisted,
                _ => ListingStatus::Open,
            };

            let id = Uuid::parse_str(&id).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
            let seller_id =
                Uuid::parse_str(&seller_id).map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

            Ok(LoraListing {
                id,
                seller_id,
                adapter_path,
                model_family,
                base_model,
                title,
                description,
                price_coins: price_coins as u64,
                adapter_hash,
                adapter_size_bytes: adapter_size_bytes as u64,
                tags,
                status,
                created_at: chrono::Utc::now(),
            })
        }

        let listings: Vec<LoraListing> = match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let mut query = sqlx::query(&q);
                for v in &bind_values {
                    query = query.bind(v);
                }
                let rows = query
                    .fetch_all(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                rows.into_iter()
                    .filter_map(|row| {
                        parse_listing_row(
                            row.get("id"),
                            row.get("seller_id"),
                            row.get("adapter_path"),
                            row.get("model_family"),
                            row.get("base_model"),
                            row.get("title"),
                            row.get("description"),
                            row.get("price_coins"),
                            row.get("adapter_hash"),
                            row.get("adapter_size_bytes"),
                            row.get("tags"),
                            row.get("status"),
                        )
                        .ok()
                    })
                    .collect()
            }
            DatabasePool::Postgres(pool) => {
                let mut query = sqlx::query(&q);
                for v in &bind_values {
                    query = query.bind(v);
                }
                let rows = query
                    .fetch_all(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                rows.into_iter()
                    .filter_map(|row| {
                        parse_listing_row(
                            row.get("id"),
                            row.get("seller_id"),
                            row.get("adapter_path"),
                            row.get("model_family"),
                            row.get("base_model"),
                            row.get("title"),
                            row.get("description"),
                            row.get("price_coins"),
                            row.get("adapter_hash"),
                            row.get("adapter_size_bytes"),
                            row.get("tags"),
                            row.get("status"),
                        )
                        .ok()
                    })
                    .collect()
            }
        };

        Ok(listings)
    }

    async fn purchase(&self, listing_id: Uuid, buyer_id: Uuid) -> Result<LoraPurchase, AiomeError> {
        // 1. Fetch listing
        let listing_id_str = listing_id.to_string();
        let q = format!(
            "SELECT seller_id, price_coins, status FROM lora_listings WHERE id = {}",
            self.pool.ph(0)
        );

        let (seller_id_str, price, status_str) = match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(&q)
                    .bind(&listing_id_str)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Listing not found: {}", e),
                    })?;
                (
                    row.get::<String, _>("seller_id"),
                    row.get::<i64, _>("price_coins"),
                    row.get::<String, _>("status"),
                )
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(&q)
                    .bind(&listing_id_str)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Listing not found: {}", e),
                    })?;
                (
                    row.get::<String, _>("seller_id"),
                    row.get::<i64, _>("price_coins"),
                    row.get::<String, _>("status"),
                )
            }
        };

        // SEC: ステータス検証
        if status_str != "Open" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Listing is not available (status: {})", status_str),
            });
        }

        // SEC: 自己購入ブロック
        let seller_id =
            Uuid::parse_str(&seller_id_str).map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid seller UUID".into(),
            })?;
        if seller_id == buyer_id {
            return Err(AiomeError::Infrastructure {
                reason: "Cannot purchase your own listing".to_string(),
            });
        }

        // 2. Optimistic Locking: Open → Sold (Lock the listing BEFORE charging)
        let q_update = format!(
            "UPDATE lora_listings SET status = 'Sold' WHERE id = {} AND status = 'Open'",
            self.pool.ph(0)
        );

        let rows_affected = match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(&q_update)
                .bind(&listing_id.to_string())
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(&q_update)
                .bind(&listing_id.to_string())
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .rows_affected(),
        };

        if rows_affected == 0 {
            // Race condition: listing was already purchased or delisted
            return Err(AiomeError::Infrastructure {
                reason: "Race condition: listing no longer available".into(),
            });
        }

        // 3. エスクロー作成 (If this fails, we must unlock the listing)
        let escrow_result = self
            .commerce_engine
            .escrow_create(buyer_id, price as u64)
            .await;
        let escrow_id = match escrow_result {
            Ok(id) => id,
            Err(e) => {
                let q_revert = format!(
                    "UPDATE lora_listings SET status = 'Open' WHERE id = {}",
                    self.pool.ph(0)
                );
                if let Err(e) = sql_exec!(&self.pool, &q_revert, listing_id.to_string()) {
                    tracing::error!("🚨 [LoraMarketplace] CRITICAL: Failed to revert listing {} to Open after escrow failure: {:?}", listing_id, e);
                }
                return Err(AiomeError::Infrastructure {
                    reason: format!("Failed to create escrow: {}", e),
                });
            }
        };

        // 4. 購入レコード作成 (If this fails, we must refund escrow and unlock the listing)
        let purchase_id = Uuid::new_v4();
        let q_purchase = format!(
            "INSERT INTO lora_purchases (id, listing_id, buyer_id, escrow_id, status) VALUES ({0}, {1}, {2}, {3}, 'Escrowed')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );

        if let Err(e) = sql_exec!(
            &self.pool,
            &q_purchase,
            purchase_id.to_string(),
            listing_id_str,
            buyer_id.to_string(),
            escrow_id.clone()
        ) {
            // Rollback everything
            if let Err(refund_err) = self.commerce_engine.escrow_refund(&escrow_id).await {
                tracing::error!(
                    "🚨 [LoraMarketplace] CRITICAL: Escrow refund failed for {}. User funds may be locked! Error: {}",
                    escrow_id, refund_err
                );
            }
            let q_revert = format!(
                "UPDATE lora_listings SET status = 'Open' WHERE id = {}",
                self.pool.ph(0)
            );
            if let Err(e) = sql_exec!(&self.pool, &q_revert, listing_id.to_string()) {
                tracing::error!("🚨 [LoraMarketplace] CRITICAL: Failed to revert listing {} to Open after purchase insertion failure: {:?}", listing_id, e);
            }

            return Err(AiomeError::Infrastructure {
                reason: format!("Purchase insertion failed: {}", e),
            });
        }

        Ok(LoraPurchase {
            id: purchase_id,
            listing_id,
            buyer_id,
            escrow_id,
            status: PurchaseStatus::Escrowed,
            purchased_at: chrono::Utc::now(),
        })
    }

    async fn complete_purchase(
        &self,
        purchase_id: Uuid,
        caller_id: Uuid,
    ) -> Result<(), AiomeError> {
        let purchase_id_str = purchase_id.to_string();

        // 1. Fetch purchase + listing
        let q = format!(
            "SELECT p.listing_id, p.buyer_id, p.escrow_id, p.status AS p_status, \
             l.seller_id, l.adapter_path, l.adapter_hash, l.model_family \
             FROM lora_purchases p JOIN lora_listings l ON l.id = p.listing_id \
             WHERE p.id = {}",
            self.pool.ph(0)
        );

        // Extract all fields inside each match arm to avoid Row type mismatch
        type PurchaseRow = (String, String, String, String, String, String, String);
        let (
            p_status,
            escrow_id,
            adapter_path,
            expected_hash,
            model_family,
            buyer_id_str,
            seller_id_str,
        ): PurchaseRow = match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(&q)
                    .bind(&purchase_id_str)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Purchase not found: {}", e),
                    })?;
                (
                    row.get("p_status"),
                    row.get("escrow_id"),
                    row.get("adapter_path"),
                    row.get("adapter_hash"),
                    row.get("model_family"),
                    row.get("buyer_id"),
                    row.get("seller_id"),
                )
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(&q)
                    .bind(&purchase_id_str)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Purchase not found: {}", e),
                    })?;
                (
                    row.get("p_status"),
                    row.get("escrow_id"),
                    row.get("adapter_path"),
                    row.get("adapter_hash"),
                    row.get("model_family"),
                    row.get("buyer_id"),
                    row.get("seller_id"),
                )
            }
        };

        if p_status != "Escrowed" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Purchase not in Escrowed state (status: {})", p_status),
            });
        }

        if caller_id.to_string() != buyer_id_str {
            return Err(AiomeError::Infrastructure {
                reason: "Unauthorized: Only the buyer can complete this purchase".to_string(),
            });
        }

        let seller_id =
            Uuid::parse_str(&seller_id_str).map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid seller UUID".into(),
            })?;

        // 2. SHA-256 ハッシュ検証
        let source_path = self.vault_root.join(&adapter_path);
        let actual_hash = Self::compute_file_hash(&source_path)?;

        if actual_hash != expected_hash {
            // Reopen listing - refetch listing_id from purchase
            let listing_id: String = match &self.pool {
                DatabasePool::Sqlite(pool) => {
                    let r = sqlx::query(&format!(
                        "SELECT listing_id FROM lora_purchases WHERE id = {}",
                        self.pool.ph(0)
                    ))
                    .bind(&purchase_id.to_string())
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                    r.get("listing_id")
                }
                DatabasePool::Postgres(pool) => {
                    let r = sqlx::query(&format!(
                        "SELECT listing_id FROM lora_purchases WHERE id = {}",
                        self.pool.ph(0)
                    ))
                    .bind(&purchase_id.to_string())
                    .fetch_one(pool)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                    r.get("listing_id")
                }
            };

            // Update purchase status FIRST
            let q_refund = format!(
                "UPDATE lora_purchases SET status = 'Refunded' WHERE id = {}",
                self.pool.ph(0)
            );
            sql_exec!(&self.pool, &q_refund, purchase_id_str.clone()).map_err(|e| {
                AiomeError::Infrastructure {
                    reason: e.to_string(),
                }
            })?;

            let q_reopen = format!(
                "UPDATE lora_listings SET status = 'Open' WHERE id = {}",
                self.pool.ph(0)
            );
            sql_exec!(&self.pool, &q_reopen, listing_id.clone()).map_err(|e| {
                AiomeError::Infrastructure {
                    reason: e.to_string(),
                }
            })?;

            // Hash mismatch! → 返金 (外部API呼び出しを最後に配置)
            if let Err(refund_err) = self.commerce_engine.escrow_refund(&escrow_id).await {
                // Rollback if refund fails
                let q_rollback_p = format!(
                    "UPDATE lora_purchases SET status = 'Escrowed' WHERE id = {}",
                    self.pool.ph(0)
                );
                let q_rollback_l = format!(
                    "UPDATE lora_listings SET status = 'Sold' WHERE id = {}",
                    self.pool.ph(0)
                );
                if let Err(e) = sql_exec!(&self.pool, &q_rollback_p, purchase_id_str.clone()) {
                    tracing::error!(
                        "🚨 [LoraMarketplace] CRITICAL: Purchase rollback failed for {}: {:?}",
                        purchase_id_str,
                        e
                    );
                }
                if let Err(e) = sql_exec!(&self.pool, &q_rollback_l, listing_id) {
                    tracing::error!(
                        "🚨 [LoraMarketplace] CRITICAL: Listing rollback failed: {:?}",
                        e
                    );
                }
                return Err(refund_err);
            }

            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Adapter integrity check failed: expected={}, actual={}. Escrow refunded.",
                    expected_hash, actual_hash
                ),
            });
        }

        // 3. Vault にコピー
        let dest_dir = self
            .vault_root
            .join("lora")
            .join(&model_family)
            .join(format!("purchased_{}", purchase_id));
        std::fs::create_dir_all(&dest_dir).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create buyer vault directory: {}", e),
        })?;

        let source_filename = source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let dest_path = dest_dir.join(&source_filename);
        std::fs::copy(&source_path, &dest_path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to copy adapter to buyer vault: {}", e),
        })?;

        // 4. ステータス更新 (DB先行更新)
        let q_complete = format!(
            "UPDATE lora_purchases SET status = 'Completed' WHERE id = {}",
            self.pool.ph(0)
        );
        sql_exec!(&self.pool, &q_complete, purchase_id_str.clone()).map_err(|e| {
            AiomeError::Infrastructure {
                reason: e.to_string(),
            }
        })?;

        // 5. エスクロー解放 → 出品者に送金 (外部API連携)
        if let Err(release_err) = self
            .commerce_engine
            .escrow_release(&escrow_id, seller_id)
            .await
        {
            // Rollback status if external API fails
            let q_rollback = format!(
                "UPDATE lora_purchases SET status = 'Escrowed' WHERE id = {}",
                self.pool.ph(0)
            );
            if let Err(e) = sql_exec!(&self.pool, &q_rollback, purchase_id_str.clone()) {
                tracing::error!("🚨 [LoraMarketplace] CRITICAL: Escrow release rollback failed for purchase {}: {:?}", purchase_id_str, e);
            }
            return Err(release_err);
        }

        tracing::info!(
            "✅ [LoraMarketplace] Purchase {} completed. Adapter copied to {}",
            purchase_id,
            dest_path.display()
        );

        Ok(())
    }

    async fn delist(&self, listing_id: Uuid, seller_id: Uuid) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE lora_listings SET status = 'Delisted' WHERE id = {} AND seller_id = {} AND status = 'Open'",
            self.pool.ph(0), self.pool.ph(1)
        );

        let rows_affected = match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query(&q)
                .bind(listing_id.to_string())
                .bind(seller_id.to_string())
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(&q)
                .bind(listing_id.to_string())
                .bind(seller_id.to_string())
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .rows_affected(),
        };

        if rows_affected == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Listing not found, not owned by seller, or not in Open status".into(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
    use tempfile::tempdir;

    /// CommerceEngine のテスト用モック
    struct MockCommerceEngineForMarketplace {
        escrow_should_fail: bool,
    }

    #[async_trait]
    impl CommerceEngine for MockCommerceEngineForMarketplace {
        async fn get_balance(&self, _: Uuid) -> Result<u64, AiomeError> {
            Ok(10000)
        }
        async fn validate_activity(&self, _: Uuid, _: &str, _: u64) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn execute_autonomous_purchase(
            &self,
            _: Uuid,
            _: Uuid,
            _: serde_json::Value,
        ) -> Result<String, AiomeError> {
            Ok("tx_mock".into())
        }
        async fn get_daily_spend(&self, _: Uuid) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn get_daily_limit(&self, _: Uuid) -> Result<u64, AiomeError> {
            Ok(10000)
        }
        async fn escrow_create(&self, _: Uuid, _: u64) -> Result<String, AiomeError> {
            if self.escrow_should_fail {
                Err(AiomeError::Infrastructure {
                    reason: "Mock escrow failure".into(),
                })
            } else {
                Ok(format!("escrow_{}", Uuid::new_v4()))
            }
        }
        async fn escrow_release(&self, _: &str, _: Uuid) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn escrow_refund(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn stake(&self, _: Uuid, _: u64) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn slash(&self, _: Uuid, _: u64, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn register_license(
            &self,
            _: Uuid,
            _: Uuid,
            _: &str,
            _: &str,
        ) -> Result<String, AiomeError> {
            Ok("lic_mock".into())
        }
        fn verify_signature(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn create_checkout_session(
            &self,
            _: Uuid,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<String, AiomeError> {
            Ok("cs_dummy".into())
        }
        async fn create_subscription(&self, _: Uuid, _: &str) -> Result<String, AiomeError> {
            Ok("sub_mock".into())
        }
        async fn cancel_subscription(&self, _: Uuid, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_subscription_status(
            &self,
            _: Uuid,
        ) -> Result<aiome_core_contracts::commerce::SubscriptionStatus, AiomeError> {
            Ok(aiome_core_contracts::commerce::SubscriptionStatus::None)
        }
        async fn transfer(&self, _: Uuid, _: Uuid, _: u64) -> Result<String, AiomeError> {
            Ok("tx_mock".into())
        }
        async fn deduct_generation_cost(
            &self,
            _: Uuid,
            _: Option<Uuid>,
            _: u64,
            _: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn list_escrows(
            &self,
            _agent_id: Uuid,
        ) -> Result<Vec<aiome_core_contracts::commerce::EscrowRecord>, AiomeError> {
            Ok(vec![])
        }
        async fn instant_refund(
            &self,
            _transaction_id: &str,
            _actor_id: Uuid,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn withdraw_points(&self, _actor_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_points(
            &self,
            _agent_id: Uuid,
        ) -> Result<aiome_core_contracts::commerce::PointsBalance, AiomeError> {
            Ok(aiome_core_contracts::commerce::PointsBalance {
                balance: 0,
                lifetime_earned: 0,
                lifetime_withdrawn: 0,
                conversion_rate_bps: 10000,
            })
        }
        async fn get_transaction_history(
            &self,
            _agent_id: Uuid,
            _limit: u32,
        ) -> Result<Vec<aiome_core_contracts::commerce::TransactionRecord>, AiomeError> {
            Ok(vec![])
        }
    }

    async fn setup_marketplace(tmp: &tempfile::TempDir) -> (UniversalLoraMarketplace, PathBuf) {
        let db_path = tmp.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        use std::str::FromStr;
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(opts)
            .await
            .unwrap();

        // Create tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_ledger_global (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_name TEXT, operation TEXT, record_id TEXT,
                new_data TEXT, prev_hash TEXT, current_hash TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS lora_listings (
                id TEXT PRIMARY KEY,
                seller_id TEXT NOT NULL,
                adapter_path TEXT NOT NULL,
                model_family TEXT NOT NULL,
                base_model TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                price_coins INTEGER NOT NULL,
                adapter_hash TEXT NOT NULL,
                adapter_size_bytes INTEGER NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'Open',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS lora_purchases (
                id TEXT PRIMARY KEY,
                listing_id TEXT NOT NULL REFERENCES lora_listings(id),
                buyer_id TEXT NOT NULL,
                escrow_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Escrowed',
                purchased_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let vault_root = tmp.path().join("vault");
        std::fs::create_dir_all(vault_root.join("lora/gemma4")).unwrap();

        let commerce = Arc::new(MockCommerceEngineForMarketplace {
            escrow_should_fail: false,
        });

        let marketplace =
            UniversalLoraMarketplace::new(DatabasePool::Sqlite(pool), commerce, vault_root.clone());

        (marketplace, vault_root)
    }

    fn create_test_adapter(vault_root: &std::path::Path, filename: &str) -> (String, String, u64) {
        let adapter_dir = vault_root.join("lora/gemma4/test_job");
        std::fs::create_dir_all(&adapter_dir).unwrap();
        let adapter_path = adapter_dir.join(filename);
        let content = b"FAKE_SAFETENSOR_DATA_FOR_TESTING";
        std::fs::write(&adapter_path, content).unwrap();

        let relative_path = adapter_path
            .strip_prefix(vault_root)
            .unwrap()
            .to_string_lossy()
            .to_string();

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = format!("{:x}", hasher.finalize());

        (relative_path, hash, content.len() as u64)
    }

    #[tokio::test]
    async fn test_publish_listing() {
        let tmp = tempdir().unwrap();
        let (marketplace, vault_root) = setup_marketplace(&tmp).await;
        let (adapter_path, adapter_hash, size) =
            create_test_adapter(&vault_root, "adapter_model.safetensors");

        let listing = LoraListing {
            id: Uuid::new_v4(),
            seller_id: Uuid::new_v4(),
            adapter_path,
            model_family: "gemma4".to_string(),
            base_model: "gemma4:26b".to_string(),
            title: "Creative Writing Adapter".to_string(),
            description: "An adapter for creative writing".to_string(),
            price_coins: 100,
            adapter_hash,
            adapter_size_bytes: size,
            tags: vec!["japanese".to_string(), "creative".to_string()],
            status: ListingStatus::Open,
            created_at: chrono::Utc::now(),
        };

        let result = marketplace.publish_listing(listing).await;
        assert!(
            result.is_ok(),
            "publish_listing should succeed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_purchase_flow_happy_path() {
        let tmp = tempdir().unwrap();
        let (marketplace, vault_root) = setup_marketplace(&tmp).await;
        let (adapter_path, adapter_hash, size) =
            create_test_adapter(&vault_root, "adapter_model.safetensors");

        let seller_id = Uuid::new_v4();
        let buyer_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();

        let listing = LoraListing {
            id: listing_id,
            seller_id,
            adapter_path: adapter_path.clone(),
            model_family: "gemma4".to_string(),
            base_model: "gemma4:26b".to_string(),
            title: "Test Adapter".to_string(),
            description: "".to_string(),
            price_coins: 50,
            adapter_hash: adapter_hash.clone(),
            adapter_size_bytes: size,
            tags: vec![],
            status: ListingStatus::Open,
            created_at: chrono::Utc::now(),
        };

        marketplace.publish_listing(listing).await.unwrap();

        // Purchase
        let purchase = marketplace.purchase(listing_id, buyer_id).await.unwrap();
        assert_eq!(purchase.status, PurchaseStatus::Escrowed);

        // Complete
        let complete_result = marketplace.complete_purchase(purchase.id, buyer_id).await;
        assert!(
            complete_result.is_ok(),
            "complete_purchase should succeed: {:?}",
            complete_result
        );

        // Verify buyer vault file exists
        let buyer_vault = vault_root
            .join("lora/gemma4")
            .join(format!("purchased_{}", purchase.id));
        assert!(buyer_vault.exists(), "Buyer vault directory should exist");
    }

    #[tokio::test]
    async fn test_self_purchase_blocked() {
        let tmp = tempdir().unwrap();
        let (marketplace, vault_root) = setup_marketplace(&tmp).await;
        let (adapter_path, adapter_hash, size) =
            create_test_adapter(&vault_root, "adapter_model.safetensors");

        let agent_id = Uuid::new_v4();

        let listing = LoraListing {
            id: Uuid::new_v4(),
            seller_id: agent_id,
            adapter_path,
            model_family: "gemma4".to_string(),
            base_model: "gemma4:26b".to_string(),
            title: "Test".to_string(),
            description: "".to_string(),
            price_coins: 50,
            adapter_hash,
            adapter_size_bytes: size,
            tags: vec![],
            status: ListingStatus::Open,
            created_at: chrono::Utc::now(),
        };

        let listing_id = listing.id;
        marketplace.publish_listing(listing).await.unwrap();

        // Self-purchase should fail
        let result = marketplace.purchase(listing_id, agent_id).await;
        assert!(result.is_err(), "Self-purchase should be blocked");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot purchase your own"),);
    }

    #[tokio::test]
    async fn test_delist_and_re_purchase_blocked() {
        let tmp = tempdir().unwrap();
        let (marketplace, vault_root) = setup_marketplace(&tmp).await;
        let (adapter_path, adapter_hash, size) =
            create_test_adapter(&vault_root, "adapter_model.safetensors");

        let seller_id = Uuid::new_v4();
        let buyer_id = Uuid::new_v4();

        let listing = LoraListing {
            id: Uuid::new_v4(),
            seller_id,
            adapter_path,
            model_family: "gemma4".to_string(),
            base_model: "gemma4:26b".to_string(),
            title: "Test".to_string(),
            description: "".to_string(),
            price_coins: 50,
            adapter_hash,
            adapter_size_bytes: size,
            tags: vec![],
            status: ListingStatus::Open,
            created_at: chrono::Utc::now(),
        };

        let listing_id = listing.id;
        marketplace.publish_listing(listing).await.unwrap();

        // Delist
        marketplace.delist(listing_id, seller_id).await.unwrap();

        // Purchase should fail (listing is Delisted)
        let result = marketplace.purchase(listing_id, buyer_id).await;
        assert!(result.is_err(), "Purchase of delisted listing should fail");
    }
}
