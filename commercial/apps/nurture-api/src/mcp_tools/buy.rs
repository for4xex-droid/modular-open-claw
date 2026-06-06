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

use crate::state::SharedState;
use chrono;
use commerce_protocol::error::NurtureError;
use commerce_protocol::mcp_commerce::{BuyRequest, BuyResponse};
use commerce_protocol::transaction::Transaction;

pub async fn handle_buy(state: SharedState, req: BuyRequest) -> Result<BuyResponse, NurtureError> {
    // 0. 冪等性チェック
    if let Some(key) = &req.idempotency_key {
        if let Some(cached_res) = state.idempotency.get_response(key).await? {
            if let Some(res) = cached_res {
                // 保存済みのレスポンスがあればデシリアライズして返す
                return serde_json::from_str(&res.body).map_err(|e| {
                    NurtureError::Infrastructure(format!("冪等性レスポンス復元失敗: {}", e))
                });
            } else {
                // 予約済みだが未完了 = 二重送信
                return Err(NurtureError::IdempotencyConflict { key: key.clone() });
            }
        }
        // キーを予約
        state
            .idempotency
            .reserve_key(key, chrono::Duration::minutes(30))
            .await?;
    }

    let mut retries = 0;
    let max_retries = 3;

    let result = loop {
        match execute_purchase_core(state.clone(), &req).await {
            Ok(res) => break Ok(res),
            Err(NurtureError::OptimisticLockConflict { .. }) if retries < max_retries => {
                retries += 1;
                tracing::warn!("楽観的ロック衝突のためリトライ中 ({}回目)...", retries);
                tokio::time::sleep(std::time::Duration::from_millis(50 * retries)).await;
            }
            Err(e) => break Err(e),
        }
    };

    // 冪等性レスポンスの保存
    if let Ok(res) = &result {
        if let Some(key) = &req.idempotency_key {
            match serde_json::to_string(res) {
                Ok(body) => {
                    if let Err(e) = state.idempotency.save_response(key, 200, body).await {
                        tracing::error!(
                            "❌ [Buy] Failed to save idempotency response for key {}: {:?}",
                            key,
                            e
                        );
                    }
                }
                Err(e) => {
                    // シリアライズ失敗時は保存をスキップ。
                    // 次回同一キーでのリトライ時は reserve_key が期限切れ後に再実行される。
                    // 空文字列を保存して復元不能な状態を作るよりも安全。
                    tracing::error!(
                        "❌ [Buy] Failed to serialize BuyResponse for idempotency key {}: {:?}",
                        key,
                        e
                    );
                }
            }
        }
    }

    result
}

