/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use crate::economy::merkle::MerkleAudit;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_core::coin::{AiomeCoin, CoinWallet};
use nurture_core::ledger::{EconomyLedger, EntryType, LedgerEntry};
use nurture_core::points::{CreatorPoints, PointsAccount};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub struct SQLiteEconomyLedger {
    pool: SqlitePool,
}

impl SQLiteEconomyLedger {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// DB から取得した i64 値を安全に u64 へ変換するヘルパー。
/// 負の値は 0 にクランプする（DB データ破損時の防御）。
#[inline]
fn safe_u64(val: i64) -> u64 {
    val.max(0) as u64
}

#[inline]
fn safe_i64(val: u64) -> Result<i64, commerce_protocol::error::NurtureError> {
    i64::try_from(val).map_err(|_| commerce_protocol::error::NurtureError::Ledger {
        reason: format!("Value {} exceeds i64 maximum", val),
    })
}

#[async_trait]
impl EconomyLedger for SQLiteEconomyLedger {
    async fn record_entry(&self, entry: &LedgerEntry) -> Result<(), NurtureError> {
        self.record_batch(std::slice::from_ref(entry)).await
    }

    async fn record_batch(&self, entries: &[LedgerEntry]) -> Result<(), NurtureError> {
        let mut tx = self.pool.begin().await.map_err(|e| NurtureError::Ledger {
            reason: e.to_string(),
        })?;

        Self::record_batch_internal(&mut tx, entries).await?;

        tx.commit().await.map_err(|e| NurtureError::Ledger {
            reason: e.to_string(),
        })?;
        Ok(())
    }
    async fn get_balance(&self, actor: &ActorId) -> Result<CoinWallet, NurtureError> {
        let row = sqlx::query(
            "SELECT balance, lifetime_charged, lifetime_spent, daily_limit, spent_today, last_reset, version, last_transaction_at FROM nurture_wallets WHERE actor_id = ?"
        )
        .bind(actor.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;

        match row {
            Some(row) => {
                let last_reset: DateTime<Utc> = row.get("last_reset");
                let mut spent_today: u64 = safe_u64(row.get("spent_today"));

                // 日付が変わっていれば spent_today をリセット (🔴 N6 解決)
                let now = Utc::now();
                if last_reset.date_naive() < now.date_naive() {
                    spent_today = 0;
                }

                Ok(CoinWallet {
                    owner: *actor,
                    coin: AiomeCoin {
                        balance: safe_u64(row.get("balance")),
                        lifetime_charged: safe_u64(row.get("lifetime_charged")),
                        lifetime_spent: safe_u64(row.get("lifetime_spent")),
                    },
                    daily_limit: safe_u64(row.get("daily_limit")),
                    spent_today,
                    last_reset,
                    last_transaction_at: row.try_get("last_transaction_at").ok(),
                    version: safe_u64(row.get("version")),
                })
            }
            None => {
                let now = chrono::Utc::now();
                // デフォルトの空ウォレットをDBにも確保しておく（新規ユーザーが0コイン商品を買う場合のDB不整合防止 🔴 追加修正）
                if let Err(e) = sqlx::query(
                    "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES (?, 0, 0, ?) ON CONFLICT DO NOTHING"
                )
                .bind(actor.0.to_string())
                .bind(now)
                .execute(&self.pool)
                .await {
                    tracing::warn!("New wallet DB creation failed (actor: {}): {}", actor.0, e);
                }

                // デフォルトの空ウォレットを作成
                Ok(CoinWallet {
                    owner: *actor,
                    coin: AiomeCoin {
                        balance: 0,
                        lifetime_charged: 0,
                        lifetime_spent: 0,
                    },
                    daily_limit: 10_000,
                    spent_today: 0,
                    last_reset: now,
                    last_transaction_at: None,
                    version: 0,
                })
            }
        }
    }

    async fn get_points(&self, creator: &ActorId) -> Result<PointsAccount, NurtureError> {
        let row = sqlx::query(
            "SELECT balance, lifetime_earned, lifetime_withdrawn, conversion_rate FROM nurture_points WHERE actor_id = ?"
        )
        .bind(creator.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;

        match row {
            Some(row) => Ok(PointsAccount {
                creator: *creator,
                points: CreatorPoints {
                    balance: safe_u64(row.get("balance")),
                    lifetime_earned: safe_u64(row.get("lifetime_earned")),
                    lifetime_withdrawn: safe_u64(row.get("lifetime_withdrawn")),
                },
                conversion_rate: {
                    let raw = safe_u64(row.get("conversion_rate"));
                    // bps は最大 10000 (100%) を想定。DB 破損値をクランプ
                    u32::try_from(raw.min(10000)).unwrap_or(10000)
                },
            }),
            None => Ok(PointsAccount {
                creator: *creator,
                points: CreatorPoints {
                    balance: 0,
                    lifetime_earned: 0,
                    lifetime_withdrawn: 0,
                },
                conversion_rate: 10000,
            }),
        }
    }

    async fn get_history(
        &self,
        actor: &ActorId,
        limit: u32,
    ) -> Result<Vec<LedgerEntry>, NurtureError> {
        let rows = sqlx::query(
            "SELECT id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at
             FROM nurture_ledger WHERE debit_account = ? OR credit_account = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(actor.0.to_string())
        .bind(actor.0.to_string())
        .bind(safe_i64(limit.into())?)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;

        let mut entries = Vec::new();
        for row in rows {
            let id_str: &str = row.get("id");
            let tx_id_str: &str = row.get("transaction_id");
            let debit_str: &str = row.get("debit_account");
            let credit_str: &str = row.get("credit_account");
            let entry_type_str: &str = row.get("entry_type");

            entries.push(LedgerEntry {
                id: Uuid::parse_str(id_str).map_err(|e| NurtureError::Ledger {
                    reason: format!("ID パースエラー: {}", e),
                })?,
                transaction_id: Uuid::parse_str(tx_id_str).map_err(|e| NurtureError::Ledger {
                    reason: format!("TransactionID パースエラー: {}", e),
                })?,
                asset_id: {
                    // C-2: カラム未存在（マイグレーション前）は None を返すが、
                    //       存在するのにパース失敗した場合は debug ログを出力する。
                    match row.try_get::<Option<String>, _>("asset_id") {
                        Ok(Some(s)) => match Uuid::parse_str(&s) {
                            Ok(uuid) => Some(uuid),
                            Err(e) => {
                                tracing::debug!("asset_id UUID パース失敗 (id={}): {}", id_str, e);
                                None
                            }
                        },
                        Ok(None) => None,
                        Err(_) => None, // カラム未存在（マイグレーション前）
                    }
                },
                debit_account: ActorId(Uuid::parse_str(debit_str).map_err(|e| {
                    NurtureError::Ledger {
                        reason: format!("DebitAccount パースエラー: {}", e),
                    }
                })?),
                credit_account: ActorId(Uuid::parse_str(credit_str).map_err(|e| {
                    NurtureError::Ledger {
                        reason: format!("CreditAccount パースエラー: {}", e),
                    }
                })?),
                coin_amount: safe_u64(row.get("coin_amount")),
                points_amount: safe_u64(row.get("points_amount")),
                entry_type: serde_json::from_str(entry_type_str).map_err(|e| {
                    NurtureError::Ledger {
                        reason: format!("EntryType デシリアライズエラー: {}", e),
                    }
                })?,
                created_at: row.get("created_at"),
                debit_account_version: None,
            });
        }
        Ok(entries)
    }

    async fn get_entries_by_transaction(
        &self,
        transaction_id: &Uuid,
    ) -> Result<Vec<LedgerEntry>, NurtureError> {
        let rows = sqlx::query(
            "SELECT id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at
             FROM nurture_ledger WHERE transaction_id = ? ORDER BY created_at ASC"
        )
        .bind(transaction_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;

        let mut entries = Vec::new();
        for row in rows {
            let id_str: &str = row.get("id");
            let tx_id_str: &str = row.get("transaction_id");
            let debit_str: &str = row.get("debit_account");
            let credit_str: &str = row.get("credit_account");
            let entry_type_str: &str = row.get("entry_type");

            entries.push(LedgerEntry {
                id: Uuid::parse_str(id_str).map_err(|e| NurtureError::Ledger {
                    reason: format!("ID パースエラー: {}", e),
                })?,
                transaction_id: Uuid::parse_str(tx_id_str).map_err(|e| NurtureError::Ledger {
                    reason: format!("TransactionID パースエラー: {}", e),
                })?,
                asset_id: {
                    // C-2: カラム未存在（マイグレーション前）は None を返すが、
                    //       存在するのにパース失敗した場合は debug ログを出力する。
                    match row.try_get::<Option<String>, _>("asset_id") {
                        Ok(Some(s)) => match Uuid::parse_str(&s) {
                            Ok(uuid) => Some(uuid),
                            Err(e) => {
                                tracing::debug!("asset_id UUID パース失敗 (id={}): {}", id_str, e);
                                None
                            }
                        },
                        Ok(None) => None,
                        Err(_) => None, // カラム未存在（マイグレーション前）
                    }
                },
                debit_account: ActorId(Uuid::parse_str(debit_str).map_err(|e| {
                    NurtureError::Ledger {
                        reason: format!("DebitAccount パースエラー: {}", e),
                    }
                })?),
                credit_account: ActorId(Uuid::parse_str(credit_str).map_err(|e| {
                    NurtureError::Ledger {
                        reason: format!("CreditAccount パースエラー: {}", e),
                    }
                })?),
                coin_amount: safe_u64(row.get("coin_amount")),
                points_amount: safe_u64(row.get("points_amount")),
                entry_type: serde_json::from_str(entry_type_str).map_err(|e| {
                    NurtureError::Ledger {
                        reason: format!("EntryType デシリアライズエラー: {}", e),
                    }
                })?,
                created_at: row.get("created_at"),
                debit_account_version: None,
            });
        }
        Ok(entries)
    }
}
impl SQLiteEconomyLedger {
    /// 内部利用・トランザクション(UoW)用の記帳ロジック
    pub(crate) async fn record_batch_internal(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntry],
    ) -> Result<(), NurtureError> {
        // 1. まず出金側/入金側のウォレット更新を行い、SQLiteのRESERVEDロックを即座に取得する。
        // これにより「BEGIN DEFERRED」のデッドロック(同時SELECTによるSHAREDロックの競合)を防ぐ。
        for entry in entries {
            let debit_str = entry.debit_account.0.to_string();
            let credit_str = entry.credit_account.0.to_string();

            // 1.1 日次制限のリセットチェック (🔴 N6 解決)
            let now = Utc::now();
            let today_start = now
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .map(|t| t.and_utc())
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Failed to calculate today's start time".into(),
                })?;

            sqlx::query(
                "UPDATE nurture_wallets SET spent_today = 0, last_reset = ? 
                 WHERE actor_id = ? AND last_reset < ?",
            )
            .bind(now)
            .bind(&debit_str)
            .bind(today_start)
            .execute(&mut **tx)
            .await
            .map_err(|e| NurtureError::Ledger {
                reason: e.to_string(),
            })?;

            // 2. 出金側 (Debit) の更新
            let rows_affected = if entry.entry_type == EntryType::Charge
                || entry.entry_type == EntryType::SurpriseBonus
                || entry.entry_type == EntryType::Gift
            {
                // Charge / SurpriseBonus はシステムから発行するため、 debit 側の更新をスキップ
                1
            } else if let Some(version) = entry.debit_account_version {
                match entry.entry_type {
                    EntryType::Refund | EntryType::CloneMerge => {
                        // Refund時: debit(元のseller) の残高のみ減らす。spent_today は増やさない (DOS防御)
                        sqlx::query("UPDATE nurture_wallets SET balance = balance - ?, version = version + 1, last_transaction_at = CURRENT_TIMESTAMP WHERE actor_id = ? AND version = ? AND balance >= ?")
                            .bind(safe_i64(entry.coin_amount)?)
                            .bind(entry.debit_account.0.to_string())
                            .bind(safe_i64(version)?)
                            .bind(safe_i64(entry.coin_amount)?)
                            .execute(&mut **tx)
                            .await
                            .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?
                            .rows_affected()
                    }
                    // Note: EntryType::Charge は外側の if ガード (L128) で処理済みのためここには到達しない
                    EntryType::Transfer | EntryType::Purchase | EntryType::SystemFee | EntryType::PointsWithdrawal | EntryType::Burn | EntryType::CloneFork | EntryType::SageMeditation => {
                        sqlx::query(
                            "UPDATE nurture_wallets SET balance = balance - ?, lifetime_spent = lifetime_spent + ?, spent_today = spent_today + ?, version = version + 1, last_transaction_at = CURRENT_TIMESTAMP WHERE actor_id = ? AND version = ? AND balance >= ?"
                        )
                        .bind(safe_i64(entry.coin_amount)?)
                        .bind(safe_i64(entry.coin_amount)?)
                        .bind(safe_i64(entry.coin_amount)?)
                        .bind(entry.debit_account.0.to_string())
                        .bind(safe_i64(version)?)
                        .bind(safe_i64(entry.coin_amount)?)
                        .execute(&mut **tx)
                        .await
                        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?
                        .rows_affected()
                    }
                    EntryType::Charge | EntryType::SurpriseBonus | EntryType::Gift => return Err(NurtureError::Ledger { reason: "Internal logic error: Unexpected entry type in debit update".into() }),
                }
            } else {
                // Version 指定なし
                match entry.entry_type {
                    EntryType::Refund | EntryType::CloneMerge => {
                        // Refund時: debit(元のseller) の残高のみ減らす。spent_today は増やさない (DOS防御)
                        sqlx::query("UPDATE nurture_wallets SET balance = balance - ?, version = version + 1, last_transaction_at = CURRENT_TIMESTAMP WHERE actor_id = ? AND balance >= ?")
                            .bind(safe_i64(entry.coin_amount)?)
                            .bind(entry.debit_account.0.to_string())
                            .bind(safe_i64(entry.coin_amount)?)
                            .execute(&mut **tx)
                            .await
                            .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?
                            .rows_affected()
                    }
                    // Note: EntryType::Charge は外側の if ガード (L128) で処理済みのためここには到達しない
                    EntryType::Transfer | EntryType::Purchase | EntryType::SystemFee | EntryType::PointsWithdrawal | EntryType::Burn | EntryType::CloneFork | EntryType::SageMeditation => {
                        sqlx::query(
                            "UPDATE nurture_wallets SET balance = balance - ?, lifetime_spent = lifetime_spent + ?, spent_today = spent_today + ?, version = version + 1, last_transaction_at = CURRENT_TIMESTAMP WHERE actor_id = ? AND balance >= ?"
                        )
                        .bind(safe_i64(entry.coin_amount)?)
                        .bind(safe_i64(entry.coin_amount)?)
                        .bind(safe_i64(entry.coin_amount)?)
                        .bind(entry.debit_account.0.to_string())
                        .bind(safe_i64(entry.coin_amount)?)
                        .execute(&mut **tx)
                        .await
                        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?
                        .rows_affected()
                    }
                    EntryType::Charge | EntryType::SurpriseBonus | EntryType::Gift => return Err(NurtureError::Ledger { reason: "Internal logic error: Unexpected entry type in debit update".into() }),
                }
            };

            if rows_affected == 0 {
                return Err(NurtureError::OptimisticLockConflict {
                    entity: format!("wallet:{}", entry.debit_account.0),
                });
            }

            // 3. 入金先の残高更新
            match entry.entry_type {
                EntryType::Refund | EntryType::CloneMerge => {
                    // Refund時: credit(元のbuyer) の残高を増やす。さらに spent_today と lifetime_spent を回復させる
                    sqlx::query(
                        "INSERT INTO nurture_wallets (actor_id, balance) VALUES (?, ?)
                         ON CONFLICT(actor_id) DO UPDATE SET 
                         balance = balance + ?, 
                         lifetime_spent = MAX(0, lifetime_spent - ?), 
                         spent_today = MAX(0, spent_today - ?)",
                    )
                    .bind(&credit_str)
                    .bind(safe_i64(entry.coin_amount)?)
                    .bind(safe_i64(entry.coin_amount)?)
                    .bind(safe_i64(entry.coin_amount)?)
                    .bind(safe_i64(entry.coin_amount)?)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| NurtureError::Ledger {
                        reason: e.to_string(),
                    })?;

                    // Refund時: 売り手(debit) からポイントを没収 (🔴 無限ポイント錬金 防御)
                    sqlx::query(
                        "UPDATE nurture_points SET balance = MAX(0, balance - ?), lifetime_earned = MAX(0, lifetime_earned - ?) WHERE actor_id = ?"
                    )
                    .bind(safe_i64(entry.points_amount)?)
                    .bind(safe_i64(entry.points_amount)?)
                    .bind(&debit_str)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;
                }
                EntryType::Transfer
                | EntryType::Purchase
                | EntryType::SystemFee
                | EntryType::Charge
                | EntryType::Burn
                | EntryType::CloneFork
                | EntryType::SageMeditation
                | EntryType::PointsWithdrawal
                | EntryType::SurpriseBonus
                | EntryType::Gift => {
                    // 購入・チャージ等: credit 側の残高を単純に増やす
                    sqlx::query(
                        "INSERT INTO nurture_wallets (actor_id, balance) VALUES (?, ?)
                         ON CONFLICT(actor_id) DO UPDATE SET balance = balance + ?",
                    )
                    .bind(&credit_str)
                    .bind(safe_i64(entry.coin_amount)?)
                    .bind(safe_i64(entry.coin_amount)?)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| NurtureError::Ledger {
                        reason: e.to_string(),
                    })?;

                    // Purchase/PointsWithdrawal 等
                    if entry.entry_type == EntryType::Purchase {
                        // Purchase時: credit(seller) にポイントを付与
                        sqlx::query(
                            "INSERT INTO nurture_points (actor_id, balance, lifetime_earned) VALUES (?, ?, ?)
                             ON CONFLICT(actor_id) DO UPDATE SET balance = balance + ?, lifetime_earned = lifetime_earned + ?"
                        )
                        .bind(&credit_str)
                        .bind(safe_i64(entry.points_amount)?)
                        .bind(safe_i64(entry.points_amount)?)
                        .bind(safe_i64(entry.points_amount)?)
                        .bind(safe_i64(entry.points_amount)?)
                        .execute(&mut **tx)
                        .await
                        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;
                    } else if entry.entry_type == EntryType::PointsWithdrawal {
                        // PointsWithdrawal時: debit(ユーザー) からポイントを減らす
                        sqlx::query(
                            "UPDATE nurture_points SET balance = MAX(0, balance - ?) WHERE actor_id = ?"
                        )
                        .bind(safe_i64(entry.points_amount)?)
                        .bind(entry.debit_account.0.to_string())
                        .execute(&mut **tx)
                        .await
                        .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;
                    }
                }
            }
        }

