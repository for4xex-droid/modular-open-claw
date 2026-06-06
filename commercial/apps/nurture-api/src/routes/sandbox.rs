/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::mcp_tools::handle_sandbox_exec;
use crate::state::SharedState;
use axum::{response::IntoResponse, routing::post, Extension, Json, Router};
use commerce_protocol::mcp_commerce::SandboxExecRequest;

pub fn sandbox_routes() -> Router<()> {
    Router::new().route("/exec", post(exec_sandbox))
}

use crate::auth::McpAuth;

async fn exec_sandbox(
    _: McpAuth,
    Extension(state): Extension<SharedState>,
    Json(req): Json<SandboxExecRequest>,
) -> Result<impl IntoResponse, commerce_protocol::error::NurtureError> {
    let res = handle_sandbox_exec(state.python_executor.clone(), req).await?;
    Ok((axum::http::StatusCode::OK, Json(res)))
}
