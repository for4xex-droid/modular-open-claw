/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

//! 経済インターセプター — 全取引のプリフライト検証レイヤー。
//!
//! [`EconomyInterceptor`] はトランザクションが決済 (Settlement) に進む前に、
//! ポリシー準拠性・残高・日次上限・月次上限・取引頻度の各チェックを一元的に実施する。
//!
//! ## アーキテクチャ上の位置づけ
//! - **呼び出し元**: `bridge.rs` (自律型購入) / `buy.rs` (MCP API 購入)
//! - **TOCTOU 緩和**: 本モジュールはスナップショットベースのプリフライトチェックであり、
//!   アトミックな排他制御は下流の `SettlementProtocol` 内の楽観的ロック
//!   (`debit_account_version`) が担保する。

use commerce_protocol::error::NurtureError;
use commerce_protocol::transaction::{Initiated, Transaction};
use nurture_core::coin::CoinWallet;
use nurture_core::policy::SharedPolicy;

/// 経済取引のプリフライト検証を実行するインターセプター。
///
/// Settlement 前に以下を検証する:
/// 1. **ポリシー検証** — 取引額が最低/最大価格・単一購入上限の範囲内か
/// 2. **残高検証** — ウォレット残高が取引額以上か
/// 3. **日次上限検証** — 日次支出制限を超過しないか
/// 4. **月次上限検証** — 月次支出制限を超過しないか（0 = 無制限）
/// 5. **アノマリー検知** — 未来タイムスタンプ / 高頻度取引がないか
///
/// # 注意: ゼロ額トランザクション
/// `amount_coins == 0` (無料アイテム) はポリシーバイパスが許可されている。
/// ただし、高頻度チェック (`min_transaction_interval_ms`) は引き続き適用される。
pub struct EconomyInterceptor {
    policy: SharedPolicy,
}

impl EconomyInterceptor {
    /// 新しいインターセプターを構築する。
    ///
    /// # 引数
    /// - `policy`: `Arc<RwLock<EconomyPolicy>>` — 共有ポリシーへの参照
    pub fn new(policy: SharedPolicy) -> Self {
        Self { policy }
    }