        // 2. 最後に監査ハッシュの取得とレジャーへの記録を実行する
        // 既にRESERVEDロックを保持しているため、ここでのSELECTは完全に直列化される。
        let mut prev_hash: String =
            sqlx::query_scalar("SELECT audit_hash FROM nurture_ledger ORDER BY rowid DESC LIMIT 1")
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| NurtureError::Ledger {
                    reason: e.to_string(),
                })?
                .unwrap_or_else(|| "sha256:initial".to_string());

        for entry in entries {
            let debit_str = entry.debit_account.0.to_string();
            let credit_str = entry.credit_account.0.to_string();

            let entry_type_str =
                serde_json::to_string(&entry.entry_type).map_err(|e| NurtureError::Ledger {
                    reason: format!("EntryType シリアライズエラー: {}", e),
                })?;

            let new_hash = MerkleAudit::calculate(
                &prev_hash,
                entry.id,
                &entry_type_str,
                &debit_str,
                &credit_str,
                entry.coin_amount,
                entry.points_amount,
            );

            sqlx::query(
                "INSERT INTO nurture_ledger (id, transaction_id, asset_id, debit_account, credit_account, coin_amount, points_amount, entry_type, created_at, audit_hash)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(entry.id.to_string())
            .bind(entry.transaction_id.to_string())
            .bind(entry.asset_id.map(|id| id.to_string()))
            .bind(&debit_str)
            .bind(&credit_str)
            .bind(safe_i64(entry.coin_amount)?)
            .bind(safe_i64(entry.points_amount)?)
            .bind(&entry_type_str)
            .bind(entry.created_at)
            .bind(&new_hash)
            .execute(&mut **tx)
            .await
            .map_err(|e| NurtureError::Ledger { reason: e.to_string() })?;

            prev_hash = new_hash;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_ledger_record_and_balance() {
        let pool = setup_db().await;
        let ledger = SQLiteEconomyLedger::new(pool.clone());

        let buyer = ActorId(Uuid::new_v4());
        let seller = ActorId(Uuid::new_v4());

        // 初期残高設定
        sqlx::query("INSERT INTO nurture_wallets (actor_id, balance) VALUES (?, ?)")
            .bind(buyer.0.to_string())
            .bind(1000)
            .execute(&pool)
            .await
            .unwrap();

        let entry = LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: None,
            debit_account: buyer,
            credit_account: seller,
            coin_amount: 100,
            points_amount: 10,
            entry_type: EntryType::Purchase,
            created_at: Utc::now(),
            debit_account_version: None,
        };

        ledger.record_entry(&entry).await.expect("Record failed");

        let buyer_wallet = ledger.get_balance(&buyer).await.unwrap();
        assert_eq!(buyer_wallet.coin.balance, 900);
        assert_eq!(buyer_wallet.spent_today, 100);

        let seller_wallet = ledger.get_balance(&seller).await.unwrap();
        assert_eq!(seller_wallet.coin.balance, 100);

        let seller_points = ledger.get_points(&seller).await.unwrap();
        assert_eq!(seller_points.points.balance, 10);

        let history = ledger.get_history(&buyer, 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].coin_amount, 100);
    }

