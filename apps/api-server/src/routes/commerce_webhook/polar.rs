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
use tracing::{error, info, warn};

use super::checkout::handle_checkout_completed;
use super::invoice::apply_pending_agent_states;
use super::relay::enqueue_coin_charge_to_nurture;

/// [POST] /api/v1/commerce/webhook/polar
#[utoipa::path(
    post,
    path = "/api/v1/commerce/webhook/polar",
    responses(
        (status = 200, description = "Webhook processed successfully"),
        (status = 400, description = "Bad request / Invalid signature")
    )
)]
// auth-exempt: Polar 署名検証
pub async fn polar_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    let safe_len = body.len().clamp(0, 1048576);
    if body.len() != safe_len {
        return Err(AppError::bad_request("Payload too large"));
    }

    info!("🔗 [PolarWebhook] Received webhook request.");

    let webhook_id = headers
        .get("webhook-id")
        .ok_or_else(|| {
            warn!("⚠️ [PolarWebhook] Missing webhook-id header.");
            AppError::bad_request("Missing webhook-id header")
        })?
        .to_str()
        .map_err(|_| AppError::bad_request("Invalid webhook-id"))?;

    let webhook_timestamp = headers
        .get("webhook-timestamp")
        .ok_or_else(|| {
            warn!("⚠️ [PolarWebhook] Missing webhook-timestamp header.");
            AppError::bad_request("Missing webhook-timestamp header")
        })?
        .to_str()
        .map_err(|_| AppError::bad_request("Invalid webhook-timestamp"))?;

    let sig_header = headers
        .get("webhook-signature")
        .ok_or_else(|| {
            warn!("⚠️ [PolarWebhook] Missing webhook-signature header.");
            AppError::bad_request("Missing webhook-signature header")
        })?
        .to_str()
        .map_err(|_| AppError::bad_request("Invalid webhook-signature format"))?;

    let svix_payload = format!("{}.{}.{}", webhook_id, webhook_timestamp, body);

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        error!("❌ [PolarWebhook] Commerce Engine not enabled.");
        AppError::internal("Commerce Engine not enabled")
    })?;

    if let Err(e) = engine.verify_signature(&svix_payload, sig_header) {
        warn!("🚨 [PolarWebhook] Signature verification failed: {}", e);
        return Err(AppError::bad_request("Invalid webhook signature"));
    }

    let event_val: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        error!("❌ [PolarWebhook] Failed to parse webhook JSON: {}", e);
        AppError::bad_request("Invalid webhook JSON payload")
    })?;

    let event_id = event_val["id"].as_str().unwrap_or(webhook_id);
    let event_type = event_val["type"].as_str().unwrap_or("unknown");

    info!(
        "📦 [PolarWebhook] Verified event: {} ({})",
        event_id, event_type
    );

    // 5. 冪等性の保証とライセンス付与
    let db_pool = state.db_pool.get_inner();
    let mut tx = db_pool.begin().await.map_err(|e| {
        error!("❌ [PolarWebhook] Failed to begin transaction: {}", e);
        AppError::internal("Database error")
    })?;

    const Q_IDEMPOTENCY_SQLITE: &str =
        "INSERT INTO polar_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)";
    const Q_IDEMPOTENCY_PG: &str =
        "INSERT INTO polar_webhook_events (event_id, event_type, metadata) VALUES ($1, $2, $3)";

    let metadata_json = serde_json::to_string(&event_val["data"]).unwrap_or_else(|e| {
        warn!(
            "⚠️ [PolarWebhook] Failed to serialize event metadata: {}",
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
                "📦 [PolarWebhook] Webhook event {} inserted for processing.",
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
                    "💡 [PolarWebhook] Webhook event {} was already processed. Skipping.",
                    event_id
                );
                drop(tx);
                return Ok(StatusCode::OK);
            }
            error!(
                "❌ [PolarWebhook] Idempotency DB error {}: {:?}",
                event_id, e
            );
            return Err(AppError::internal("Database error"));
        }
    }

    let mut pending_coin_charge = None;
    let mut pending_unlock_agent = None;
    let mut pending_suspend_agent = None;

    // 6. トランザクション処理
    if event_type == "checkout.completed" {
        let object = &event_val["data"];
        let registry = state.registry.get_inner();
        pending_coin_charge =
            handle_checkout_completed(&mut tx, registry, event_id, object).await?;
    } else if event_type == "subscription.created" || event_type == "subscription.updated" {
        let object = &event_val["data"];
        let status = object["status"].as_str().unwrap_or_default();
        let agent_id_str = object["metadata"]["actor_id"].as_str().unwrap_or_default();

        if !agent_id_str.is_empty() {
            if status == "active" || status == "trialing" || status.is_empty() {
                // If status is empty (like in basic Polar payload), assume active/unlock
                pending_unlock_agent = Some(agent_id_str.to_string());
            } else if status == "past_due" || status == "unpaid" || status == "canceled" {
                pending_suspend_agent = Some(agent_id_str.to_string());
            }
        }
    } else if event_type == "subscription.deleted" {
        let object = &event_val["data"];
        let agent_id_str = object["metadata"]["actor_id"].as_str().unwrap_or_default();
        if !agent_id_str.is_empty() {
            pending_suspend_agent = Some(agent_id_str.to_string());
        }
    } else {
        info!("ℹ️ [PolarWebhook] Unhandled event type: {}", event_type);
    }

    if let Err(e) = tx.commit().await {
        error!("❌ [PolarWebhook] Failed to commit transaction: {}", e);
        return Err(AppError::internal("Transaction commit failed"));
    }

    // Apply setting mutations via job queue
    let job_queue: std::sync::Arc<dyn aiome_core::traits::JobQueue> =
        state.job_queue.get_inner().clone();
    apply_pending_agent_states(
        &job_queue,
        pending_unlock_agent.clone(),
        pending_suspend_agent.clone(),
    )
    .await;

    // Broadcast invoice events to SSE
    if let Some(agent_id_str) = pending_unlock_agent {
        if let Ok(agent_id) = uuid::Uuid::parse_str(&agent_id_str) {
            if let Some(sender) = state.event_sender.as_opt() {
                let _ = sender.send(aiome_core_contracts::events::CoreEvent::CommerceEvent {
                    event_type: "invoice.paid".to_string(),
                    agent_id,
                    amount: 0,
                    currency: "jpy".to_string(),
                    description: format!("Polar event ID: {}", event_id),
                });
                metrics::counter!("aiome_commerce_events_broadcast_total", "type" => "invoice.paid").increment(1);
            }
        }
    }

    if let Some(agent_id_str) = pending_suspend_agent {
        if let Ok(agent_id) = uuid::Uuid::parse_str(&agent_id_str) {
            if let Some(sender) = state.event_sender.as_opt() {
                let _ = sender.send(aiome_core_contracts::events::CoreEvent::CommerceEvent {
                    event_type: "invoice.payment_failed".to_string(),
                    agent_id,
                    amount: 0,
                    currency: "jpy".to_string(),
                    description: format!("Polar event ID: {}", event_id),
                });
                metrics::counter!("aiome_commerce_events_broadcast_total", "type" => "invoice.payment_failed").increment(1);
            }
        }
    }

    // Result-consistent transfer of coin charges to Nurture
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

        if let Some(sender) = state.event_sender.as_opt() {
            let _ = sender.send(aiome_core_contracts::events::CoreEvent::CommerceEvent {
                event_type: "checkout.completed".to_string(),
                agent_id: agent_uuid,
                amount,
                currency: "jpy".to_string(),
                description: format!("Polar event ID: {}", ev_id),
            });
            metrics::counter!("aiome_commerce_events_broadcast_total", "type" => "checkout.session.completed").increment(1);
        }
    }

    Ok(StatusCode::OK)
}
