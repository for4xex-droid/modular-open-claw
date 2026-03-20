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
    
    let sig_str = sig_header.to_str().map_err(|_| {
        AppError::bad_request("Invalid stripe-signature header format")
    })?;

    // 2. CommerceEngine の取得
    let engine = state.commerce_engine.as_ref().ok_or_else(|| {
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
    info!("📦 [StripeWebhook] Verified event: {} ({})", event_id, event_type);

    // 5. 冪等性の保証 (Gate 2/Expert 2)
    let payload_val = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
    if let Err(e) = engine.process_webhook(event_id, event_type, &payload_val).await {
        error!("❌ [StripeWebhook] Idempotency processing failed for {}: {}", event_id, e);
        return Err(AppError::internal("Webhook processing failed"));
    }

    // 6. トランザクション処理 & AuditLedger 記録 (Gate 2/perfect-plan 追加対応)
    match event.type_ {
        stripe::EventType::CheckoutSessionCompleted => {
            // TODO: (Phase 10.2) Checkout完了時の処理
            // VoiceKeyVault を用いたライセンス付与、AiomeLedger への記録など
            info!("💳 [StripeWebhook] Checkout session completed processing stub.");
        }
        _ => {
            info!("ℹ️ [StripeWebhook] Unhandled event type: {}", event_type);
        }
    }

    Ok(StatusCode::OK)
}
