/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
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

/// OP-059: `pro_monthly_kc_allowance` 設定値を解釈する。
/// 未設定・非数値・0 は「付与なし」。上限 1,000,000 KC でクランプ（設定ミスによる過剰付与防止）。
// auth-exempt: Helper function (Not an endpoint)
pub fn parse_monthly_allowance(setting: Option<String>) -> u64 {
    const MAX_ALLOWANCE: u64 = 1_000_000;
    setting
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .min(MAX_ALLOWANCE)
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

#[cfg(test)]
mod tests {
    use super::parse_monthly_allowance;

    #[test]
    fn test_parse_monthly_allowance_valid() {
        assert_eq!(parse_monthly_allowance(Some("1000".to_string())), 1000);
        assert_eq!(parse_monthly_allowance(Some(" 250 ".to_string())), 250);
    }

    #[test]
    fn test_parse_monthly_allowance_unset_or_invalid_is_zero() {
        assert_eq!(parse_monthly_allowance(None), 0);
        assert_eq!(parse_monthly_allowance(Some("".to_string())), 0);
        assert_eq!(parse_monthly_allowance(Some("abc".to_string())), 0);
        assert_eq!(parse_monthly_allowance(Some("-5".to_string())), 0);
        assert_eq!(parse_monthly_allowance(Some("1.5".to_string())), 0);
    }

    #[test]
    fn test_parse_monthly_allowance_clamped_to_max() {
        assert_eq!(
            parse_monthly_allowance(Some("999999999999".to_string())),
            1_000_000
        );
    }
}
