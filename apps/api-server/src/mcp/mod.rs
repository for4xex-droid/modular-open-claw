/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub mod client;
pub mod discovery;
pub mod http_client;
pub mod server;
pub mod types;

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sse", get(server::sse_handler))
        .route("/messages", post(server::message_handler))
}