    #[tokio::test]
    async fn test_safe_u64_regression() {
        // [Reflexion Sprint A v3] 回帰テスト: DBの負の値を正しく0にクランプするか
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE nurture_wallets (
                actor_id TEXT PRIMARY KEY,
                balance INTEGER NOT NULL,
                lifetime_charged INTEGER NOT NULL,
                lifetime_spent INTEGER NOT NULL,
                daily_limit INTEGER NOT NULL,
                spent_today INTEGER NOT NULL,
                last_reset TIMESTAMP NOT NULL,
                last_transaction_at TIMESTAMP,
                version INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let actor = ActorId(Uuid::new_v4());
        let now = Utc::now();

        // 不正な負の値 (-1000) を直接データベースに挿入
        sqlx::query(
            "INSERT INTO nurture_wallets (actor_id, balance, lifetime_charged, lifetime_spent, daily_limit, spent_today, last_reset, version)
             VALUES (?, -1000, -500, -200, -100, -50, ?, 0)"
        )
        .bind(actor.0.to_string())
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        let ledger = SQLiteEconomyLedger::new(pool);
        let wallet = ledger.get_balance(&actor).await.unwrap();

        // 負の値が 0 に安全にクランプされ、巨大な u64::MAX 付近にラップアラウンドしないことを確認
        assert_eq!(wallet.coin.balance, 0, "Balance should be clamped to 0");
        assert_eq!(
            wallet.coin.lifetime_charged, 0,
            "Lifetime charged should be clamped to 0"
        );
        assert_eq!(
            wallet.coin.lifetime_spent, 0,
            "Lifetime spent should be clamped to 0"
        );
        assert_eq!(wallet.daily_limit, 0, "Daily limit should be clamped to 0");
        assert_eq!(wallet.spent_today, 0, "Spent today should be clamped to 0");
    }

    #[tokio::test]
    async fn test_conversion_rate_clamping() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // 開発環境等のインメモリDBテストのため、スキーマを作成
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS nurture_points (
                actor_id TEXT PRIMARY KEY,
                balance BIGINT NOT NULL DEFAULT 0,
                lifetime_earned BIGINT NOT NULL DEFAULT 0,
                lifetime_withdrawn BIGINT NOT NULL DEFAULT 0,
                conversion_rate BIGINT NOT NULL DEFAULT 10000,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let creator = ActorId(Uuid::new_v4());

        // 異常値 (20000 bps) をDBに直接挿入
        sqlx::query("INSERT INTO nurture_points (actor_id, conversion_rate) VALUES (?, ?)")
            .bind(creator.0.to_string())
            .bind(20000i64)
            .execute(&pool)
            .await
            .unwrap();

        let ledger = SQLiteEconomyLedger::new(pool);
        let account = ledger.get_points(&creator).await.unwrap();

        // 10000 bps に安全にクランプされることを確認
        assert_eq!(
            account.conversion_rate, 10000,
            "Conversion rate should be clamped to max 10000 bps"
        );
    }

    #[tokio::test]
    async fn test_ledger_entry_asset_id_preserved() {
        let pool = setup_db().await;
        // W-3: LedgerEntry に asset_id を追加 (DRM 判定用) マイグレーションをエミュレート
        sqlx::query("ALTER TABLE nurture_ledger ADD COLUMN asset_id TEXT;")
            .execute(&pool)
            .await
            .ok();

        let ledger = SQLiteEconomyLedger::new(pool.clone());
        let buyer = ActorId(Uuid::new_v4());
        let seller = ActorId(Uuid::new_v4());
        let asset = Uuid::new_v4();

        sqlx::query("INSERT INTO nurture_wallets (actor_id, balance) VALUES (?, ?)")
            .bind(buyer.0.to_string())
            .bind(1000)
            .execute(&pool)
            .await
            .unwrap();

        let entry = LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: Some(asset),
            debit_account: buyer,
            credit_account: seller,
            coin_amount: 100,
            points_amount: 0,
            entry_type: EntryType::Purchase,
            created_at: Utc::now(),
            debit_account_version: None,
        };

        ledger.record_entry(&entry).await.unwrap();

        let history = ledger.get_history(&buyer, 10).await.unwrap();
        assert_eq!(
            history[0].asset_id,
            Some(asset),
            "asset_id should be preserved through DB round-trip"
        );
    }

