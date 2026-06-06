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

pub mod asset;
pub mod clone;
pub mod internal;
pub mod marketplace;
pub mod polar;
pub mod sandbox;
pub mod stripe;
pub mod wallet;

use crate::state::SharedState;
use axum::{Extension, Router};

pub fn nurture_routes(state: SharedState) -> Router<()> {
    Router::new()
        .nest("/mcp", crate::mcp::mcp_routes())
        .nest("/marketplace", marketplace::marketplace_routes())
        .nest("/wallet", wallet::wallet_routes())
        .nest("/clone", clone::clone_routes())
        .nest("/stripe", stripe::stripe_routes())
        .nest("/polar", polar::polar_routes())
        .nest("/sandbox", sandbox::sandbox_routes())
        .layer(Extension(state))
}
