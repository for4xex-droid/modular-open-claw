/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::error::AppError;
use infrastructure::db::DatabaseTransaction;
use infrastructure::registry::RegistryManager;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Platform fee rate applied to revenue splits.
/// Change this value to adjust the Aiome platform commission percentage.
const PLATFORM_FEE_RATE: f64 = 0.15;

// auth-exempt: Helper function (Not an endpoint)
pub async fn handle_checkout_completed<'a>(
    tx: &mut DatabaseTransaction<'a>,
    registry: &std::sync::Arc<RegistryManager>,
    event_id: &str,
    object: &serde_json::Value,
) -> Result<Option<(Uuid, u64, String)>, AppError> {
    let agent_id_str = object["metadata"]["agent_id"].as_str();
    let asset_id_str = object["metadata"]["asset_id"].as_str();
    let mode = object["mode"].as_str().unwrap_or("");
    let customer_id = object["customer"].as_str().unwrap_or("");

    // Pro subscription checkout (mode=subscription) has no asset_id — license unlock
    // is driven by invoice.paid / subscription.updated via stripe_customers mapping.
    if mode == "subscription" || asset_id_str.is_none() {
        if let Some(a) = agent_id_str {
            if !customer_id.is_empty() {
                let agent_uuid = Uuid::parse_str(a).map_err(|e| {
                    warn!("⚠️ [StripeWebhook] Invalid agent_id UUID '{}': {}", a, e);
                    AppError::bad_request("Invalid agent_id in event metadata")
                })?;
                upsert_stripe_customer(tx, agent_uuid, customer_id).await?;
                info!(
                    "✅ [StripeWebhook] Subscription checkout completed: agent {} customer {}",
                    agent_uuid, customer_id
                );
            } else {
                info!(
                    "ℹ️ [StripeWebhook] Subscription checkout completed for agent {} (no customer id in payload)",
                    a
                );
            }
        } else if !customer_id.is_empty() {
            info!(
                "ℹ️ [StripeWebhook] Subscription checkout completed for customer {} (agent_id not in metadata; mapping exists from session create)",
                customer_id
            );
        } else {
            warn!(
                "⚠️ [StripeWebhook] checkout.session.completed event {} missing agent_id, asset_id, and customer",
                event_id
            );
        }
        return Ok(None);
    }

    match (agent_id_str, asset_id_str) {
        (Some(a), Some(asset)) => {
            let agent_uuid = Uuid::parse_str(a).map_err(|e| {
                warn!("⚠️ [StripeWebhook] Invalid agent_id UUID '{}': {}", a, e);
                AppError::bad_request("Invalid agent_id in event metadata")
            })?;
            let asset_uuid = Uuid::parse_str(asset).map_err(|e| {
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
            let charge_for_coin = match registry.get_asset(asset_uuid).await {
                Ok(asset_manifest) => {
                    let amount = object["amount_total"]
                        .as_i64()
                        .or_else(|| i64::try_from(asset_manifest.price_coins).ok())
                        .unwrap_or_else(|| {
                            warn!(
                                "⚠️ [StripeWebhook] No amount_total or price_coins for event {}. Defaulting to 0.",
                                event_id
                            );
                            0
                        });
                    if amount > 0 {
                        if let Err(e) = aiome_commerce::splitter::RevenueSplitter::split_revenue(
                            &mut *tx,
                            event_id,
                            amount,
                            asset_manifest.creator_id,
                            PLATFORM_FEE_RATE,
                        )
                        .await
                        {
                            error!("❌ [StripeWebhook] Failed to split revenue: {}", e);
                            return Err(AppError::internal("Revenue split failed"));
                        }
                        info!("💸 [StripeWebhook] Revenue split completed: tx_id={}, amount={}, creator={}", event_id, amount, asset_manifest.creator_id);
                    }
                    amount
                }
                Err(e) => {
                    error!(
                        "⚠️ [StripeWebhook] Failed to get asset {} for revenue split: {}",
                        asset_uuid, e
                    );
                    return Err(AppError::internal(
                        "Failed to retrieve asset for revenue split",
                    ));
                }
            };

            // 6b. ライセンス付与
            if let Err(e) = registry
                .grant_license_with_tx(tx, agent_uuid, asset_uuid, event_id.to_string())
                .await
            {
                error!("❌ [StripeWebhook] Failed to grant license: {}", e);
                return Err(AppError::internal("License grant failed"));
            }

            if charge_for_coin > 0 {
                if let Ok(safe_charge) = u64::try_from(charge_for_coin) {
                    return Ok(Some((agent_uuid, safe_charge, event_id.to_string())));
                } else {
                    warn!(
                        "⚠️ [StripeWebhook] Coin charge amount {} overflow, skipping.",
                        charge_for_coin
                    );
                }
            }
            Ok(None)
        }
        _ => {
            warn!(
                "⚠️ [StripeWebhook] checkout.session.completed event {} missing agent_id/asset_id metadata (non-subscription)",
                event_id
            );
            Ok(None)
        }
    }
}

/// Ensures stripe_customers maps agent_id ↔ customer_id (idempotent).
async fn upsert_stripe_customer<'a>(
    tx: &mut DatabaseTransaction<'a>,
    agent_id: Uuid,
    customer_id: &str,
) -> Result<(), AppError> {
    const Q_UPSERT_SQLITE: &str =
        "INSERT INTO stripe_customers (customer_id, agent_id) VALUES (?, ?) \
        ON CONFLICT(customer_id) DO UPDATE SET agent_id = excluded.agent_id";
    const Q_UPSERT_PG: &str =
        "INSERT INTO stripe_customers (customer_id, agent_id) VALUES ($1, $2) \
        ON CONFLICT(customer_id) DO UPDATE SET agent_id = EXCLUDED.agent_id";

    let agent_str = agent_id.to_string();
    infrastructure::sql_tx_exec!(
        tx,
        sqlite: Q_UPSERT_SQLITE,
        pg: Q_UPSERT_PG,
        customer_id,
        agent_str.as_str()
    )
    .map_err(|e| {
        error!(
            "❌ [StripeWebhook] Failed to upsert stripe_customers: {}",
            e
        );
        AppError::internal("Failed to upsert stripe customer mapping")
    })?;
    Ok(())
}
