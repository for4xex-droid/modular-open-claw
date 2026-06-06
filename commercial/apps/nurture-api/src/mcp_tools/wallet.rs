/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

//! MCP Tool: Wallet Management.
//! Allows agents to check balance and manage funds.

use crate::state::SharedState;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_core::coin::CoinWallet;
use nurture_core::ledger::LedgerEntry;
use nurture_core::points::PointsAccount;

pub async fn handle_get_balance(
    state: SharedState,
    actor_id: ActorId,
) -> Result<CoinWallet, NurtureError> {
    state.ledger.get_balance(&actor_id).await
}

pub async fn handle_get_points(
    state: SharedState,
    actor_id: ActorId,
) -> Result<PointsAccount, NurtureError> {
    state.ledger.get_points(&actor_id).await
}

pub async fn handle_get_history(
    state: SharedState,
    actor_id: ActorId,
    limit: u32,
) -> Result<Vec<LedgerEntry>, NurtureError> {
    state.ledger.get_history(&actor_id, limit).await
}