async fn execute_purchase_core(
    state: SharedState,
    req: &BuyRequest,
) -> Result<BuyResponse, NurtureError> {
    use nurture_bridge::commerce::CommerceEngine;

    // 1. 商品情報の取得
    let item = state.marketplace.get_item(&req.item_id).await?;

    // 2. CSAM 安全性チェック (🚨 全購入経路で必須)
    let verdict = state
        .csam_pipeline
        .run_all(&req.item_id, &item.metadata)
        .await?;
    if let nurture_infra::csam::ScanVerdict::Rejected { reason, layer, .. } = verdict {
        tracing::warn!(
            "🚨 CSAM Rejected via API buy: item={}, layer={}, reason={}",
            req.item_id,
            layer,
            reason
        );
        return Err(NurtureError::CsamRejected {
            item_id: req.item_id,
            reason: format!("[{}] {}", layer, reason),
        });
    }

    // 3. 購入者のウォレット取得
    let buyer_id = req.buyer;

    // 🔴 自己売買防止（ウォッシュトレード攻撃対策）
    if buyer_id == item.creator_id {
        return Err(NurtureError::PolicyViolation(
            "自分のアイテムを購入することはできません".to_string(),
        ));
    }

    let wallet = state.ledger.get_balance(&buyer_id).await?;

    // 4. トランザクション生成 (Initiated)
    // Note: RwLockReadGuard を .await ポイントをまたいで保持しないよう、
    // 必要な値をコピーして即座にガードを解放する。
    let creator_points_rate = state.policy.read().await.creator_points_rate;
    let mut tx_initiated =
        Transaction::new(buyer_id, item.creator_id, item.clone(), creator_points_rate);

    // 楽観的ロック用のバージョンをセット (Interceptor が検証するため、check_transaction 前に必要)
    tx_initiated.debit_account_version = Some(wallet.version);

    // 5. セキュリティチェック (EconomyInterceptor)
    state
        .interceptor
        .check_transaction(&tx_initiated, &wallet)
        .await?;

    // 6. 承認状態へ移行 (Authorized) — debit_account_version は authorize() で引き継がれる
    let tx_authorized = tx_initiated.authorize();

    let use_escrow = req.use_escrow.unwrap_or(false);
    let mut escrow_id = None;

    // 7. 決済実行 (SettlementProtocol または Escrow)
    let receipt = if use_escrow {
        let price_amount = match item.price {
            commerce_protocol::PriceTag::Fixed(coins) => coins,
            _ => {
                return Err(NurtureError::PolicyViolation(
                    "Dynamic pricing is not supported for escrow".to_string(),
                ))
            }
        };

        // 信託決済の実行 (残高差し引きとレコード作成)
        let eid = state
            .commerce_engine
            .escrow_create(buyer_id.0, price_amount)
            .await
            .map_err(|e| NurtureError::Infrastructure(format!("Escrow creation failed: {}", e)))?;

        escrow_id = Some(eid);

        // エスクロー用の仮 Receipt — transaction_id は承認済みトランザクションの ID を使用し、
        // 台帳監査上の一貫性を保つ。
        // Note: エスクローは pending 状態のためトランザクションを Settled に遷移させない。
        commerce_protocol::settlement::SettlementReceipt {
            id: uuid::Uuid::new_v4(),
            transaction_id: tx_authorized.id,
            coin_debited: price_amount,
            points_credited: 0,
            settled_at: chrono::Utc::now(),
        }
    } else {
        // 即時決済: Settlement 実行後、トランザクションを Settled に遷移
        let r = state.settlement.settle(&tx_authorized).await?;
        let _tx_settled = tx_authorized.settle();
        r
    };

    // 8. DRM ライセンス発行 (🔴 Step 2 連携)
    let mut license_id = None;
    if item.drm_enabled {
        let license_type = match item.sale_mode {
            commerce_protocol::offer::SaleMode::Subscription { .. } => "subscription",
            commerce_protocol::offer::SaleMode::Instant => "perpetual",
        };

        // アーキテクチャの境界を遵守し、直接DBを叩くのではなくCommerceEngineを経由する
        let license_result = state
            .commerce_engine
            .register_license(
                buyer_id.0,
                item.id,
                &receipt.transaction_id.to_string(),
                license_type,
            )
            .await;

        let lid_str = match license_result {
            Ok(lid) => lid,
            Err(e) => {
                // ⚠️ 部分的失敗: エスクロー作成済みだがライセンス発行に失敗。
                // エスクローは TTL (24h) で自動返金されるが、運用者への通知が必要。
                if escrow_id.is_some() {
                    tracing::error!(
                        "🚨 PARTIAL FAILURE: Escrow {} created but license registration failed for item {}. \
                         Escrow will auto-refund after TTL. Error: {}",
                        escrow_id.as_deref().unwrap_or("unknown"),
                        item.id,
                        e
                    );
                }
                return Err(NurtureError::Infrastructure(format!(
                    "Failed to register license via bridge: {}",
                    e
                )));
            }
        };

        license_id = Some(uuid::Uuid::parse_str(&lid_str).map_err(|e| {
            NurtureError::Infrastructure(format!(
                "License ID parse failed (bridge returned invalid UUID '{}'): {}",
                lid_str, e
            ))
        })?);
    }

    Ok(BuyResponse {
        transaction_id: receipt.transaction_id,
        receipt,
        license_id,
        escrow_id,
    })
}
