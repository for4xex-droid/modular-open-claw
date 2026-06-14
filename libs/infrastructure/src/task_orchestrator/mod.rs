/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub mod csam;
pub mod geo_audit;
pub mod llm_conductor;
pub mod planner;
pub mod seo_content;
pub mod workflow_conductor;

pub mod dispatch_loop;
pub mod dispatcher;
pub mod goal_processor;
pub mod types;

// Re-exports for external use
pub use dispatcher::TaskDispatcher;
pub use types::{TaskConductor, TaskEvent};

// Re-exports for internal/test use (via `use super::*`)
pub(crate) use dispatcher::MAX_GIG_BUDGET;
pub(crate) use types::compute_soul_hash;

// Re-export common dependencies that tests rely on via `use super::*`
pub(crate) use aiome_core::error::AiomeError;
pub(crate) use aiome_core_contracts::traits::Job;
pub(crate) use async_trait::async_trait;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use tokio::sync::mpsc;

#[cfg(test)]
mod tests;
