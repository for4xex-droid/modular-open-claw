/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
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
    let event: stripe::Event = match serde_json::from_str(&body) {
        Ok(ev) => ev,
        Err(e) => {
            error!("❌ [StripeWebhook] Failed to parse webhook JSON: {}", e);
            return Err(AppError::bad_request("Invalid webhook JSON payload"));
        }
    };

    let event_id = event.id.as_str();
    let event_type_str = format!("{:?}", event.type_);
    let event_type = event_type_str.as_str();
    info!(
        "📦 [StripeWebhook] Verified event: {} ({})",
        event_id, event_type
    );

    // 5. 冪等性の保証とライセンス付与 (単一トランザクション / Phase 11)
    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?;
    let mut tx = pool.begin().await.map_err(|e| {
        error!("❌ [StripeWebhook] Failed to begin transaction: {}", e);
        AppError::internal("Database error")
    })?;

    // 冪等性チェック
    let result = sqlx::query(
        "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)",
    )
    .bind(event_id)
    .bind(event_type)
    .bind(serde_json::to_string(&event.data.object).unwrap_or_default())
    .execute(&mut *tx)
    .await;

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
    if event.type_ == stripe::EventType::CheckoutSessionCompleted {
        if let Some(session) = match &event.data.object {
            stripe::EventObject::CheckoutSession(s) => Some(s),
            _ => None,
        } {
            let agent_id_str = session.metadata.as_ref().and_then(|m| m.get("agent_id"));
            let asset_id_str = session.metadata.as_ref().and_then(|m| m.get("asset_id"));

            if let (Some(a), Some(asset)) = (agent_id_str, asset_id_str) {
                if let (Ok(agent_uuid), Ok(asset_uuid)) =
                    (uuid::Uuid::parse_str(a), uuid::Uuid::parse_str(asset))
                {
                    info!(
                        "💳 [StripeWebhook] Processing License Grant: Agent {} -> Asset {}",
                        agent_uuid, asset_uuid
                    );

                    // 6a. 収益分配 (Revenue Split)
                    match state.registry.get_asset(asset_uuid).await {
                        Ok(asset_manifest) => {
                            let amount = session
                                .amount_total
                                .unwrap_or(asset_manifest.price_coins as i64);
                            if amount > 0 {
                                if let Err(e) = infrastructure::commerce::splitter::RevenueSplitter::split_revenue(
                                    &mut tx,
                                    event_id,
                                    amount,
                                    asset_manifest.creator_id,
                                    0.15 // 15% platform fee
                                ).await {
                                    let _ = tx.rollback().await;
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
                            // Continue to grant license even if asset metadata fails?
                            // Better to fail to maintain consistency.
                            let _ = tx.rollback().await;
                            return Err(AppError::internal(
                                "Failed to retrieve asset for revenue split",
                            ));
                        }
                    }

                    // 6b. ライセンス付与
                    if let Err(e) = state
                        .registry
                        .grant_license_with_tx(
                            &mut tx,
                            agent_uuid,
                            asset_uuid,
                            event_id.to_string(),
                        )
                        .await
                    {
                        let _ = tx.rollback().await;
                        error!("❌ [StripeWebhook] Failed to grant license: {}", e);
                        return Err(AppError::internal("License grant failed"));
                    }
                }
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
