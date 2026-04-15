/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use infrastructure::db::{DatabasePool, DatabaseTransaction};
use tracing::{error, info, warn};

/// [POST] /api/v1/commerce/webhook
#[utoipa::path(
    post,
    path = "/api/v1/commerce/webhook",
    responses(
        (status = 200, description = "Webhook processed successfully"),
        (status = 400, description = "Bad request / Invalid signature")
    )
)]
// auth-exempt: Stripe 署名検証
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    info!("🔗 [StripeWebhook] Received webhook request.");

    // 1. Stripe-Signature ヘッダーの取得
    let sig_header = headers.get("stripe-signature").ok_or_else(|| {
        warn!("⚠️ [StripeWebhook] Missing stripe-signature header.");
        AppError::bad_request("Missing stripe-signature header")
    })?;

    let sig_str = sig_header
        .to_str()
        .map_err(|_| AppError::bad_request("Invalid stripe-signature header format"))?;

    // 2. CommerceEngine の取得
    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        error!("❌ [StripeWebhook] Commerce Engine not enabled.");
        AppError::internal("Commerce Engine not enabled")
    })?;

    // 3. 署名検証 (Gate 2/Expert 2) + リプレイアタック防止 (内部での timestamp 検証)
    if let Err(e) = engine.verify_signature(&body, sig_str) {
        warn!("🚨 [StripeWebhook] Signature verification failed: {}", e);
        return Err(AppError::bad_request("Invalid webhook signature"));
    }

    // 4. イベントのパース
    let event_val: serde_json::Value = match serde_json::from_str(&body) {
        Ok(ev) => ev,
        Err(e) => {
            error!("❌ [StripeWebhook] Failed to parse webhook JSON: {}", e);
            // CWE-209: パースエラーの詳細をレスポンスに露出しない
            return Err(AppError::bad_request("Invalid webhook JSON payload"));
        }
    };

    // 必須フィールドの早期検証 — 空 event_id での INSERT は冪等性テーブルを汚染する
    let event_id = event_val["id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            warn!("⚠️ [StripeWebhook] Event missing 'id' field.");
            AppError::bad_request("Event missing required 'id' field")
        })?;
    let event_type = event_val["type"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            warn!("⚠️ [StripeWebhook] Event missing 'type' field.");
            AppError::bad_request("Event missing required 'type' field")
        })?;
    info!(
        "📦 [StripeWebhook] Verified event: {} ({})",
        event_id, event_type
    );

    // 5. 冪等性の保証とライセンス付与 (単一トランザクション / Phase 11)
    let db_pool = state.job_queue.get_pool();
    let mut tx = db_pool.begin().await.map_err(|e| {
        error!("❌ [StripeWebhook] Failed to begin transaction: {}", e);
        AppError::internal("Database error")
    })?;

    // 冪等性チェック (G-47 Mitigation: RDBMS Agnostic)
    let q_idempotency =
        format!(
        "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES ({}, {}, {})",
        db_pool.ph(0), db_pool.ph(1), db_pool.ph(2)
    );
    let metadata_json = serde_json::to_string(&event_val["data"]["object"]).unwrap_or_else(|e| {
        warn!(
            "⚠️ [StripeWebhook] Failed to serialize event metadata: {}",
            e
        );
        String::from("{}")
    });

    let result = match &mut tx {
        DatabaseTransaction::Sqlite(itx) => sqlx::query(&q_idempotency)
            .bind(event_id)
            .bind(event_type)
            .bind(&metadata_json)
            .execute(&mut **itx)
            .await
            .map(|r| r.rows_affected()),
        DatabaseTransaction::Postgres(itx) => sqlx::query(&q_idempotency)
            .bind(event_id)
            .bind(event_type)
            .bind(&metadata_json)
            .execute(&mut **itx)
            .await
            .map(|r| r.rows_affected()),
    };

    match result {
        Ok(_) => {
            info!(
                "📦 [StripeWebhook] Webhook event {} inserted for processing.",
                event_id
            );
        }
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            info!(
                "💡 [StripeWebhook] Webhook event {} was already processed. Skipping.",
                event_id
            );
            // 明示的に破棄 — SQLx の Drop は自動 rollback するが意図を明示
            drop(tx);
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            error!(
                "❌ [StripeWebhook] Idempotency DB error {}: {}",
                event_id, e
            );
            return Err(AppError::internal("Database error"));
        }
    }

    // 6. トランザクション処理 (ライセンス付与等)
    if event_type == "checkout.session.completed" {
        let object = &event_val["data"]["object"];
        let agent_id_str = object["metadata"]["agent_id"].as_str();
        let asset_id_str = object["metadata"]["asset_id"].as_str();

        match (agent_id_str, asset_id_str) {
            (Some(a), Some(asset)) => {
                // NOTE: UUID パース失敗で 400 を返す場合、冪等性テーブルには既に INSERT 済み。
                // リトライ時は重複としてスキップされるが、Stripe は同一 event_id を再送する
                // ため、元データの metadata 修正後に再処理される保証はない。
                // 運用上は Stripe Dashboard からの手動再送で対処する想定。
                let agent_uuid = uuid::Uuid::parse_str(a).map_err(|e| {
                    warn!("⚠️ [StripeWebhook] Invalid agent_id UUID '{}': {}", a, e);
                    AppError::bad_request("Invalid agent_id in event metadata")
                })?;
                let asset_uuid = uuid::Uuid::parse_str(asset).map_err(|e| {
                    warn!(
                        "⚠️ [StripeWebhook] Invalid asset_id UUID '{}': {}",
                        asset, e
                    );
                    AppError::bad_request("Invalid asset_id in event metadata")
                })?;

                info!(
                    "💳 [StripeWebhook] Processing License Grant: Agent {} -> Asset {}",
                    agent_uuid, asset_uuid
                );

                // 6a. 収益分配 (Revenue Split)
                match state.registry.get_asset(asset_uuid).await {
                    Ok(asset_manifest) => {
                        let amount = object["amount_total"]
                            .as_i64()
                            .unwrap_or(asset_manifest.price_coins as i64);
                        if amount > 0 {
                            if let Err(e) =
                                aiome_commerce::splitter::RevenueSplitter::split_revenue(
                                    &mut tx,
                                    event_id,
                                    amount,
                                    asset_manifest.creator_id,
                                    0.15, // 15% platform fee
                                )
                                .await
                            {
                                if let Err(rb_err) = tx.rollback().await {
                                    error!("❌ [StripeWebhook] Rollback also failed: {}", rb_err);
                                }
                                error!("❌ [StripeWebhook] Failed to split revenue: {}", e);
                                return Err(AppError::internal("Revenue split failed"));
                            }
                            info!("💸 [StripeWebhook] Revenue split completed: tx_id={}, amount={}, creator={}", event_id, amount, asset_manifest.creator_id);
                        }
                    }
                    Err(e) => {
                        error!(
                            "⚠️ [StripeWebhook] Failed to get asset {} for revenue split: {}",
                            asset_uuid, e
                        );
                        if let Err(rb_err) = tx.rollback().await {
                            error!("❌ [StripeWebhook] Rollback also failed: {}", rb_err);
                        }
                        return Err(AppError::internal(
                            "Failed to retrieve asset for revenue split",
                        ));
                    }
                }

                // 6b. ライセンス付与
                if let Err(e) = state
                    .registry
                    .grant_license_with_tx(&mut tx, agent_uuid, asset_uuid, event_id.to_string())
                    .await
                {
                    if let Err(rb_err) = tx.rollback().await {
                        error!("❌ [StripeWebhook] Rollback also failed: {}", rb_err);
                    }
                    error!("❌ [StripeWebhook] Failed to grant license: {}", e);
                    return Err(AppError::internal("License grant failed"));
                }
            }
            _ => {
                // 支払い完了イベントなのに metadata が欠落 — サイレント失敗を防止
                error!(
                    "❌ [StripeWebhook] checkout.session.completed event {} missing agent_id/asset_id metadata",
                    event_id
                );
                if let Err(rb_err) = tx.rollback().await {
                    error!("❌ [StripeWebhook] Rollback also failed: {}", rb_err);
                }
                return Err(AppError::internal(
                    "Checkout event missing required metadata",
                ));
            }
        }
    } else {
        info!("ℹ️ [StripeWebhook] Unhandled event type: {}", event_type);
    }

    if let Err(e) = tx.commit().await {
        error!("❌ [StripeWebhook] Failed to commit transaction: {}", e);
        return Err(AppError::internal("Transaction commit failed"));
    }

    Ok(StatusCode::OK)
}
