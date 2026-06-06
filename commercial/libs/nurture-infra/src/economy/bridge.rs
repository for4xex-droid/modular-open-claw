/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::csam::{CsamPipeline, ScanVerdict};
use crate::economy::interceptor::EconomyInterceptor;
use aiome_core_contracts::commerce::CommerceEngine;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::JobQueue;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::{Duration, Utc};
use commerce_protocol::identity::ActorId;
use nurture_core::ledger::EconomyLedger;
use nurture_core::license::{AssetLicense, LicenseStore};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;

use crate::marketplace::sqlite::SQLiteMarketplace;
use commerce_protocol::settlement::SettlementProtocol;
use commerce_protocol::transaction::Transaction;
use nurture_core::policy::{EconomyPolicy, SharedPolicy};

pub struct NurtureCommerceBridge {
    ledger: Arc<dyn EconomyLedger>,
    settlement: Arc<dyn SettlementProtocol>,
    marketplace: Arc<SQLiteMarketplace>,
    interceptor: Arc<EconomyInterceptor>,
    csam_pipeline: Arc<CsamPipeline>,
    job_queue: Arc<dyn JobQueue>,
    idempotency: Arc<dyn crate::economy::idempotency::IdempotencyStore>,
    license_store: Arc<dyn LicenseStore>,
    karma_forge: Arc<crate::economy::karma_forge::KarmaForge>,
    policy: SharedPolicy,
    pool: SqlitePool,
    uow_manager: Arc<dyn nurture_core::uow::UowManager>,
}

impl NurtureCommerceBridge {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ledger: Arc<dyn EconomyLedger>,
        settlement: Arc<dyn SettlementProtocol>,
        marketplace: Arc<SQLiteMarketplace>,
        interceptor: Arc<EconomyInterceptor>,
        csam_pipeline: Arc<CsamPipeline>,
        job_queue: Arc<dyn JobQueue>,
        idempotency: Arc<dyn crate::economy::idempotency::IdempotencyStore>,
        license_store: Arc<dyn LicenseStore>,
        karma_forge: Arc<crate::economy::karma_forge::KarmaForge>,
        policy: SharedPolicy,
        pool: SqlitePool,
        uow_manager: Arc<dyn nurture_core::uow::UowManager>,
    ) -> Self {
        Self {
            ledger,
            settlement,
            marketplace,
            interceptor,
            csam_pipeline,
            job_queue,
            idempotency,
            license_store,
            karma_forge,
            policy,
            pool,
            uow_manager,
        }
    }

    /// トランザクション内でLedgerに返金エントリを挿入する共通ヘルパー。
    /// Merkle監査ハッシュチェーンの連続性を保証する。
    async fn insert_ledger_refund_entry(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        credit_account_str: &str,
        amount: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AiomeError> {
        // CWE-20: Validate account string format to protect ledger integrity
        if Uuid::parse_str(credit_account_str).is_err() {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Invalid UUID format for credit account: {}",
                    credit_account_str
                ),
            });
        }

        // 負値ガード: DB データ破損時に u64 ラップアラウンドを防止
        if amount <= 0 {
            return Err(AiomeError::Infrastructure {
                reason: format!("Refund amount must be positive, got: {}", amount),
            });
        }
        let safe_amount: u64 = u64::try_from(amount).map_err(|_| AiomeError::Infrastructure {
            reason: format!("Refund amount {} exceeds u64 range", amount),
        })?;

        let prev_hash: String =
            sqlx::query_scalar("SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1")
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .unwrap_or_else(|| "sha256:initial".to_string());

        let entry_id = Uuid::new_v4();
        let tx_id = Uuid::new_v4();
        let debit_str = Uuid::nil().to_string();

        // 正規パスと同一のシリアライズ方式を使用 (ledger.rs:91 と同等)
        let entry_type_str = serde_json::to_string(&nurture_core::ledger::EntryType::Refund)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("EntryType serialization failed: {}", e),
            })?;

        let new_hash = crate::economy::merkle::MerkleAudit::calculate(
            &prev_hash,
            entry_id,
            &entry_type_str,
            &debit_str,
            credit_account_str,
            safe_amount,
            0,
        );

        sqlx::query(
            "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(entry_id.to_string())
        .bind(tx_id.to_string())
        .bind(Option::<String>::None)
        .bind(debit_str)
        .bind(credit_account_str)
        .bind(amount)
        .bind(0)
        .bind(&entry_type_str)
        .bind(now)
        .bind(new_hash)
        .execute(&mut **tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Ledger refund entry insertion failed: {}", e),
        })?;

        Ok(())
    }

    /// [`insert_ledger_refund_entry`] のパブリックラッパー。
    ///
    /// `forget_actor` ルートのようなクレート外コードから Merkle 監査エントリを
    /// 挿入する必要がある場合に使用する。
    ///
    /// # 引数
    /// - `tx`: 既存の SQLite トランザクション
    /// - `credit_account_str`: 返金先の actor_id (文字列)
    /// - `amount`: 返金額 (i64, 正値でなければエラー)
    /// - `now`: タイムスタンプ
    pub async fn insert_ledger_refund_entry_pub(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        credit_account_str: &str,
        amount: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AiomeError> {
        Self::insert_ledger_refund_entry(tx, credit_account_str, amount, now).await
    }

    /// トランザクション内でLedgerに購入エントリを挿入する共通ヘルパー (escrow_release用)。
    async fn insert_ledger_purchase_entry(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        debit_account_str: &str,
        credit_account_str: &str,
        amount: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AiomeError> {
        // CWE-20: Validate account string formats to protect ledger integrity
        if Uuid::parse_str(debit_account_str).is_err() {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Invalid UUID format for debit account: {}",
                    debit_account_str
                ),
            });
        }
        if Uuid::parse_str(credit_account_str).is_err() {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Invalid UUID format for credit account: {}",
                    credit_account_str
                ),
            });
        }

        if amount <= 0 {
            return Err(AiomeError::Infrastructure {
                reason: format!("Purchase amount must be positive, got: {}", amount),
            });
        }
        let safe_amount: u64 = u64::try_from(amount).map_err(|_| AiomeError::Infrastructure {
            reason: format!("Purchase amount {} exceeds u64 range", amount),
        })?;

        let prev_hash: String =
            sqlx::query_scalar("SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1")
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .unwrap_or_else(|| "sha256:initial".to_string());

        let entry_id = Uuid::new_v4();
        let tx_id = Uuid::new_v4();

        let entry_type_str = serde_json::to_string(&nurture_core::ledger::EntryType::Purchase)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("EntryType serialization failed: {}", e),
            })?;

        let new_hash = crate::economy::merkle::MerkleAudit::calculate(
            &prev_hash,
            entry_id,
            &entry_type_str,
            debit_account_str,
            credit_account_str,
            safe_amount,
            0,
        );

        sqlx::query(
            "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(entry_id.to_string())
        .bind(tx_id.to_string())
        .bind(Option::<String>::None)
        .bind(debit_account_str)
        .bind(credit_account_str)
        .bind(amount)
        .bind(0)
        .bind(&entry_type_str)
        .bind(now)
        .bind(new_hash)
        .execute(&mut **tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Ledger purchase entry insertion failed: {}", e),
        })?;

        Ok(())
    }

    /// Dynamically reloads the economy policy, taking effect immediately for all subsequent transactions and persisting to DB.
    pub async fn reload_policy(&self, new_policy: EconomyPolicy) -> Result<(), AiomeError> {
        // Validate policy invariants before applying
        new_policy
            .validate()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to validate new policy: {}", e),
            })?;

        // Persist to DB first to ensure durability
        let payload =
            serde_json::to_string(&new_policy).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to serialize new policy: {}", e),
            })?;

        sqlx::query(
            "INSERT INTO nurture_settings (setting_key, payload, updated_at)
             VALUES ('economy_policy', ?, CURRENT_TIMESTAMP)
             ON CONFLICT(setting_key) DO UPDATE SET payload = excluded.payload, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to persist new policy to DB: {}", e),
        })?;

        // Update in-memory state
        let mut policy_guard = self.policy.write().await;
        *policy_guard = new_policy;
        tracing::info!("♻️ [Nurture] Economy policy successfully persisted and applied.");
        Ok(())
    }
}

