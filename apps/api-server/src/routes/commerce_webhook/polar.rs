use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use tracing::{error, info, warn};

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
        "📦 [PolarWebhook] Processing event: {} ({})",
        event_id, event_type
    );

    if let Err(e) = engine
        .process_webhook(event_id, event_type, &event_val)
        .await
    {
        error!("❌ [PolarWebhook] Event processing failed: {}", e);
        return Err(AppError::internal("Event processing failed"));
    }

    Ok(StatusCode::OK)
}
