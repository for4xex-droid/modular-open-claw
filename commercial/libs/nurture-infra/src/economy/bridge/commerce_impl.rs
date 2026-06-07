/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use aiome_core_contracts::commerce::CommerceEngine;
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::Utc;
use commerce_protocol::identity::ActorId;
use commerce_protocol::transaction::Transaction;
use nurture_core::ledger::EconomyLedger;
use nurture_core::license::AssetLicense;
use sqlx::Row;
use uuid::Uuid;

use super::NurtureCommerceBridge;

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
        // 0. ゼロ額ガード (Fail-fast: DB CHECK 制約に達する前に拒否)
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
                base64::Engine::encode(&STANDARD, raw_key)
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
