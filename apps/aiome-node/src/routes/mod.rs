/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use axum::routing::get;
use axum::Router;

pub mod agent_card;
pub mod federation;

pub fn well_known_routes() -> Router {
    Router::new().route("/agent.json", get(agent_card::get_agent_card))
}