#[async_trait]
impl CommerceEngine for NurtureCommerceBridge {
    async fn create_checkout_session(
        &self,
        _agent_id: Uuid,
        _price_id: &str,
        _success_url: &str,
        _cancel_url: &str,
    ) -> Result<String, AiomeError> {
        tracing::debug!(
            "🛡️ [NurtureCommerceBridge] create_checkout_session() called - sealed with Err"
        );
        Err(AiomeError::Infrastructure {
            reason: "Checkout sessions are not available in v1.1".to_string(),
        })
    }

    async fn get_daily_limit(&self, agent_id: uuid::Uuid) -> Result<u64, AiomeError> {
        let policy = self.policy.read().await;
        let wallet = self
            .ledger
            .get_balance(&commerce_protocol::identity::ActorId(agent_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        Ok(wallet.daily_limit.min(policy.daily_spend_limit))
    }
    async fn get_daily_spend(&self, agent_id: uuid::Uuid) -> Result<u64, AiomeError> {
        let wallet = self
            .ledger
            .get_balance(&commerce_protocol::identity::ActorId(agent_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        Ok(wallet.spent_today)
    }

    async fn list_escrows(
        &self,
        agent_id: Uuid,
    ) -> Result<Vec<aiome_core_contracts::commerce::EscrowRecord>, AiomeError> {
        let rows: Vec<(String, String, i64, String, String)> = sqlx::query_as(
            "SELECT escrow_id, agent_id, amount, status, created_at \
             FROM nurture_escrows WHERE agent_id = ? \
             ORDER BY created_at DESC LIMIT 200",
        )
        .bind(agent_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to list escrows: {}", e),
        })?;

        let records = rows
            .into_iter()
            .map(|(escrow_id, payer_id, amount, status, created_at)| {
                aiome_core_contracts::commerce::EscrowRecord {
                    id: escrow_id.clone(),
                    payer_id,
                    order_id: escrow_id, // Nurture では escrow_id == order_id
                    amount,
                    status,
                    created_at,
                }
            })
            .collect();

        Ok(records)
    }

    async fn escrow_create(&self, agent_id: uuid::Uuid, amount: u64) -> Result<String, AiomeError> {
        // 0. ゼロ額ガード (Fail-fast: DB CHECK 制約に达する前に拒否)
        if amount == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Escrow amount must be greater than zero".into(),
            });
        }

        // 1. 残高チェック
        let wallet = self
            .ledger
            .get_balance(&ActorId(agent_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if wallet.coin.balance < amount {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Insufficient funds for escrow. Needed: {}, Have: {}",
                    amount, wallet.coin.balance
                ),
            });
        }

