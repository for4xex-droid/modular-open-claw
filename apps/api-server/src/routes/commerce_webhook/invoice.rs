/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::error::AppError;
use infrastructure::db::DatabaseTransaction;
use tracing::{error, info, warn};

// auth-exempt: Helper function (Not an endpoint)
pub async fn handle_invoice_paid<'a>(
    tx: &mut DatabaseTransaction<'a>,
    customer_id: &str,
    subscription_id: &str,
) -> Result<Option<String>, AppError> {
    if customer_id.is_empty() {
        warn!("⚠️ [StripeWebhook] Missing customer_id in invoice.paid event.");
        return Ok(None);
    }

    info!(
        "💳 [StripeWebhook] Invoice paid for subscription {}. Ensuring service unlocked for customer {}.",
        subscription_id, customer_id
    );

    const Q_FIND_SQLITE: &str = "SELECT agent_id FROM stripe_customers WHERE customer_id = ?";
    const Q_FIND_PG: &str = "SELECT agent_id FROM stripe_customers WHERE customer_id = $1";

    let row: Option<(String,)> = infrastructure::sql_tx_fetch_optional!(
        tx,
        (String,),
        sqlite: Q_FIND_SQLITE,
        pg: Q_FIND_PG,
        customer_id
    )
    .map_err(|e| {
        error!("❌ [StripeWebhook] Failed to query stripe_customers: {}", e);
        AppError::internal("DB error")
    })?;

    if let Some((agent_uuid_str,)) = row {
        info!(
            "🔓 [StripeWebhook] Queueing MCP unlock for agent: {}",
            agent_uuid_str
        );
        Ok(Some(agent_uuid_str))
    } else {
        Ok(None)
    }
}

// auth-exempt: Helper function (Not an endpoint)
pub async fn handle_invoice_payment_failed<'a>(
    tx: &mut DatabaseTransaction<'a>,
    customer_id: &str,
    subscription_id: &str,
) -> Result<Option<String>, AppError> {
    if customer_id.is_empty() {
        warn!("⚠️ [StripeWebhook] Missing customer_id in invoice.payment_failed event.");
        return Ok(None);
    }

    warn!(
        "🚨 [StripeWebhook] Invoice payment failed for subscription {}. Suspending account {}.",
        subscription_id, customer_id
    );

    const Q_FIND_SQLITE: &str = "SELECT agent_id FROM stripe_customers WHERE customer_id = ?";
    const Q_FIND_PG: &str = "SELECT agent_id FROM stripe_customers WHERE customer_id = $1";

    let row: Option<(String,)> = infrastructure::sql_tx_fetch_optional!(
        tx,
        (String,),
        sqlite: Q_FIND_SQLITE,
        pg: Q_FIND_PG,
        customer_id
    )
    .map_err(|e| {
        error!("❌ [StripeWebhook] Failed to query stripe_customers: {}", e);
        AppError::internal("DB error")
    })?;

    if let Some((agent_uuid_str,)) = row {
        warn!(
            "🔒 [StripeWebhook] Queueing MCP suspend for agent: {}",
            agent_uuid_str
        );
        Ok(Some(agent_uuid_str))
    } else {
        Ok(None)
    }
}

// auth-exempt: Helper function (Not an endpoint)
pub async fn apply_pending_agent_states(
    job_queue: &std::sync::Arc<dyn aiome_core::traits::JobQueue>,
    pending_unlock_agent: Option<String>,
    pending_suspend_agent: Option<String>,
) {
    if let Some(agent_uuid_str) = pending_unlock_agent {
        if let Err(e) = job_queue
            .update_setting(
                &format!("agency.{}.mcp_suspended", agent_uuid_str),
                "false",
                "billing",
                false,
            )
            .await
        {
            error!(
                "❌ Failed to unlock MCP for agent {}: {}",
                agent_uuid_str, e
            );
        }
    }

    if let Some(agent_uuid_str) = pending_suspend_agent {
        if let Err(e) = job_queue
            .update_setting(
                &format!("agency.{}.mcp_suspended", agent_uuid_str),
                "true",
                "billing",
                false,
            )
            .await
        {
            error!(
                "❌ Failed to suspend MCP for agent {}: {}",
                agent_uuid_str, e
            );
        }
    }
}