    #[tokio::test]
    async fn test_ledger_occ_conflict() {
        let pool = setup_db().await;
        let ledger = SQLiteEconomyLedger::new(pool.clone());

        let buyer = ActorId(Uuid::new_v4());
        let system = ActorId(Uuid::new_v4());

        // 1. 初期残高とバージョン設定 (Version = 0)
        sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, version) VALUES (?, ?, ?)")
            .bind(buyer.0.to_string())
            .bind(100)
            .bind(0) // version = 0
            .execute(&pool)
            .await
            .unwrap();

        // 2. T1: 正常な引き落とし (OCC version = 0 を指定)
        let tx1_entry = LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: None,
            debit_account: buyer,
            credit_account: system,
            coin_amount: 50,
            points_amount: 0,
            entry_type: EntryType::CloneFork, // CloneManager::fork をシミュレート
            created_at: Utc::now(),
            debit_account_version: Some(0), // 古いバージョンを指定
        };

        // これは成功し、ウォレットのバージョンが 1 になる
        ledger
            .record_entry(&tx1_entry)
            .await
            .expect("T1 (First tx) should succeed");

        // 3. T2: 二重引き落とし攻撃 (同じく OCC version = 0 を指定)
        let tx2_entry = LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: None,
            debit_account: buyer,
            credit_account: system,
            coin_amount: 50,
            points_amount: 0,
            entry_type: EntryType::CloneFork,
            created_at: Utc::now(),
            debit_account_version: Some(0), // 古いバージョンを指定したまま
        };

        // T2 は OCC 競合で失敗しなければならない
        let result = ledger.record_entry(&tx2_entry).await;
        assert!(
            matches!(
                result,
                Err(commerce_protocol::error::NurtureError::OptimisticLockConflict { .. })
            ),
            "T2 (Double spend) MUST fail with OptimisticLockConflict, but got: {:?}",
            result
        );

        // 最終残高が 100 - 50 = 50 (T1のみ成功) であることを確認
        let wallet = ledger.get_balance(&buyer).await.unwrap();
        assert_eq!(wallet.coin.balance, 50, "Only T1 should deduct balance");
        assert_eq!(wallet.version, 1, "Version should be exactly 1");
    }
}