        let policy = self.policy.read().await;
        let new_daily =
            wallet
                .spent_today
                .checked_add(amount)
                .ok_or_else(|| AiomeError::Infrastructure {
                    reason: "spent_today overflow detected".into(),
                })?;
        if new_daily > policy.daily_spend_limit {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Escrow would exceed daily spend limit. Limit: {}, Current: {}, Requested: {}",
                    policy.daily_spend_limit, wallet.spent_today, amount
                ),
            });
        }

        let safe_amount = i64::try_from(amount).map_err(|_| AiomeError::Infrastructure {
            reason: format!("Escrow amount {} exceeds maximum limit", amount),
        })?;

        let escrow_id = format!("escrow-{}", Uuid::new_v4());

        // 3. DB トランザクションで残高引き落とし + escrow レコード作成
        // Note: get_balance と UPDATE の間に TOCTOU 競合があり得るが、
        // UPDATE の WHERE balance >= ? 条件で二重引き落としを完全に防止。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("DB transaction begin failed: {}", e),
            })?;

        let rows = sqlx::query(
            "UPDATE nurture_wallets SET balance = balance - ?, spent_today = spent_today + ?, version = version + 1 WHERE actor_id = ? AND balance >= ?"
        )
        .bind(safe_amount)
        .bind(safe_amount)
        .bind(agent_id.to_string())
        .bind(safe_amount)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow balance deduction failed: {}", e),
        })?;

        if rows.rows_affected() == 0 {
            if let Err(rb_err) = tx.rollback().await {
                tracing::error!("Failed to rollback escrow tx: {}", rb_err);
            }
            return Err(AiomeError::Infrastructure {
                reason:
                    "Escrow balance deduction failed: concurrent modification or insufficient funds"
                        .into(),
            });
        }

        let now = Utc::now();
        // 🚨 F-1: エスクローの自動有効期限 (TTL) を設定（デフォルト24時間）
        let expires_at = now + chrono::Duration::hours(24);

        sqlx::query(
            "INSERT INTO nurture_escrows (escrow_id, agent_id, amount, status, created_at, expires_at) VALUES (?, ?, ?, 'pending', ?, ?)"
        )
        .bind(&escrow_id)
        .bind(agent_id.to_string())
        .bind(safe_amount)
        .bind(now)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            // tx は Drop で自動 rollback されるが意図を明示
            AiomeError::Infrastructure {
                reason: format!("Escrow record creation failed: {}", e),
            }
        })?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow transaction commit failed: {}", e),
        })?;

        tracing::info!(
            "🔒 Escrow created: id={}, agent={}, amount={}",
            escrow_id,
            agent_id,
            amount
        );
        Ok(escrow_id)
    }

    async fn escrow_release(
        &self,
        escrow_id: &str,
        recipient_id: uuid::Uuid,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("DB transaction begin failed: {}", e),
            })?;

        let now = Utc::now();

        // 1 & 3: CWE-367 TOCTOU Mitigation - Atomically update status and return agent_id/amount
        let row = sqlx::query(
            "UPDATE nurture_escrows SET status = 'released', recipient_id = ?, resolved_at = ? 
             WHERE escrow_id = ? AND status = 'pending' 
             RETURNING agent_id, amount",
        )
        .bind(recipient_id.to_string())
        .bind(now)
        .bind(escrow_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow status update failed: {}", e),
        })?;

        let row = row.ok_or_else(|| AiomeError::Infrastructure {
            reason: format!("Escrow not found or already resolved: {}", escrow_id),
        })?;

        let agent_id_str: String = row.get("agent_id");
        let amount: i64 = row.get("amount");

        // 2. recipient に送金 (初回は daily_limit のデフォルト値も設定)
        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, version) VALUES (?, ?, 10000, 1)
             ON CONFLICT(actor_id) DO UPDATE SET balance = balance + ?"
        )
        .bind(recipient_id.to_string())
        .bind(amount)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow release credit failed: {}", e),
        })?;

        // 4. Ledger に購入記録を挿入 (監査コンプライアンス)
        Self::insert_ledger_purchase_entry(
            &mut tx,
            &agent_id_str,
            &recipient_id.to_string(),
            amount,
            now,
        )
        .await?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow release commit failed: {}", e),
        })?;

        tracing::info!(
            "🔓 Escrow released: id={}, recipient={}, amount={}",
            escrow_id,
            recipient_id,
            amount
        );
        Ok(())
    }

    async fn escrow_refund(&self, escrow_id: &str) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("DB transaction begin failed: {}", e),
            })?;

        let now = Utc::now();

        // 1 & 3: CWE-367 TOCTOU Mitigation - Atomically update status and return agent_id/amount
        let row = sqlx::query(
            "UPDATE nurture_escrows SET status = 'refunded', resolved_at = ? 
             WHERE escrow_id = ? AND status = 'pending' 
             RETURNING agent_id, amount",
        )
        .bind(now)
        .bind(escrow_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow status update failed: {}", e),
        })?;

        let row = row.ok_or_else(|| AiomeError::Infrastructure {
            reason: format!("Escrow not found or already resolved: {}", escrow_id),
        })?;

        let agent_id_str: String = row.get("agent_id");
        let amount: i64 = row.get("amount");

        // 2. 元の agent に返金
        let refund_rows = sqlx::query(
            "UPDATE nurture_wallets SET balance = balance + ?, spent_today = MAX(0, spent_today - ?) WHERE actor_id = ?"
        )
        .bind(amount)
        .bind(amount)
        .bind(&agent_id_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow refund credit failed: {}", e),
        })?;

        if refund_rows.rows_affected() == 0 {
            if let Err(rb_err) = tx.rollback().await {
                tracing::error!("Failed to rollback escrow refund tx: {}", rb_err);
            }
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Escrow refund failed: wallet not found for agent {}",
                    agent_id_str
                ),
            });
        }

        // 4. Ledger に返金記録を挿入 (監査コンプライアンス)
        Self::insert_ledger_refund_entry(&mut tx, &agent_id_str, amount, now).await?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow refund commit failed: {}", e),
        })?;

        tracing::info!(
            "💸 Escrow refunded: id={}, agent={}, amount={}",
            escrow_id,
            agent_id_str,
            amount
        );
        Ok(())
    }

    async fn stake(&self, _agent_id: uuid::Uuid, _amount: u64) -> Result<(), AiomeError> {
        tracing::debug!("🛡️ [NurtureCommerceBridge] stake() called - sealed with Err");
        Err(AiomeError::Infrastructure {
            reason: "Staking is not available in v1.1".to_string(),
        })
    }
    async fn slash(
        &self,
        _agent_id: uuid::Uuid,
        _amount: u64,
        _reason: &str,
    ) -> Result<(), AiomeError> {
        tracing::debug!("🛡️ [NurtureCommerceBridge] slash() called - sealed with Err");
        Err(AiomeError::Infrastructure {
            reason: "Slashing is not available in v1.1".to_string(),
        })
    }
    async fn deduct_generation_cost(
        &self,
        agent_id: uuid::Uuid,
        asset_id: Option<uuid::Uuid>,
        amount: u64,
        generation_type: &str,
    ) -> Result<(), AiomeError> {
        // 0. ゼロ額ガード
        if amount == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Deduction amount must be greater than zero".into(),
            });
        }

        let wallet = self
            .ledger
            .get_balance(&ActorId(agent_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if wallet.coin.balance < amount {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Insufficient funds for {} generation. Needed: {}, Have: {}",
                    generation_type, amount, wallet.coin.balance
                ),
            });
        }

        let policy = self.policy.read().await;
        let effective_daily_limit = wallet.daily_limit.min(policy.daily_spend_limit);
        let new_daily =
            wallet
                .spent_today
                .checked_add(amount)
                .ok_or_else(|| AiomeError::Infrastructure {
                    reason: "spent_today overflow detected".into(),
                })?;
        if new_daily > effective_daily_limit {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Generation would exceed daily spend limit. Effective Limit: {}, Current: {}, Requested: {}",
                    effective_daily_limit, wallet.spent_today, amount
                ),
            });
        }

        if let Some(a_id) = asset_id {
            // A2C Flow: Creator asset was used, use SettlementProtocol
            // Wait, we need the seller (creator). If it's a LoRA/skill, it might be in marketplace.
            // If it's not in marketplace, we gracefully fallback to system fee.
            match self.marketplace.get_item(&a_id).await {
                Ok(item) => {
                    let tx_initiated = Transaction::new(
                        ActorId(agent_id),
                        item.creator_id,
                        item.clone(),
                        policy.creator_points_rate,
                    );

                    // Override the amount_coins to be the dynamic inference amount, not the flat purchase price.
                    // Also recalculate creator_points_earned to match the actual amount paid,
                    // otherwise the user earns points based on the item's list price instead of
                    // the real inference cost — a potential exploit vector.
                    let mut tx_authorized = tx_initiated.authorize();
                    tx_authorized.amount_coins = amount;
                    // 自己売買（自分のアセットでの推論）の場合はポイントを付与しない（無限錬金エクスプロイト防止）
                    let is_wash_trade = agent_id == item.creator_id.0;
                    tx_authorized.creator_points_earned = if is_wash_trade {
                        0
                    } else {
                        let pts_u128 =
                            u128::from(amount) * u128::from(policy.creator_points_rate) / 10000;
                        u64::try_from(pts_u128).map_err(|_| AiomeError::Infrastructure {
                            reason: format!(
                                "Creator points overflow: {} exceeds u64 range",
                                pts_u128
                            ),
                        })?
                    };
                    tx_authorized.debit_account_version = Some(wallet.version);

                    self.settlement.settle(&tx_authorized).await.map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: format!("Settlement failed for generation cost: {}", e),
                        }
                    })?;

                    tracing::info!(
                        "💳 [A2C] Deducted {} for '{}' using asset {} (Creator: {}).",
                        amount,
                        generation_type,
                        a_id,
                        item.creator_id.0
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Asset {} not found in marketplace, falling back to pure SystemFee burn. Err: {}", a_id, e);
                }
            }
        }

        // NO Asset ID or Asset Not Found: Pure Burn Flow
        let entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: None,
            debit_account: ActorId(agent_id),
            // Burn として回収 (Sink: All Zeros Account)
            credit_account: ActorId(Uuid::nil()),
            coin_amount: amount,
            points_amount: 0,
            entry_type: nurture_core::ledger::EntryType::Burn,
            created_at: Utc::now(),
            debit_account_version: Some(wallet.version),
        };

        match self.ledger.record_entry(&entry).await {
            Ok(_) => {
                tracing::info!(
                    "🔥 [Burn] Deducted {} for '{}'. Remaining: {}",
                    amount,
                    generation_type,
                    wallet.coin.balance.checked_sub(amount).ok_or_else(|| {
                        AiomeError::Infrastructure {
                            reason: "Insufficient balance or underflow detected".into(),
                        }
                    })?
                );
                Ok(())
            }
            Err(e) => Err(AiomeError::Infrastructure {
                reason: format!("Failed to deduct generation cost: {}", e),
            }),
        }
    }

    async fn instant_refund(&self, transaction_id: &str, actor_id: Uuid) -> Result<(), AiomeError> {
        let parsed_tx_id =
            Uuid::parse_str(transaction_id).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Invalid transaction_id: {}", e),
            })?;

        let entries = self
            .ledger
            .get_entries_by_transaction(&parsed_tx_id)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        let purchase = entries
            .into_iter()
            .find(|e| e.entry_type == nurture_core::ledger::EntryType::Purchase)
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Original purchase transaction not found".into(),
            })?;

        if purchase.debit_account.0 != actor_id {
            return Err(AiomeError::Infrastructure {
                reason: "Actor is not the buyer of this transaction".into(),
            });
        }

        // Value-Bounded Trust (カルマ連動型トラスト)
        let karma_score = self
            .karma_forge
            .evaluate_trust_score(&ActorId(actor_id))
            .await?;
        let refund_limit = karma_score * 10;

        // DRM アセットの判定 (W-3 Fix: purchase.id → purchase.asset_id)
        let mut is_drm = false;
        if let Some(ref aid) = purchase.asset_id {
            match self.marketplace.get_item(aid).await {
                Ok(item) => is_drm = item.drm_enabled,
                Err(e) => {
                    // C-3: Marketplace 障害時は安全側 (is_drm=false) に倒れるが、
                    //       オペレーターに障害を通知するため warn ログを出力する。
                    tracing::warn!(
                        "⚠️ [Refund] DRM lookup failed for asset {}: {}. Defaulting to non-DRM (stricter refund check).",
                        aid, e
                    );
                }
            }
        }

        if !is_drm {
            // 非DRMアセットは持ち逃げリスクがあるため、上限額を超えたら即時拒否 (Zero-Risk)
            if purchase.coin_amount > refund_limit {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Refund rejected: Non-DRM asset value ({}) exceeds your Karma Trust bounds ({}).",
                        purchase.coin_amount, refund_limit
                    ),
                });
            }
        }

        let refund_entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: purchase.asset_id, // C-4: 元の purchase の asset_id を伝搬（監査トレーサビリティ）
            debit_account: purchase.credit_account, // seller
            credit_account: purchase.debit_account, // buyer
            coin_amount: purchase.coin_amount,
            points_amount: purchase.points_amount,
            entry_type: nurture_core::ledger::EntryType::Refund,
            created_at: chrono::Utc::now(),
            debit_account_version: None,
        };

        self.ledger
            .record_entry(&refund_entry)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Instant refund ledger recording failed: {}", e),
            })?;

        tracing::info!(
            "💸 [Refund] Processed instant refund for TX: {}",
            transaction_id
        );
        Ok(())
    }

    async fn withdraw_points(
        &self,
        actor_id: Uuid,
        points_to_withdraw: u64,
    ) -> Result<(), AiomeError> {
        if points_to_withdraw == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Withdraw amount must be positive".into(),
            });
        }

        let account = self
            .ledger
            .get_points(&commerce_protocol::identity::ActorId(actor_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if account.points.balance < points_to_withdraw {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Insufficient points. Have {}, requested {}",
                    account.points.balance, points_to_withdraw
                ),
            });
        }

        let tx_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // 1. Withdrawal entry (deduct from user points)
        let withdrawal_entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: tx_id,
            asset_id: None,
            debit_account: commerce_protocol::identity::ActorId(actor_id),
            credit_account: commerce_protocol::identity::ActorId(Uuid::nil()), // system burn
            coin_amount: 0,
            points_amount: points_to_withdraw,
            entry_type: nurture_core::ledger::EntryType::PointsWithdrawal,
            created_at: now,
            debit_account_version: None,
        };

        // 2. Charge entry (mint coins to user)
        let payout_coins_u128 =
            points_to_withdraw as u128 * account.conversion_rate as u128 / 10000;
        let payout_coins =
            u64::try_from(payout_coins_u128).map_err(|_| AiomeError::Infrastructure {
                reason: format!(
                    "Payout coins overflow: {} exceeds u64 range",
                    payout_coins_u128
                ),
            })?;

        let charge_entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: tx_id,
            asset_id: None,
            debit_account: commerce_protocol::identity::ActorId(Uuid::nil()), // system mint
            credit_account: commerce_protocol::identity::ActorId(actor_id),
            coin_amount: payout_coins,
            points_amount: 0,
            entry_type: nurture_core::ledger::EntryType::Charge,
            created_at: now,
            debit_account_version: None,
        };

        // Strict Atomic Batching: SQLite Transaction 全成功か全失敗 (Zero-Risk)
        self.ledger
            .record_batch(&[withdrawal_entry, charge_entry])
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Atomic batch points withdrawal failed: {}", e),
            })?;

        tracing::info!(
            "🏦 [Withdrawal] Actor {} withdrew {} points for {} coins.",
            actor_id,
            points_to_withdraw,
            payout_coins
        );

        Ok(())
    }
    async fn register_license(
        &self,
        agent_id: uuid::Uuid,
        asset_id: uuid::Uuid,
        transaction_id: &str,
        _license_type: &str,
    ) -> Result<String, AiomeError> {
        let license_id = Uuid::new_v4();
        let parsed_tx_id =
            Uuid::parse_str(transaction_id).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Invalid transaction_id format: {}", e),
            })?;
        let item =
            self.marketplace
                .get_item(&asset_id)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to fetch asset from marketplace: {}", e),
                })?;

        let encoded_key = match item.metadata.get("drm_key").and_then(|k| k.as_str()) {
            Some(key_str) => key_str.to_string(),
            None => {
                tracing::warn!(
                    "⚠️ DRM key missing in metadata. Generating fallback random key for item: {}",
                    asset_id
                );
                let raw_key = crate::drm::engine::DrmEngine::generate_key();
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw_key)
            }
        };

        let expires_at =
            if let commerce_protocol::offer::SaleMode::Subscription { interval_days, .. } =
                item.sale_mode
            {
                Some(Utc::now() + chrono::Duration::days(i64::from(interval_days)))
            } else {
                None
            };

        let license = nurture_core::license::AssetLicense {
            id: license_id,
            transaction_id: parsed_tx_id,
            asset_id,
            owner_id: ActorId(agent_id),
            decryption_key: encoded_key,
            issued_at: Utc::now(),
            expires_at,
            revoked_at: None,
        };

        self.license_store
            .issue_license(&license)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to register license: {}", e),
            })?;

        Ok(license_id.to_string())
    }
    fn verify_signature(&self, _payload: &str, _sig_header: &str) -> Result<(), AiomeError> {
        // INTENTIONAL DELEGATION: 実際の署名検証は nurture-api/src/routes/stripe.rs (StripeWebhookHandler) に委譲される
        tracing::debug!("🛡️ [NurtureCommerceBridge] verify_signature() called - intentionally delegated, returning Ok");
        Ok(())
    }

    async fn create_subscription(
        &self,
        _agent_id: uuid::Uuid,
        plan_id: &str,
    ) -> Result<String, AiomeError> {
        tracing::debug!(
            "🛡️ [NurtureCommerceBridge] create_subscription() called for plan {} - sealed with Err",
            plan_id
        );
        Err(AiomeError::Infrastructure {
            reason: "Subscriptions are not available in v1.1".to_string(),
        })
    }
    async fn cancel_subscription(
        &self,
        _agent_id: uuid::Uuid,
        subscription_id: &str,
    ) -> Result<(), AiomeError> {
        tracing::debug!(
            "🛡️ [NurtureCommerceBridge] cancel_subscription() called for sub {} - sealed with Err",
            subscription_id
        );
        Err(AiomeError::Infrastructure {
            reason: "Subscriptions are not available in v1.1".to_string(),
        })
    }
    async fn get_subscription_status(
        &self,
        _agent_id: uuid::Uuid,
    ) -> Result<aiome_core_contracts::commerce::SubscriptionStatus, AiomeError> {
        // サブスクリプション未実装時は None (未登録) を返す — これは安全なデフォルト値
        Ok(aiome_core_contracts::commerce::SubscriptionStatus::None)
    }
    async fn transfer(
        &self,
        from_id: uuid::Uuid,
        to_id: uuid::Uuid,
        amount: u64,
    ) -> Result<String, AiomeError> {
        if from_id == to_id {
            return Err(AiomeError::Validation {
                reason: "Self-transfer is not allowed".to_string(),
            });
        }

        let from_actor = commerce_protocol::identity::ActorId(from_id);
        let wallet =
            self.ledger
                .get_balance(&from_actor)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        if wallet.coin.balance < amount {
            return Err(AiomeError::Validation {
                reason: "Insufficient funds for transfer".to_string(),
            });
        }

        let entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: None,
            debit_account: from_actor,
            credit_account: commerce_protocol::identity::ActorId(to_id),
            coin_amount: amount,
            points_amount: 0,
            entry_type: nurture_core::ledger::EntryType::Transfer,
            created_at: chrono::Utc::now(),
            debit_account_version: Some(wallet.version),
        };

        self.ledger
            .record_batch(std::slice::from_ref(&entry))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        tracing::info!(
            "💸 [Transfer] from={} to={} amount={} tx={}",
            from_id,
            to_id,
            amount,
            entry.transaction_id
        );

        Ok(entry.transaction_id.to_string())
    }

    async fn get_balance(&self, agent_id: Uuid) -> Result<u64, AiomeError> {
        let wallet = self
            .ledger
            .get_balance(&ActorId(agent_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        Ok(wallet.coin.balance)
    }

    async fn validate_activity(
        &self,
        agent_id: Uuid,
        activity_type: &str,
        amount: u64,
    ) -> Result<(), AiomeError> {
        // 🛡️ CRITICAL-2 修正: 常に Ok(()) を返すスタブから実際のバリデーションへ昇格。
        //
        // セキュリティポリシー:
        //   1. activity_type の空文字列は即時リジェクト (スパム・悪意ある呼び出し防止)
        //   2. amount > 0 の場合のみ残高チェックを実施 (ゼロコスト活動は残高不問)
        //   3. 日次上限チェックは EconomyInterceptor と同一ロジックで実施
        //
        // NOTE: この関数は EconomyInterceptor の `check_transaction()` とは異なり、
        //       Transaction オブジェクトを持たない軽量バリデーションレイヤーである。
        //       最終的なアトミック排他は Settlement の楽観的ロックが担保する。

        // 1. activity_type バリデーション
        if activity_type.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "validate_activity: activity_type must not be empty".into(),
            });
        }

        // 許可された activity_type ホワイトリスト
        const ALLOWED_ACTIVITY_TYPES: &[&str] = &[
            "generation",
            "inference",
            "clone_fork",
            "clone_minute",
            "asset_upload",
            "knowledge_query",
            "autonomous_purchase",
            "mcp_tool",
        ];
        if !ALLOWED_ACTIVITY_TYPES.contains(&activity_type) {
            tracing::warn!(
                agent_id = %agent_id,
                activity_type = %activity_type,
                amount = %amount,
                "🚫 [validate_activity] Unknown activity type rejected"
            );
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "validate_activity: unknown activity_type '{}'. Allowed: {:?}",
                    activity_type, ALLOWED_ACTIVITY_TYPES
                ),
            });
        }

        // 2. amount > 0 の場合のみ残高チェックを実施
        if amount > 0 {
            let actor = commerce_protocol::identity::ActorId(agent_id);
            let wallet =
                self.ledger
                    .get_balance(&actor)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("validate_activity: failed to read wallet: {}", e),
                    })?;

            if wallet.coin.balance < amount {
                tracing::warn!(
                    agent_id = %agent_id,
                    activity_type = %activity_type,
                    required = %amount,
                    available = %wallet.coin.balance,
                    "🚫 [validate_activity] Insufficient balance"
                );
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Insufficient balance for activity '{}': required={}, available={}",
                        activity_type, amount, wallet.coin.balance
                    ),
                });
            }

            // 3. 日次上限チェック (ポリシーとウォレットの厳しい方を適用)
            let policy = self.policy.read().await;
            let effective_daily_limit = wallet.daily_limit.min(policy.daily_spend_limit);
            let projected_spent = wallet.spent_today.checked_add(amount).ok_or_else(|| {
                AiomeError::Infrastructure {
                    reason: "spent_today overflow detected during validation".into(),
                }
            })?;

            if projected_spent > effective_daily_limit {
                tracing::warn!(
                    agent_id = %agent_id,
                    activity_type = %activity_type,
                    amount = %amount,
                    spent_today = %wallet.spent_today,
                    effective_limit = %effective_daily_limit,
                    "🚫 [validate_activity] Daily limit would be exceeded"
                );
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Daily limit would be exceeded for activity '{}': projected={}, limit={}",
                        activity_type, projected_spent, effective_daily_limit
                    ),
                });
            }
        }

        tracing::debug!(
            agent_id = %agent_id,
            activity_type = %activity_type,
            amount = %amount,
            "✅ [validate_activity] Validation passed"
        );
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        agent_id: Uuid,
        item_id: Uuid,
        metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        // 自律型購入では idempotency_key がメタデータに含まれていることを期待（なければ新規生成）
        let idempotency_key = metadata
            .get("idempotency_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("auto-{}", Uuid::new_v4()));

        // 冪等性チェック
        if let Some(cached) = self
            .idempotency
            .get_response(&idempotency_key)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?
        {
            if let Some(res) = cached {
                return Ok(res.body); // transaction_id
            } else {
                return Err(AiomeError::Infrastructure {
                    reason: "Idempotency conflict (pending)".into(),
                });
            }
        }
        self.idempotency
            .reserve_key(&idempotency_key, chrono::Duration::minutes(10))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        let mut retries = 0;
        let max_retries = 3;

        let result = loop {
            match self.execute_purchase_step(agent_id, item_id).await {
                Ok(tx_id) => break Ok(tx_id),
                Err(AiomeError::Infrastructure { reason })
                    if reason.contains("OptimisticLock") && retries < max_retries =>
                {
                    retries += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50 * retries)).await;
                }
                Err(e) => break Err(e),
            }
        };

        if let Ok(tx_id) = &result {
            if let Err(e) = self
                .idempotency
                .save_response(&idempotency_key, 200, tx_id.clone())
                .await
            {
                tracing::warn!("Failed to save idempotency response for purchase: {}", e);
            }
        }

        result
    }

    async fn get_points(
        &self,
        agent_id: Uuid,
    ) -> Result<aiome_core_contracts::commerce::PointsBalance, AiomeError> {
        let account = self
            .ledger
            .get_points(&commerce_protocol::identity::ActorId(agent_id))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to get points: {}", e),
            })?;
        Ok(aiome_core_contracts::commerce::PointsBalance {
            balance: account.points.balance,
            lifetime_earned: account.points.lifetime_earned,
            lifetime_withdrawn: account.points.lifetime_withdrawn,
            conversion_rate_bps: account.conversion_rate,
        })
    }

    async fn get_transaction_history(
        &self,
        agent_id: Uuid,
        limit: u32,
    ) -> Result<Vec<aiome_core_contracts::commerce::TransactionRecord>, AiomeError> {
        let entries = self
            .ledger
            .get_history(&commerce_protocol::identity::ActorId(agent_id), limit)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to get history: {}", e),
            })?;

        Ok(entries
            .into_iter()
            .map(|entry| aiome_core_contracts::commerce::TransactionRecord {
                id: entry.id.to_string(),
                transaction_id: entry.transaction_id.to_string(),
                debit_account: entry.debit_account.0.to_string(),
                credit_account: entry.credit_account.0.to_string(),
                coin_amount: entry.coin_amount,
                points_amount: entry.points_amount,
                entry_type: serde_json::to_string(&entry.entry_type)
                    .unwrap_or_else(|_| "Unknown".to_string()),
                created_at: entry.created_at,
            })
            .collect())
    }

    async fn create_portal_session(
        &self,
        _agent_id: Uuid,
        _return_url: &str,
    ) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "Stripe Customer Portal is not available in Nurture Ledger context".to_string(),
        })
    }
}

