/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use crate::state::SharedState;
use commerce_protocol::error::NurtureError;
use commerce_protocol::mcp_commerce::{MarketSearchRequest, MarketSearchResponse};

pub async fn handle_marketplace_search(
    state: SharedState,
    req: MarketSearchRequest,
) -> Result<MarketSearchResponse, NurtureError> {
    let query = req.query.unwrap_or_default();
    let items = state.marketplace.search_items(query, req.limit).await?;
    let total_count = items.len() as u64;

    Ok(MarketSearchResponse { items, total_count })
}