    /// トランザクションのプリフライトチェックを実行する。
    ///
    /// # 引数
    /// - `tx`: 検証対象の `Initiated` 状態トランザクション
    /// - `wallet`: 購入者のウォレットスナップショット
    ///
    /// # エラー
    /// - [`NurtureError::PolicyViolation`] — ポリシー違反 (価格範囲・頻度制限・日次上限)
    /// - [`NurtureError::InsufficientBalance`] — 残高不足
    /// - [`NurtureError::DailyLimitExceeded`] — 日次上限超過
    /// - [`NurtureError::MonthlyLimitExceeded`] — 月次上限超過
    ///
    /// # RwLock に関する注意
    /// 本メソッドは内部で `self.policy.read().await` を呼ぶ。
    /// 呼び出し元が同一の `SharedPolicy` に対して `read()` を保持したまま本メソッドを
    /// 呼ぶ場合、write-preferring RwLock の特性上、間に writer が割り込むと
    /// デッドロックのリスクがある。呼び出し元ではポリシーのロックを
    /// 本メソッド呼び出し前にドロップすることを推奨する。
    pub async fn check_transaction(
        &self,
        tx: &Transaction<Initiated>,
        wallet: &CoinWallet,
    ) -> Result<(), NurtureError> {
        // ポリシーの read guard を早期ドロップするため、必要なフィールドを
        // ローカル変数にコピーする。これにより呼び出し元が同一の SharedPolicy に対して
        // read() を保持したまま本メソッドを呼んでもデッドロックしない。
        let (daily_spend_limit, monthly_spend_limit, min_transaction_interval_ms) = {
            let policy = self.policy.read().await;
            if let Err(e) = nurture_core::policy::validate_transaction(&policy, tx) {
                tracing::warn!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    amount = tx.amount_coins,
                    reason = %e,
                    "🚫 [Interceptor] ポリシー検証失敗"
                );
                return Err(e);
            }

            // [Zero-Trust Defense in Depth]
            // ポイント付与額がポリシーレートを上回る不正な取引（マネープリンティング攻撃）を防ぐ
            // CBA: u128→u64 ダウンキャストは try_from() で fail-fast
            let expected_points_u128 =
                tx.amount_coins as u128 * policy.creator_points_rate as u128 / 10000;
            let expected_points = u64::try_from(expected_points_u128).map_err(|_| {
                tracing::error!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    raw_value = expected_points_u128,
                    "🚨 [Interceptor] ポイント期待値が u64 範囲を超過 — データ異常"
                );
                NurtureError::Infrastructure(
                    "ポイント計算で u64 オーバーフローが発生しました".to_string(),
                )
            })?;
            if tx.creator_points_earned > expected_points {
                tracing::error!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    expected = expected_points,
                    actual = tx.creator_points_earned,
                    "🚨 [Interceptor] 不正なポイント付与額 (Creator Points) を検知"
                );
                return Err(NurtureError::PolicyViolation(
                    "システムエラー: 不正なポイント付与額が検出されました".to_string(),
                ));
            }

            (
                policy.daily_spend_limit,
                policy.monthly_spend_limit,
                policy.min_transaction_interval_ms,
            )
            // ← policy guard は ここでドロップされる
        };

        // 🚨 [Zero-Trust Defense] 楽観的ロックのバージョン指定がない場合は弾く
        if tx.debit_account_version.is_none() {
            tracing::warn!(
                buyer_id = %wallet.owner.0,
                tx_id = %tx.id,
                "🚨 [Interceptor] Transaction rejected: Missing debit_account_version (Optimistic Lock bypass attempt)"
            );
            return Err(NurtureError::PolicyViolation(
                "システムエラー: 楽観的ロックのバージョン指定がありません".to_string(),
            ));
        }

        // 残高不足チェック
        // NOTE: amount_coins == 0 (無料アイテム) は 0 < 0 → false で素通りする。
        // これはドキュメント記載の意図的な設計 (L39 参照)。
        if wallet.coin.balance < tx.amount_coins {
            tracing::warn!(
                buyer_id = %wallet.owner.0,
                tx_id = %tx.id,
                required = tx.amount_coins,
                available = wallet.coin.balance,
                "🚫 [Interceptor] 残高不足により拒否"
            );
            return Err(NurtureError::InsufficientBalance {
                required: tx.amount_coins,
                available: wallet.coin.balance,
            });
        }

        // 日次上限チェック: ポリシーの daily_spend_limit とウォレットの daily_limit の
        // 厳しい方 (小さい方) を実効上限とする。ウォレットが古いデフォルト値を保持している
        // 場合でもポリシー側の上限が適用される。
        let effective_daily_limit = wallet.daily_limit.min(daily_spend_limit);
        let projected_spent = match wallet.spent_today.checked_add(tx.amount_coins) {
            Some(v) => v,
            None => {
                tracing::error!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    spent_today = wallet.spent_today,
                    amount = tx.amount_coins,
                    "🚨 [Interceptor] spent_today の加算でオーバーフロー発生 — データ破損の可能性"
                );
                return Err(NurtureError::PolicyViolation(
                    "システムエラー: 支出計算のオーバーフローを検知しました".to_string(),
                ));
            }
        };

        if tx.amount_coins > 0 && projected_spent > effective_daily_limit {
            tracing::warn!(
                buyer_id = %wallet.owner.0,
                tx_id = %tx.id,
                amount = tx.amount_coins,
                wallet_daily_limit = wallet.daily_limit,
                policy_daily_limit = daily_spend_limit,
                effective_limit = effective_daily_limit,
                spent_today = wallet.spent_today,
                "🚫 [Interceptor] 日次上限超過により拒否"
            );
            return Err(NurtureError::DailyLimitExceeded {
                limit: effective_daily_limit,
                current: projected_spent,
            });
        }

        // 月次上限チェック: ポリシーとウォレットの厳しい方。effective == 0 は無制限。
        let effective_monthly_limit =
            effective_spend_limit(wallet.monthly_limit, monthly_spend_limit);
        if tx.amount_coins > 0 && effective_monthly_limit > 0 {
            let projected_monthly = match wallet.spent_this_month.checked_add(tx.amount_coins) {
                Some(v) => v,
                None => {
                    tracing::error!(
                        buyer_id = %wallet.owner.0,
                        tx_id = %tx.id,
                        spent_this_month = wallet.spent_this_month,
                        amount = tx.amount_coins,
                        "🚨 [Interceptor] spent_this_month の加算でオーバーフロー発生 — データ破損の可能性"
                    );
                    return Err(NurtureError::PolicyViolation(
                        "システムエラー: 月次支出計算のオーバーフローを検知しました".to_string(),
                    ));
                }
            };

            if projected_monthly > effective_monthly_limit {
                tracing::warn!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    amount = tx.amount_coins,
                    wallet_monthly_limit = wallet.monthly_limit,
                    policy_monthly_limit = monthly_spend_limit,
                    effective_limit = effective_monthly_limit,
                    spent_this_month = wallet.spent_this_month,
                    "🚫 [Interceptor] 月次上限超過により拒否"
                );
                return Err(NurtureError::MonthlyLimitExceeded {
                    limit: effective_monthly_limit,
                    current: projected_monthly,
                });
            }
        }

        // アノマリー検知: 高頻度取引チェック
        // NOTE: ゼロ額トランザクションにも適用される (スパム防止)
        if let Some(last_tx) = wallet.last_transaction_at {
            let elapsed = chrono::Utc::now() - last_tx;
            let elapsed_ms = elapsed.num_milliseconds();

            if elapsed_ms < 0 {
                tracing::warn!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    last_transaction_at = %last_tx,
                    "🚨 [Interceptor] 未来タイムスタンプ異常を検出"
                );
                return Err(NurtureError::PolicyViolation(
                    "タイムスタンプ異常: 未来の取引時刻が記録されています".to_string(),
                ));
            }

            // CBA: elapsed_ms >= 0 が上の分岐で保証されているが、
            // 防衛的に try_from() を使い、万が一の不整合を fail-fast する
            let elapsed_ms_u64 = u64::try_from(elapsed_ms).map_err(|_| {
                tracing::error!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    elapsed_ms = elapsed_ms,
                    "🚨 [Interceptor] elapsed_ms の u64 変換に失敗 — タイムスタンプ異常"
                );
                NurtureError::Infrastructure(
                    "タイムスタンプ演算でオーバーフローが発生しました".to_string(),
                )
            })?;
            if elapsed_ms_u64 < min_transaction_interval_ms {
                tracing::warn!(
                    buyer_id = %wallet.owner.0,
                    tx_id = %tx.id,
                    elapsed_ms = elapsed_ms,
                    min_interval_ms = min_transaction_interval_ms,
                    "🚫 [Interceptor] 高頻度取引を検出"
                );
                return Err(NurtureError::PolicyViolation(format!(
                    "取引頻度が高すぎます (最短間隔: {}ms)",
                    min_transaction_interval_ms
                )));
            }
        }

        tracing::debug!(
            buyer_id = %wallet.owner.0,
            tx_id = %tx.id,
            amount = tx.amount_coins,
            "✅ [Interceptor] プリフライトチェック通過"
        );
        Ok(())
    }
}