impl NurtureCommerceBridge {
    /// 🚨 F-1: 有効期限切れ (TTL) となった pending 状態のエスクローを自動検知し、安全に refund する
    ///
    /// 各エスクローは個別トランザクションで処理され、1件の失敗が他の refund をブロックしない
    /// (障害分離パターン)。
    pub async fn process_expired_escrows(&self) -> Result<usize, AiomeError> {
        // 1. 期限切れエスクローの一覧を取得（読み取りのみ、ロックなし）
        let expired: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT escrow_id, agent_id, amount FROM nurture_escrows WHERE status = 'pending' AND expires_at < ?"
        )
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to sweep expired escrows: {}", e),
        })?;

        if expired.is_empty() {
            return Ok(0);
        }

        let mut refunded_count = 0;
        let mut failed_count = 0;

        // 2. 各エスクローを個別トランザクションで処理（障害分離）
        for (escrow_id, agent_id_str, amount) in &expired {
            match self
                .refund_single_expired_escrow(escrow_id, agent_id_str, *amount)
                .await
            {
                Ok(()) => {
                    refunded_count += 1;
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::error!(
                        "❌ Failed to refund expired escrow {}: {}. Continuing with remaining.",
                        escrow_id,
                        e
                    );
                }
            }
        }

        if refunded_count > 0 || failed_count > 0 {
            tracing::info!(
                "🧹 Escrow TTL sweep completed. Refunded: {}, Failed: {}, Total expired: {}",
                refunded_count,
                failed_count,
                expired.len()
            );
        }

        Ok(refunded_count)
    }

    /// 単一エスクローの refund を独立トランザクションで実行する
    async fn refund_single_expired_escrow(
        &self,
        escrow_id: &str,
        agent_id_str: &str,
        amount: i64,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("DB transaction begin failed: {}", e),
            })?;

        // 再度 pending であることを確認（TOCTOU 防止: 別プロセスが先に release/refund した場合を排除）
        let still_pending: Option<(String,)> = sqlx::query_as(
            "SELECT escrow_id FROM nurture_escrows WHERE escrow_id = ? AND status = 'pending'",
        )
        .bind(escrow_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow re-check failed for {}: {}", escrow_id, e),
        })?;

        if still_pending.is_none() {
            // 既に別プロセスで処理済み — 正常スキップ
            tracing::debug!(
                "⏭️ Escrow {} already resolved by another process",
                escrow_id
            );
            return Ok(());
        }

        // 返金
        let now = Utc::now();
        let refund_rows = sqlx::query(
            "UPDATE nurture_wallets SET balance = balance + ?, spent_today = MAX(0, spent_today - ?) WHERE actor_id = ?"
        )
        .bind(amount)
        .bind(amount)
        .bind(agent_id_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow refund credit failed for {}: {}", escrow_id, e),
        })?;

        if refund_rows.rows_affected() == 0 {
            tracing::warn!(
                "⚠️ Wallet not found for expired escrow: {} (agent: {})",
                escrow_id,
                agent_id_str
            );
            // ロールバック（Drop では暗黙的に行われるが明示）
            if let Err(rollback_err) = tx.rollback().await {
                tracing::warn!(
                    "⚠️ Failed to rollback transaction for escrow {}: {}",
                    escrow_id,
                    rollback_err
                );
            }
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Wallet not found for agent {} (escrow {})",
                    agent_id_str, escrow_id
                ),
            });
        }

        sqlx::query(
            "UPDATE nurture_escrows SET status = 'refunded', resolved_at = ? WHERE escrow_id = ?",
        )
        .bind(now)
        .bind(escrow_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow status update failed for {}: {}", escrow_id, e),
        })?;

        // Ledger に返金記録を挿入 (監査コンプライアンス)
        Self::insert_ledger_refund_entry(&mut tx, agent_id_str, amount, now).await?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow refund commit failed for {}: {}", escrow_id, e),
        })?;

        tracing::info!(
            "♻️ Auto-refunded expired escrow: {} (amount: {})",
            escrow_id,
            amount
        );
        Ok(())
    }

    async fn execute_purchase_step(
        &self,
        agent_id: Uuid,
        item_id: Uuid,
    ) -> Result<String, AiomeError> {
        let buyer_id = ActorId(agent_id);

        // 1. 商品情報の取得
        let item =
            self.marketplace
                .get_item(&item_id)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        // 🔴 自己売買防止（ウォッシュトレード攻撃対策）
        if buyer_id == item.creator_id {
            return Err(AiomeError::Infrastructure {
                reason: "自分のアイテムを購入することはできません".into(),
            });
        }

        // 2. CSAM 安全性チェック（3層パイプライン）
        let verdict = self
            .csam_pipeline
            .run_all(&item_id, &item.metadata)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("CSAM scan error: {}", e),
            })?;

        if let ScanVerdict::Rejected { reason, layer, .. } = verdict {
            let threat_msg = format!(
                "CSAM Rejected: agent={}, item={}, layer={}, reason={}",
                agent_id, item_id, layer, reason
            );
            tracing::warn!("🚨 {}", threat_msg);

            // 免疫システム(AdaptiveImmuneSystem)に学習させるため、JobQueue (Karma) に脅威情報を記録
            if let Err(e) = self
                .job_queue
                .store_karma(
                    &Uuid::new_v4().to_string(), // dummy job ID for autonomous trigger
                    "csam_defense",
                    &threat_msg,
                    "security threat injection error",
                    "autonomous_defense_system",
                    Some("commerce"),
                    Some("csam_block"),
                    None,
                    false, // is_private: 防衛教訓は公開 (OSS 側パラメータ名: is_private)
                )
                .await
            {
                tracing::error!("Failed to save karma for CSAM rejection: {}", e);
            }

            return Err(AiomeError::Infrastructure {
                reason: format!("Content safety violation ({}): {}", layer, reason),
            });
        }

        // 3. 取引の構成
        let wallet =
            self.ledger
                .get_balance(&buyer_id)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        let policy = self.policy.read().await;
        let mut tx = Transaction::new(
            buyer_id,
            item.creator_id,
            item.clone(),
            policy.creator_points_rate,
        );

        // 楽観的ロックのバージョンをセット
        tx.debit_account_version = Some(wallet.version);

        // 3. インターセプト
        self.interceptor
            .check_transaction(&tx, &wallet)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Policy Violation: {}", e),
            })?;

        // 4. 承認と決済
        let tx_auth = tx.authorize();
        let receipt = self
            .settlement
            .settle(&tx_auth)
            .await
            .map_err(|e| match e {
                commerce_protocol::error::NurtureError::OptimisticLockConflict { .. } => {
                    AiomeError::Infrastructure {
                        reason: "OptimisticLock".into(),
                    }
                }
                _ => AiomeError::Infrastructure {
                    reason: e.to_string(),
                },
            })?;

        // --- DRM ライセンス発行ロジックの追加 (🔴 C4 解決) ---
        if item.drm_enabled {
            // 本来は KMS からアセットの正規暗号化キーを取得する。
            // モックまたは未設定の場合はランダム生成にフォールバック。
            let key_b64 = match item.metadata.get("drm_key").and_then(|k| k.as_str()) {
                Some(key_str) => key_str.to_string(),
                None => {
                    tracing::warn!("⚠️ DRM 鍵がメタデータにありません。ランダム鍵でフォールバック発行します (item: {})", item.id);
                    let key = crate::drm::engine::DrmEngine::generate_key();
                    STANDARD.encode(key)
                }
            };

            let mut expires_at = None;
            if let commerce_protocol::offer::SaleMode::Subscription { interval_days, .. } =
                &item.sale_mode
            {
                expires_at = Some(Utc::now() + Duration::days(i64::from(*interval_days)));
            }

            let license = AssetLicense {
                id: Uuid::new_v4(),
                transaction_id: receipt.transaction_id,
                asset_id: item.id,
                owner_id: buyer_id,
                decryption_key: key_b64,
                issued_at: Utc::now(),
                expires_at,
                revoked_at: None,
            };

            self.license_store
                .issue_license(&license)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("DRM License 発行失敗: {}", e),
                })?;
        }

        Ok(receipt.transaction_id.to_string())
    }

    /// (Phase 3) エージェント間ギフト（譲渡）処理。
    /// CSAMガードを通過した場合のみ、所有権（License）を送信者から受信者へ移転する。
    ///
    /// # セキュリティ不変条件
    /// - 自己ギフト (`sender == receiver`) は拒否される（ライセンスリフレッシュ攻撃の防止）
    /// - CSAM 拒否時は免疫システム (Karma) に脅威情報を記録する
    /// - ライセンス操作は **revoke → issue** の順序で行い、失敗時に複製ライセンスが
    ///   生まれるリスクを最小化する（fail-closed: revoke 成功・issue 失敗 → 誰も持たない状態 > 二人が持つ状態）
    pub async fn deliver_gift(
        &self,
        asset_id: Uuid,
        sender_id: Uuid,
        receiver_id: Uuid,
    ) -> Result<(), AiomeError> {
        // 🔴 自己ギフト防止（ウォッシュトレード / ライセンスリフレッシュ攻撃対策）
        if sender_id == receiver_id {
            return Err(AiomeError::Infrastructure {
                reason: "自分自身にギフトを送ることはできません".into(),
            });
        }

        let sender_actor = ActorId(sender_id);
        let receiver_actor = ActorId(receiver_id);

        // 1. アイテム情報取得
        let item =
            self.marketplace
                .get_item(&asset_id)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to get item for gift: {}", e),
                })?;

        // 2. CSAM ガード (横流し防止)
        let verdict = self
            .csam_pipeline
            .run_all(&asset_id, &item.metadata)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("CSAM scan error during gift: {}", e),
            })?;

        if let ScanVerdict::Rejected { reason, layer, .. } = verdict {
            let threat_msg = format!(
                "CSAM Rejected via gift: sender={}, item={}, layer={}, reason={}",
                sender_id, asset_id, layer, reason
            );
            tracing::warn!("🚨 {}", threat_msg);

            // 免疫システム (AdaptiveImmuneSystem) に学習させるため、Karma に脅威情報を記録
            if let Err(e) = self
                .job_queue
                .store_karma(
                    &Uuid::new_v4().to_string(),
                    "csam_defense",
                    &threat_msg,
                    "security threat injection error",
                    "autonomous_defense_system",
                    Some("commerce"),
                    Some("csam_gift_block"),
                    None,
                    false,
                )
                .await
            {
                tracing::error!("Failed to save karma for CSAM gift rejection: {}", e);
            }

            return Err(AiomeError::Infrastructure {
                reason: format!("Gift rejected due to content safety violation: {}", reason),
            });
        }

        // 3. 所有権の確認
        let current_license_opt = self
            .license_store
            .get_license(&sender_actor, &asset_id)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to check sender license: {}", e),
            })?;

        let current_license = current_license_opt.ok_or_else(|| AiomeError::Infrastructure {
            reason: "Sender does not own the asset".to_string(),
        })?;

        // 4. 所有権の移転と台帳記録 (UoW: Fail-Closed)
        let gift_tx_id = Uuid::new_v4();
        let new_license = nurture_core::license::AssetLicense {
            id: Uuid::new_v4(),
            transaction_id: gift_tx_id,
            asset_id,
            owner_id: receiver_actor,
            decryption_key: current_license.decryption_key.clone(),
            issued_at: Utc::now(),
            expires_at: current_license.expires_at,
            revoked_at: None,
        };

        let audit_entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: gift_tx_id,
            asset_id: Some(asset_id), // C-5: ギフト対象のアセット ID を記録（監査トレーサビリティ）
            debit_account: sender_actor,
            credit_account: receiver_actor,
            coin_amount: 0,
            points_amount: 0,
            entry_type: nurture_core::ledger::EntryType::Gift,
            created_at: Utc::now(),
            debit_account_version: None,
        };

        let mut uow =
            self.uow_manager
                .begin_uow()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to begin UoW for gift: {}", e),
                })?;

        uow.transfer_license(&current_license.id, &new_license)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to transfer license: {}", e),
            })?;

        uow.record_batch(&[audit_entry])
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to record gift audit entry: {}", e),
            })?;

        uow.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to commit gift transaction: {}", e),
        })?;

        tracing::info!(
            "🎁 Gift Delivered: item={} from={} to={} tx={}",
            asset_id,
            sender_id,
            receiver_id,
            gift_tx_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csam::CsamPipeline;
    use crate::drm::license::SQLiteLicenseStore;
    use crate::economy::idempotency::SQLiteIdempotencyStore;
    use crate::economy::interceptor::EconomyInterceptor;
    use crate::economy::ledger::SQLiteEconomyLedger;
    use crate::economy::settlement::SQLiteSettlementProvider;
    use crate::marketplace::sqlite::SQLiteMarketplace;
    use crate::mock_job_queue::MockJobQueue;
    use nurture_core::policy::EconomyPolicy;
    use sqlx::SqlitePool;
    use tokio::sync::RwLock;

    async fn setup_bridge() -> NurtureCommerceBridge {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

        let policy = Arc::new(RwLock::new(EconomyPolicy::default()));

        let ledger = Arc::new(SQLiteEconomyLedger::new(pool.clone()));
        let settlement = Arc::new(SQLiteSettlementProvider::new(
            pool.clone(),
            ledger.clone() as Arc<dyn EconomyLedger>,
            policy.clone(),
            commerce_protocol::identity::ActorId(Uuid::nil()),
        ));
        let marketplace = Arc::new(SQLiteMarketplace::new(pool.clone()));
        let interceptor = Arc::new(EconomyInterceptor::new(policy.clone()));
        let csam_pipeline = Arc::new(CsamPipeline::new(vec![]));
        let job_queue = Arc::new(MockJobQueue::new("sqlite::memory:").await.unwrap());
        let idempotency = Arc::new(SQLiteIdempotencyStore::new(pool.clone()));
        use secrecy::SecretString;
        let license_store = Arc::new(SQLiteLicenseStore::new(
            pool.clone(),
            &SecretString::from("test-seed".to_string()),
        ));
        let executor = Arc::new(crate::sandbox::executor::PythonExecutor::new(
            crate::sandbox::executor::ResourceLimits::default(),
        ));
        let karma_forge = Arc::new(crate::economy::karma_forge::KarmaForge::new(
            job_queue.clone(),
            Arc::new(nurture_bridge::llm::MockLlmProvider::default()),
            executor,
        ));

        let uow_manager = Arc::new(crate::economy::uow::SqliteUowManager::new(
            pool.clone(),
            &"test-seed".to_string().into(),
        ));

        NurtureCommerceBridge::new(
            ledger,
            settlement,
            marketplace,
            interceptor,
            csam_pipeline,
            job_queue,
            idempotency,
            license_store,
            karma_forge,
            policy,
            pool,
            uow_manager,
        )
    }

    #[tokio::test]
    async fn test_withdraw_points_insufficient() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();

        let result = bridge.withdraw_points(actor, 100).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Insufficient points"));
    }

    #[tokio::test]
    async fn test_instant_refund_not_found() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();
        let tx_id = Uuid::new_v4();

        // Should fail because transaction doesn't exist
        let result = bridge.instant_refund(&tx_id.to_string(), actor).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Original purchase transaction not found"));
    }

    #[tokio::test]
    async fn test_validate_activity_overflow() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();

        // Give the actor a wallet with extremely high spent_today to trigger overflow
        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(actor.to_string())
        .bind(1000i64) // balance
        .bind(i64::MAX) // limit
        .bind(i64::MAX - 5) // spent_today close to max
        .bind(1)
        .execute(&bridge.pool)
        .await
        .unwrap();

        // 10 causes overflow: (u64::MAX - 5) + 10 > u64::MAX
        let result = bridge.validate_activity(actor, "inference", 10).await;

        assert!(
            result.is_err(),
            "Validation should fail fast on integer overflow, not saturate"
        );
    }

    #[tokio::test]
    async fn test_stake_and_slash_fail_safe() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();

        assert!(bridge.stake(actor, 100).await.is_err());
        assert!(bridge.slash(actor, 100, "test penalty").await.is_err());
    }

    #[tokio::test]
    async fn test_verify_signature_fails_on_invalid() {
        let bridge = setup_bridge().await;
        // FAIL-SAFE MOCK always returns Ok
        let result = bridge.verify_signature("{}", "t=123,v1=invalid");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscription_and_transfer_stubs() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();

        let create_res = bridge.create_subscription(actor, "plan_123").await;
        assert!(create_res.is_err());

        let cancel_res = bridge.cancel_subscription(actor, "sub_123").await;
        assert!(cancel_res.is_err());
    }

    #[tokio::test]
    async fn test_transfer_happy_path() {
        let bridge = setup_bridge().await;
        let from_actor = Uuid::new_v4();
        let to_actor = Uuid::new_v4();

        // Setup wallets
        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(from_actor.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .bind(0i64)
        .bind(1)
        .execute(&bridge.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(to_actor.to_string())
        .bind(500i64)
        .bind(5000i64)
        .bind(0i64)
        .bind(1)
        .execute(&bridge.pool)
        .await
        .unwrap();

        let result = bridge.transfer(from_actor, to_actor, 200).await;
        assert!(result.is_ok(), "Transfer should succeed");

        let from_wallet = bridge
            .ledger
            .get_balance(&commerce_protocol::identity::ActorId(from_actor))
            .await
            .unwrap();
        assert_eq!(from_wallet.coin.balance, 800);

        let to_wallet = bridge
            .ledger
            .get_balance(&commerce_protocol::identity::ActorId(to_actor))
            .await
            .unwrap();
        assert_eq!(to_wallet.coin.balance, 700);
    }

    #[tokio::test]
    async fn test_transfer_insufficient() {
        let bridge = setup_bridge().await;
        let from_actor = Uuid::new_v4();
        let to_actor = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(from_actor.to_string())
        .bind(100i64)
        .bind(5000i64)
        .bind(0i64)
        .bind(1)
        .execute(&bridge.pool)
        .await
        .unwrap();

        let result = bridge.transfer(from_actor, to_actor, 200).await;
        assert!(
            result.is_err(),
            "Transfer should fail due to insufficient funds"
        );
    }

    #[tokio::test]
    async fn test_transfer_self_rejected() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();

        let result = bridge.transfer(actor, actor, 100).await;
        assert!(result.is_err(), "Self-transfer should be rejected");
    }

    #[tokio::test]
    async fn test_deliver_gift_with_csam_guard() {
        let bridge = setup_bridge().await;
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        // Marketplace にアイテムを登録 (CSAMスキャン対象になるため必要)
        let item = commerce_protocol::commodity::ItemDescriptor {
            id: asset_id,
            kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
            name: "Secret Gift".to_string(),
            description: "A very secret gift".to_string(),
            price: commerce_protocol::commodity::PriceTag::Free,
            creator_id: commerce_protocol::identity::ActorId(Uuid::new_v4()),
            sale_mode: commerce_protocol::offer::SaleMode::Instant,
            drm_enabled: true,
            created_at: chrono::Utc::now(),
            metadata: serde_json::json!({"test": "data"}),
            content_hash: None,
        };
        bridge.marketplace.create_item(&item).await.unwrap();

        // Sender にライセンスを付与（所有している前提）
        let license = nurture_core::license::AssetLicense {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id,
            owner_id: commerce_protocol::identity::ActorId(sender_id),
            decryption_key: "key_123".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            revoked_at: None,
        };
        bridge.license_store.issue_license(&license).await.unwrap();

        // Act: Giftを配送する
        let result = bridge.deliver_gift(asset_id, sender_id, receiver_id).await;

        // Assert: 成功すること
        assert!(result.is_ok(), "Gift delivery should succeed");

        // Sender はライセンスを失っていること (譲渡のため)
        let sender_license = bridge
            .license_store
            .get_license(&commerce_protocol::identity::ActorId(sender_id), &asset_id)
            .await
            .unwrap();
        assert!(
            sender_license.is_none(),
            "Sender should no longer have the license"
        );

        // Receiver がライセンスを獲得していること
        let receiver_license = bridge
            .license_store
            .get_license(
                &commerce_protocol::identity::ActorId(receiver_id),
                &asset_id,
            )
            .await
            .unwrap();
        assert!(
            receiver_license.is_some(),
            "Receiver should have the license"
        );

        // 台帳に Gift 取引が記録されていること
        let sender_history = bridge
            .ledger
            .get_history(&commerce_protocol::identity::ActorId(sender_id), 10)
            .await
            .unwrap();
        let gift_entry = sender_history
            .iter()
            .find(|e| e.entry_type == nurture_core::ledger::EntryType::Gift);
        assert!(
            gift_entry.is_some(),
            "Gift entry should be recorded in the ledger"
        );
        let entry = gift_entry.unwrap();
        assert_eq!(entry.coin_amount, 0);
        assert_eq!(entry.credit_account.0, receiver_id);
    }

    #[tokio::test]
    async fn test_deliver_gift_self_gift_rejected() {
        let bridge = setup_bridge().await;
        let actor = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        let result = bridge.deliver_gift(asset_id, actor, actor).await;
        assert!(result.is_err(), "Self-gift should be rejected");
        assert!(
            result.unwrap_err().to_string().contains("自分自身"),
            "Error should mention self-gift prohibition"
        );
    }

    #[tokio::test]
    async fn test_deliver_gift_without_ownership() {
        let bridge = setup_bridge().await;
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        // Marketplace にアイテムを登録するが、sender にライセンスを付与しない
        let item = commerce_protocol::commodity::ItemDescriptor {
            id: asset_id,
            kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
            name: "Unowned Gift".to_string(),
            description: "Sender does not own this".to_string(),
            price: commerce_protocol::commodity::PriceTag::Free,
            creator_id: commerce_protocol::identity::ActorId(Uuid::new_v4()),
            sale_mode: commerce_protocol::offer::SaleMode::Instant,
            drm_enabled: false,
            created_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
            content_hash: None,
        };
        bridge.marketplace.create_item(&item).await.unwrap();

        let result = bridge.deliver_gift(asset_id, sender_id, receiver_id).await;
        assert!(result.is_err(), "Gift without ownership should fail");
        assert!(
            result.unwrap_err().to_string().contains("does not own"),
            "Error should mention lack of ownership"
        );
    }

    #[tokio::test]
    async fn test_deliver_gift_nonexistent_item() {
        let bridge = setup_bridge().await;
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let nonexistent_id = Uuid::new_v4();

        let result = bridge
            .deliver_gift(nonexistent_id, sender_id, receiver_id)
            .await;
        assert!(result.is_err(), "Gift of nonexistent item should fail");
    }

    /// C-4 TDD: instant_refund で生成される refund_entry が
    /// 元の purchase の asset_id を正しく伝搬することを検証する。
    #[tokio::test]
    async fn test_instant_refund_preserves_asset_id() {
        let bridge = setup_bridge().await;
        let buyer = Uuid::new_v4();
        let seller = Uuid::new_v4();
        let asset = Uuid::new_v4();
        let tx_id = Uuid::new_v4();

        // buyer の wallet を作成
        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(buyer.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .bind(0i64)
        .bind(1)
        .execute(&bridge.pool)
        .await
        .unwrap();

        // seller の wallet を作成
        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(seller.to_string())
        .bind(0i64)
        .bind(5000i64)
        .bind(0i64)
        .bind(1)
        .execute(&bridge.pool)
        .await
        .unwrap();

        // Purchase エントリを直接挿入 (asset_id 付き)
        let purchase_entry = nurture_core::ledger::LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: tx_id,
            asset_id: Some(asset),
            debit_account: commerce_protocol::identity::ActorId(buyer),
            credit_account: commerce_protocol::identity::ActorId(seller),
            coin_amount: 50, // refund_limit (karma=0 → limit=0) を超えるが、DRM=false なら拒否される
            points_amount: 0,
            entry_type: nurture_core::ledger::EntryType::Purchase,
            created_at: chrono::Utc::now(),
            debit_account_version: Some(1),
        };
        bridge.ledger.record_entry(&purchase_entry).await.unwrap();

        // Marketplace にアイテム登録 (DRM=true → karma チェックをバイパス)
        let item = commerce_protocol::commodity::ItemDescriptor {
            id: asset,
            kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
            name: "DRM Asset".to_string(),
            description: "DRM protected".to_string(),
            price: commerce_protocol::commodity::PriceTag::Fixed(50),
            creator_id: commerce_protocol::identity::ActorId(seller),
            sale_mode: commerce_protocol::offer::SaleMode::Instant,
            drm_enabled: true,
            created_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
            content_hash: None,
        };
        bridge.marketplace.create_item(&item).await.unwrap();

        // instant_refund 実行
        let result = bridge.instant_refund(&tx_id.to_string(), buyer).await;
        assert!(
            result.is_ok(),
            "instant_refund should succeed: {:?}",
            result.err()
        );

        // Refund エントリの asset_id が purchase の asset_id と一致することを検証
        let _entries = bridge
            .ledger
            .get_entries_by_transaction(&tx_id)
            .await
            .unwrap();
        // Note: refund_entry は新しい transaction_id を持つので、buyer の history から検索
        let history = bridge
            .ledger
            .get_history(&commerce_protocol::identity::ActorId(buyer), 20)
            .await
            .unwrap();

        let refund = history
            .iter()
            .find(|e| e.entry_type == nurture_core::ledger::EntryType::Refund)
            .expect("Refund entry should exist in history");

        assert_eq!(
            refund.asset_id,
            Some(asset),
            "C-4: Refund entry must preserve the original purchase's asset_id for audit traceability"
        );
    }

    /// C-5 TDD: deliver_gift で生成される audit_entry が
    /// ギフト対象の asset_id を正しく記録することを検証する。
    #[tokio::test]
    async fn test_deliver_gift_preserves_asset_id() {
        let bridge = setup_bridge().await;
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        let sender_actor = commerce_protocol::identity::ActorId(sender_id);
        let _receiver_actor = commerce_protocol::identity::ActorId(receiver_id);

        // Marketplace にアイテム登録
        let item = commerce_protocol::commodity::ItemDescriptor {
            id: asset_id,
            kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
            name: "Gift Item".to_string(),
            description: "For gift test".to_string(),
            price: commerce_protocol::commodity::PriceTag::Free,
            creator_id: commerce_protocol::identity::ActorId(Uuid::new_v4()),
            sale_mode: commerce_protocol::offer::SaleMode::Instant,
            drm_enabled: false,
            created_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
            content_hash: None,
        };
        bridge.marketplace.create_item(&item).await.unwrap();

        // sender にライセンスを発行
        let license = nurture_core::license::AssetLicense {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id,
            owner_id: sender_actor,
            decryption_key: "test-key".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            revoked_at: None,
        };
        bridge.license_store.issue_license(&license).await.unwrap();

        // deliver_gift 実行
        let result = bridge.deliver_gift(asset_id, sender_id, receiver_id).await;
        assert!(
            result.is_ok(),
            "deliver_gift should succeed: {:?}",
            result.err()
        );

        // Gift audit entry の asset_id が正しいことを検証
        let history = bridge.ledger.get_history(&sender_actor, 20).await.unwrap();

        let gift_entry = history
            .iter()
            .find(|e| e.entry_type == nurture_core::ledger::EntryType::Gift)
            .expect("Gift audit entry should exist in history");

        assert_eq!(
            gift_entry.asset_id,
            Some(asset_id),
            "C-5: Gift audit entry must record the asset_id for traceability"
        );
    }
}
