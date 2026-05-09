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

use super::checkout::handle_checkout_completed;
use super::invoice::{
    apply_pending_agent_states, handle_invoice_paid, handle_invoice_payment_failed,
};
use super::relay::enqueue_coin_charge_to_nurture;

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
#[tracing::instrument(skip_all, fields(path = "/api/v1/commerce/webhook"))]
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    let safe_len = body.len().clamp(0, 1048576);
    if body.len() != safe_len {
        return Err(AppError::bad_request("Payload too large"));
    }

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

    // 3. 署名検証 (Gate 2/Expert 2) + リプレイアタック防止
    if let Err(e) = engine.verify_signature(&body, sig_str) {
        warn!("🚨 [StripeWebhook] Signature verification failed: {}", e);
        return Err(AppError::bad_request("Invalid webhook signature"));
    }

    // 4. イベントのパース
    let event_val: serde_json::Value = match serde_json::from_str(&body) {
        Ok(ev) => ev,
        Err(e) => {
            error!("❌ [StripeWebhook] Failed to parse webhook JSON: {}", e);
            return Err(AppError::bad_request("Invalid webhook JSON payload"));
        }
    };

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
    let db_pool = state.db_pool.get_inner();
    let mut tx = db_pool.begin().await.map_err(|e| {
        error!("❌ [StripeWebhook] Failed to begin transaction: {}", e);
        AppError::internal("Database error")
    })?;

    const Q_IDEMPOTENCY_SQLITE: &str =
        "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)";
    const Q_IDEMPOTENCY_PG: &str =
        "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES ($1, $2, $3)";

    let metadata_json = serde_json::to_string(&event_val["data"]["object"]).unwrap_or_else(|e| {
        warn!(
            "⚠️ [StripeWebhook] Failed to serialize event metadata: {}",
            e
        );
        String::from("{}")
    });

    let result = infrastructure::sql_tx_exec!(
        &mut tx,
        sqlite: Q_IDEMPOTENCY_SQLITE,
        pg: Q_IDEMPOTENCY_PG,
        event_id,
        event_type,
        &metadata_json
    );

    match result {
        Ok(_) => {
            info!(
                "📦 [StripeWebhook] Webhook event {} inserted for processing.",
                event_id
            );
        }
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("unique constraint")
                || err_str.contains("duplicate key")
                || err_str.contains("unique_violation")
            {
                info!(
                    "💡 [StripeWebhook] Webhook event {} was already processed. Skipping.",
                    event_id
                );
                drop(tx);
                return Ok(StatusCode::OK);
            }
            error!(
                "❌ [StripeWebhook] Idempotency DB error {}: {:?}",
                event_id, e
            );
            return Err(AppError::internal("Database error"));
        }
    }

    let mut pending_coin_charge = None;
    let mut pending_unlock_agent = None;
    let mut pending_suspend_agent = None;

    // 6. トランザクション処理 (ライセンス付与等)
    if event_type == "checkout.session.completed" {
        let object = &event_val["data"]["object"];
        let registry = state.registry.get_inner();
        pending_coin_charge =
            handle_checkout_completed(&mut tx, registry, event_id, object).await?;
    } else if event_type == "invoice.paid" {
        let object = &event_val["data"]["object"];
        let customer_id = object["customer"].as_str().unwrap_or_default();
        let subscription_id = object["subscription"].as_str().unwrap_or_default();
        pending_unlock_agent = handle_invoice_paid(&mut tx, customer_id, subscription_id).await?;
    } else if event_type == "invoice.payment_failed" {
        let object = &event_val["data"]["object"];
        let customer_id = object["customer"].as_str().unwrap_or_default();
        let subscription_id = object["subscription"].as_str().unwrap_or_default();
        pending_suspend_agent =
            handle_invoice_payment_failed(&mut tx, customer_id, subscription_id).await?;
    } else {
        info!("ℹ️ [StripeWebhook] Unhandled event type: {}", event_type);
    }

    if let Err(e) = tx.commit().await {
        error!("❌ [StripeWebhook] Failed to commit transaction: {}", e);
        return Err(AppError::internal("Transaction commit failed"));
    }

    let job_queue: std::sync::Arc<dyn aiome_core::traits::JobQueue> =
        state.job_queue.get_inner().clone();
    apply_pending_agent_states(
        &job_queue,
        pending_unlock_agent.clone(),
        pending_suspend_agent.clone(),
    )
    .await;

    // 📢 Broadcast invoice events to SSE
    if let Some(agent_id_str) = pending_unlock_agent {
        match uuid::Uuid::parse_str(&agent_id_str) {
            Ok(agent_id) => {
                if let Some(sender) = state.event_sender.as_opt() {
                    let _ = sender.send(aiome_core_contracts::events::CoreEvent::CommerceEvent {
                        event_type: "invoice.paid".to_string(),
                        agent_id,
                        amount: 0, // Invoices don't directly add coins here
                        currency: "jpy".to_string(),
                        description: format!("Stripe event ID: {}", event_id),
                    });
                    metrics::counter!("aiome_commerce_events_broadcast_total", "type" => "invoice.paid").increment(1);
                }
            }
            Err(e) => {
                error!(
                    "❌ [StripeWebhook] Failed to parse agent_id UUID '{}': {}",
                    agent_id_str, e
                );
            }
        }
    }
    if let Some(agent_id_str) = pending_suspend_agent {
        match uuid::Uuid::parse_str(&agent_id_str) {
            Ok(agent_id) => {
                if let Some(sender) = state.event_sender.as_opt() {
                    let _ = sender.send(aiome_core_contracts::events::CoreEvent::CommerceEvent {
                        event_type: "invoice.payment_failed".to_string(),
                        agent_id,
                        amount: 0,
                        currency: "jpy".to_string(),
                        description: format!("Stripe event ID: {}", event_id),
                    });
                    metrics::counter!("aiome_commerce_events_broadcast_total", "type" => "invoice.payment_failed").increment(1);
                }
            }
            Err(e) => {
                error!(
                    "❌ [StripeWebhook] Failed to parse agent_id UUID '{}': {}",
                    agent_id_str, e
                );
            }
        }
    }

    // 6c. Nurture へのコインチャージ転送 (結果整合性保証)
    if let Some((agent_uuid, amount, ev_id)) = pending_coin_charge {
        let http_client = state.http_client.get_inner().clone();
        enqueue_coin_charge_to_nurture(
            http_client,
            db_pool.clone(),
            state.nurture_url.clone(),
            state.nurture_internal_secret.clone(),
            agent_uuid,
            amount,
            ev_id.clone(),
        )
        .await;

        // 📢 Broadcast CommerceEvent to SSE (Phase B)
        if let Some(sender) = state.event_sender.as_opt() {
            let _ = sender.send(aiome_core_contracts::events::CoreEvent::CommerceEvent {
                event_type: "checkout.session.completed".to_string(),
                agent_id: agent_uuid,
                amount,
                currency: "jpy".to_string(),
                description: format!("Stripe event ID: {}", ev_id),
            });
            metrics::counter!("aiome_commerce_events_broadcast_total", "type" => "checkout.session.completed").increment(1);
        }
    }

    Ok(StatusCode::OK)
}
