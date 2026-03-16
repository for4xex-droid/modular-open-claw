/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

pub mod client;
pub mod discovery;
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