/// ウォレット上限とポリシー上限の厳しい方。両方 0 の場合は 0（無制限）。
fn effective_spend_limit(wallet_limit: u64, policy_limit: u64) -> u64 {
    match (wallet_limit, policy_limit) {
        (0, 0) => 0,
        (w, 0) => w,
        (0, p) => p,
        (w, p) => w.min(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use commerce_protocol::commodity::{CommodityKind, ItemDescriptor, PriceTag};
    use commerce_protocol::identity::ActorId;
    use commerce_protocol::offer::SaleMode;
    use nurture_core::coin::AiomeCoin;
    use nurture_core::policy::EconomyPolicy;
    use std::sync::Arc;
    use uuid::Uuid;

    /// テスト用トランザクションを構築するヘルパー。
    ///
    /// # `creator_points_rate` について
    /// ここでの `7000` は `Transaction::new()` 内での初期ポイント計算
    /// (`creator_points_earned`) にのみ影響する。
    /// `check_transaction()` 内のポイント検証は **ポリシー側の `creator_points_rate`** を
    /// 参照するため、テストで `EconomyPolicy::default()` (rate=7000) を使う限り
    /// 整合性が保たれる。ポリシー側のレートを変更するテストでは `tx.creator_points_earned` を
    /// 明示的に上書きすること。
    fn mock_tx(amount: u64) -> Transaction<Initiated> {
        let item = ItemDescriptor {
            id: Uuid::new_v4(),
            kind: CommodityKind::VrmAvatar,
            name: "Test".to_string(),
            description: "Desc".to_string(),
            price: PriceTag::Fixed(amount),
            creator_id: ActorId(Uuid::new_v4()),
            sale_mode: SaleMode::Instant,
            drm_enabled: false,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
            content_hash: None,
        };
        let mut tx = Transaction::new(ActorId(Uuid::new_v4()), ActorId(Uuid::new_v4()), item, 7000);
        // テスト用のダミーバージョンを付与 (Zero-Trust 防御対応)
        tx.debit_account_version = Some(1);
        tx
    }

    /// テスト用ウォレットを構築するヘルパー (DRY)
    fn mock_wallet(
        owner: commerce_protocol::identity::ActorId,
        balance: u64,
        daily_limit: u64,
        spent_today: u64,
        last_transaction_at: Option<chrono::DateTime<Utc>>,
    ) -> CoinWallet {
        mock_wallet_with_monthly(
            owner,
            balance,
            daily_limit,
            spent_today,
            0,
            0,
            last_transaction_at,
        )
    }

    fn mock_wallet_with_monthly(
        owner: commerce_protocol::identity::ActorId,
        balance: u64,
        daily_limit: u64,
        spent_today: u64,
        monthly_limit: u64,
        spent_this_month: u64,
        last_transaction_at: Option<chrono::DateTime<Utc>>,
    ) -> CoinWallet {
        CoinWallet {
            owner,
            coin: AiomeCoin {
                balance,
                lifetime_charged: balance,
                lifetime_spent: 0,
            },
            daily_limit,
            spent_today,
            monthly_limit,
            spent_this_month,
            last_reset: Utc::now(),
            last_transaction_at,
            version: 0,
        }
    }

    #[tokio::test]
    async fn test_interceptor_insufficient_balance() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(tx.buyer, 50, 1000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::InsufficientBalance {
                required: 100,
                available: 50
            })
        ));
    }

    #[tokio::test]
    async fn test_interceptor_monthly_limit_exceeded() {
        let policy_val = EconomyPolicy {
            daily_spend_limit: 100_000,
            monthly_spend_limit: 500,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet_with_monthly(tx.buyer, 10_000, 100_000, 0, 500, 450, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::MonthlyLimitExceeded {
                limit: 500,
                current: 550
            })
        ));
    }

    #[tokio::test]
    async fn test_interceptor_daily_limit_exceeded() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(tx.buyer, 1000, 50, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::DailyLimitExceeded {
                limit: 50,
                current: 100
            })
        ));
    }

    #[tokio::test]
    async fn test_interceptor_high_frequency() {
        let policy_val = EconomyPolicy {
            min_transaction_interval_ms: 5000,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(
            tx.buyer,
            1000,
            1000,
            0,
            Some(Utc::now() - chrono::Duration::milliseconds(1000)), // 1秒前
        );

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("取引頻度が高すぎます")
        ));
    }

    #[tokio::test]
    async fn test_interceptor_future_timestamp() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(
            tx.buyer,
            1000,
            1000,
            0,
            Some(Utc::now() + chrono::Duration::milliseconds(10000)), // 10秒未来
        );

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("未来の取引時刻")
        ));
    }

    /// 正常系: すべてのチェックを通過するケース (Happy Path)
    #[tokio::test]
    async fn test_interceptor_happy_path() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(tx.buyer, 5000, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(result.is_ok(), "Happy path should succeed: {:?}", result);
    }

    /// 正常系: 十分な間隔を空けた2回目の取引
    #[tokio::test]
    async fn test_interceptor_sufficient_interval() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        // デフォルト min_transaction_interval_ms = 1000ms → 2秒前なら通過
        let wallet = mock_wallet(
            tx.buyer,
            5000,
            10000,
            0,
            Some(Utc::now() - chrono::Duration::milliseconds(2000)),
        );

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "Sufficient interval should pass: {:?}",
            result
        );
    }

    /// 境界値: ゼロ額トランザクション (無料アイテム) はポリシーバイパスで通過
    #[tokio::test]
    async fn test_interceptor_zero_amount_passes() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(0); // 無料アイテム
                             // 残高ゼロでも通過すべき
        let wallet = mock_wallet(tx.buyer, 0, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(result.is_ok(), "Zero-amount tx should pass: {:?}", result);
    }

    /// 境界値: ゼロ額トランザクションは、既に日次上限を超過していても通過する
    #[tokio::test]
    async fn test_interceptor_zero_amount_bypasses_daily_limit() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(0); // 無料アイテム
                             // 既に上限 (10000) を超過している
        let wallet = mock_wallet(tx.buyer, 0, 10000, 20000, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "Zero-amount tx should bypass daily limit even if overspent: {:?}",
            result
        );
    }

    /// 境界値: 残高ぴったりの取引は通過すべき
    #[tokio::test]
    async fn test_interceptor_exact_balance() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        // 残高ぴったり
        let wallet = mock_wallet(tx.buyer, 100, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(result.is_ok(), "Exact balance should pass: {:?}", result);
    }

    /// 異常系: ウォレットの daily_limit がポリシーの daily_spend_limit より緩い場合、
    /// ポリシー側の制限が優先される
    #[tokio::test]
    async fn test_interceptor_policy_daily_limit_overrides_wallet() {
        let policy_val = EconomyPolicy {
            daily_spend_limit: 200,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        // ウォレット側は 10000 と緩いが、ポリシー側 200 が優先
        // 既に 150 消費 → 100 追加で 250 > 200
        let wallet = mock_wallet(tx.buyer, 5000, 10000, 150, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::DailyLimitExceeded { limit: 200, .. })
        ));
    }

    /// 正常系: ウォレットとポリシーの日次上限がどちらも十分な場合は通過
    #[tokio::test]
    async fn test_interceptor_both_daily_limits_ok() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(tx.buyer, 5000, 10000, 50, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "Both limits sufficient should pass: {:?}",
            result
        );
    }

    /// 境界値: 日次上限ぴったり (spent_today + amount == effective_limit) は通過すべき
    #[tokio::test]
    async fn test_interceptor_daily_limit_exact_boundary() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        // 100 + 100 = 200 == daily_limit → ぴったり通過
        let wallet = mock_wallet(tx.buyer, 5000, 200, 100, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "Exact daily limit boundary should pass: {:?}",
            result
        );
    }

    /// 境界値: 高頻度チェックで間隔がぴったり閾値 (elapsed_ms == min_interval) は通過すべき
    #[tokio::test]
    async fn test_interceptor_frequency_exact_boundary() {
        let policy_val = EconomyPolicy {
            min_transaction_interval_ms: 2000,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        // ぴったり 2000ms 前 → elapsed_ms >= min_interval → 通過
        // NOTE: テスト実行中の微小なクロックドリフトを考慮し 2100ms に設定
        let wallet = mock_wallet(
            tx.buyer,
            5000,
            10000,
            0,
            Some(Utc::now() - chrono::Duration::milliseconds(2100)),
        );

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "Exact frequency boundary should pass: {:?}",
            result
        );
    }

    /// 異常系: ポリシーの min_item_price 違反がインターセプター経由で拒否される
    #[tokio::test]
    async fn test_interceptor_policy_min_price_violation() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default())); // min_item_price = 10
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(5); // 10 未満
        let wallet = mock_wallet(tx.buyer, 5000, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("最低価格")
        ));
    }

    /// 異常系: ゼロ額トランザクション + 高頻度 → ゼロ額でもスパム防止で拒否
    #[tokio::test]
    async fn test_interceptor_zero_amount_high_frequency_rejected() {
        let policy_val = EconomyPolicy {
            min_transaction_interval_ms: 5000,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(0); // 無料アイテム
        let wallet = mock_wallet(
            tx.buyer,
            0,
            10000,
            0,
            Some(Utc::now() - chrono::Duration::milliseconds(100)), // 100ms 前
        );

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("取引頻度が高すぎます")
        ));
    }

    /// 異常系: max_single_purchase 違反
    #[tokio::test]
    async fn test_interceptor_max_single_purchase_violation() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default())); // max_single_purchase = 5000
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(6000); // 5000 超
        let wallet = mock_wallet(tx.buyer, 10000, 100000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("購入上限")
        ));
    }

    /// 異常系: ポイント付与額 (creator_points_earned) の不正操作 (マネープリンティング攻撃) を遮断する
    #[tokio::test]
    async fn test_interceptor_forged_creator_points_rejected() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default())); // creator_points_rate = 7000 (70%)
        let interceptor = EconomyInterceptor::new(policy);
        let mut tx = mock_tx(100);
        // 期待されるポイントは 70 だが、不正に 10000 に書き換える
        tx.creator_points_earned = 10000;

        let wallet = mock_wallet(tx.buyer, 5000, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("不正なポイント付与額")
        ));
    }

    /// 境界値: creator_points_rate = 0 の場合、earned > 0 は不正として拒否される
    #[tokio::test]
    async fn test_interceptor_zero_points_rate_rejects_nonzero_earned() {
        let policy_val = EconomyPolicy {
            creator_points_rate: 0, // ポイント付与なし
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let mut tx = mock_tx(100);
        // creator_points_rate=0 → expected=0。earned=1 は不正
        tx.creator_points_earned = 1;
        tx.debit_account_version = Some(1);

        let wallet = mock_wallet(tx.buyer, 5000, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("不正なポイント付与額")
        ));
    }

    /// 正常系: creator_points_rate = 0 かつ earned = 0 は正当な取引として通過
    #[tokio::test]
    async fn test_interceptor_zero_points_rate_zero_earned_passes() {
        let policy_val = EconomyPolicy {
            creator_points_rate: 0,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let mut tx = mock_tx(100);
        tx.creator_points_earned = 0;
        tx.debit_account_version = Some(1);

        let wallet = mock_wallet(tx.buyer, 5000, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "Zero rate + zero earned should pass: {:?}",
            result
        );
    }

    /// 異常系: debit_account_version が None の場合、楽観的ロックバイパスとして拒否
    #[tokio::test]
    async fn test_interceptor_missing_debit_version_rejected() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let mut tx = mock_tx(100);
        tx.debit_account_version = None; // 意図的にバージョンを欠落

        let wallet = mock_wallet(tx.buyer, 5000, 10000, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("楽観的ロック")
        ));
    }

    /// 異常系: spent_today + amount_coins が u64 をオーバーフローする場合、
    /// データ破損として拒否 (checked_add 防御の検証)
    #[tokio::test]
    async fn test_interceptor_spent_today_overflow_rejected() {
        let policy = Arc::new(tokio::sync::RwLock::new(EconomyPolicy::default()));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        // spent_today = u64::MAX → 100 を加算すると確実にオーバーフロー
        let wallet = mock_wallet(tx.buyer, 10000, u64::MAX, u64::MAX, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(matches!(
            result,
            Err(NurtureError::PolicyViolation(msg)) if msg.contains("オーバーフロー")
        ));
    }

    /// 境界値: u64::MAX の残高でも正常に処理できる (パニック・オーバーフローなし)
    /// amount=100, rate=7000 → expected_points = 70 (u64 範囲内) → Ok(())
    #[tokio::test]
    async fn test_interceptor_max_balance_no_panic() {
        let policy_val = EconomyPolicy {
            max_item_price: u64::MAX,
            max_single_purchase: u64::MAX,
            daily_spend_limit: u64::MAX,
            ..EconomyPolicy::default()
        };
        let policy = Arc::new(tokio::sync::RwLock::new(policy_val));
        let interceptor = EconomyInterceptor::new(policy);
        let tx = mock_tx(100);
        let wallet = mock_wallet(tx.buyer, u64::MAX, u64::MAX, 0, None);

        let result = interceptor.check_transaction(&tx, &wallet).await;
        assert!(
            result.is_ok(),
            "u64::MAX balance with small tx should pass without panic: {:?}",
            result
        );
    }
}
