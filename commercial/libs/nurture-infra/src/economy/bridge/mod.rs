/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::csam::{CsamPipeline, ScanVerdict};
use crate::economy::interceptor::EconomyInterceptor;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::JobQueue;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::Utc;
use commerce_protocol::identity::ActorId;
use nurture_bridge::db::{DatabasePool, DatabaseTransaction};
use nurture_bridge::{sql_exec, sql_tx_exec, sql_tx_fetch_optional};
use nurture_core::ledger::EconomyLedger;
use nurture_core::license::{AssetLicense, LicenseStore};
use std::sync::Arc;
use uuid::Uuid;

use crate::marketplace::sqlite::SQLiteMarketplace;
use commerce_protocol::settlement::SettlementProtocol;
use commerce_protocol::transaction::Transaction;
use nurture_core::policy::{EconomyPolicy, SharedPolicy};

mod commerce_impl;
#[cfg(test)]
mod tests;

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
    pool: DatabasePool,
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
        pool: DatabasePool,
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
        tx: &mut DatabaseTransaction<'_>,
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

        let prev_hash_opt: Option<String> = sql_tx_fetch_optional!(
            tx,
            (String,),
            sqlite: "SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1",
            pg: "SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1"
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?
        .map(|r| r.0);

        let prev_hash = prev_hash_opt.unwrap_or_else(|| "sha256:initial".to_string());

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

        sql_tx_exec!(
            tx,
            sqlite: "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            pg: "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            entry_id.to_string(),
            tx_id.to_string(),
            Option::<String>::None,
            debit_str,
            credit_account_str,
            amount,
            0,
            &entry_type_str,
            now,
            new_hash
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
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
    /// - `tx`: 既存の DatabaseTransaction トランザクション
    /// - `credit_account_str`: 返金先の actor_id (文字列)
    /// - `amount`: 返金額 (i64, 正値でなければエラー)
    /// - `now`: タイムスタンプ
    pub async fn insert_ledger_refund_entry_pub(
        tx: &mut DatabaseTransaction<'_>,
        credit_account_str: &str,
        amount: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AiomeError> {
        Self::insert_ledger_refund_entry(tx, credit_account_str, amount, now).await
    }

    /// トランザクション内でLedgerに購入エントリを挿入する共通ヘルパー (escrow_release用)。
    async fn insert_ledger_purchase_entry(
        tx: &mut DatabaseTransaction<'_>,
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

        let prev_hash_opt: Option<String> = sql_tx_fetch_optional!(
            tx,
            (String,),
            sqlite: "SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1",
            pg: "SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1"
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?
        .map(|r| r.0);

        let prev_hash = prev_hash_opt.unwrap_or_else(|| "sha256:initial".to_string());

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

        sql_tx_exec!(
            tx,
            sqlite: "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            pg: "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            entry_id.to_string(),
            tx_id.to_string(),
            Option::<String>::None,
            debit_account_str,
            credit_account_str,
            amount,
            0,
            &entry_type_str,
            now,
            new_hash
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
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

        sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO nurture_settings (setting_key, payload, updated_at)
                     VALUES ('economy_policy', ?, CURRENT_TIMESTAMP)
                     ON CONFLICT(setting_key) DO UPDATE SET payload = excluded.payload, updated_at = CURRENT_TIMESTAMP",
            pg: "INSERT INTO nurture_settings (setting_key, payload, updated_at)
                 VALUES ('economy_policy', $1, CURRENT_TIMESTAMP)
                 ON CONFLICT(setting_key) DO UPDATE SET payload = excluded.payload, updated_at = CURRENT_TIMESTAMP",
            payload
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
            reason: format!("Failed to persist new policy to DB: {}", e),
        })?;

        // Update in-memory state
        let mut policy_guard = self.policy.write().await;
        *policy_guard = new_policy;
        tracing::info!("♻️ [Nurture] Economy policy successfully persisted and applied.");
        Ok(())
    }

    /// 🚨 F-1: 有効期限切れ (TTL) となった pending 状態のエスクローを自動検知し、安全に refund する
    ///
    /// 各エスクロー is processed in a separate transaction (fault isolation).
    pub async fn process_expired_escrows(&self) -> Result<usize, AiomeError> {
        // 1. 期限切れエスクローの一覧を取得（読み取りのみ、ロックなし）
        let expired: Vec<(String, String, i64)> = match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query_as(
                    "SELECT escrow_id, agent_id, amount FROM nurture_escrows WHERE status = 'pending' AND expires_at < ?"
                )
                .bind(Utc::now())
                .fetch_all(p)
                .await
            }
            DatabasePool::Postgres(p) => {
                sqlx::query_as(
                    "SELECT escrow_id, agent_id, amount FROM nurture_escrows WHERE status = 'pending' AND expires_at < $1"
                )
                .bind(Utc::now())
                .fetch_all(p)
                .await
            }
        }
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
        let still_pending: Option<(String,)> = sql_tx_fetch_optional!(
            &mut tx,
            (String,),
            sqlite: "SELECT escrow_id FROM nurture_escrows WHERE escrow_id = ? AND status = 'pending'",
            pg: "SELECT escrow_id FROM nurture_escrows WHERE escrow_id = $1 AND status = 'pending'",
            escrow_id
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
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
        let rows_affected = sql_tx_exec!(
            &mut tx,
            sqlite: "UPDATE nurture_wallets SET balance = balance + ?, spent_today = MAX(0, spent_today - ?) WHERE actor_id = ?",
            pg: "UPDATE nurture_wallets SET balance = balance + $1, spent_today = GREATEST(0, spent_today - $2) WHERE actor_id = $3",
            amount,
            amount,
            agent_id_str
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
            reason: format!("Escrow refund credit failed for {}: {}", escrow_id, e),
        })?;

        if rows_affected == 0 {
            tracing::warn!(
                "⚠️ Wallet not found for expired escrow: {} (agent: {})",
                escrow_id,
                agent_id_str
            );
            // ロールバック
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

        sql_tx_exec!(
            &mut tx,
            sqlite: "UPDATE nurture_escrows SET status = 'refunded', resolved_at = ? WHERE escrow_id = ?",
            pg: "UPDATE nurture_escrows SET status = 'refunded', resolved_at = $1 WHERE escrow_id = $2",
            now,
            escrow_id
        )
        .map_err(|e: AiomeError| AiomeError::Infrastructure {
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
                expires_at = Some(Utc::now() + chrono::Duration::days(i64::from(*interval_days)));
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
