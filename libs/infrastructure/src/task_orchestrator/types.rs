/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core_contracts::contracts::{FeedbackCategory, ReviewDecision};
use aiome_core_contracts::traits::Job;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Task orchestration event. Provides observability (like cmux read-screen).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TaskEvent {
    Spawned {
        job_id: String,
        conductor_id: String,
    },
    Progress {
        job_id: String,
        conductor_id: String,
        message: String,
        percent: Option<u8>,
    },
    Completed {
        job_id: String,
        result: String,
    },
    Evaluating {
        job_id: String,
    },
    Failed {
        job_id: String,
        error: String,
    },
    Cancelled {
        job_id: String,
    },
    AwaitingInput {
        job_id: String,
        reason: String,
    },
    GigPublished {
        job_id: String,
        intent_id: String,
        description: String,
        budget: u64,
    },
    QualityGate {
        job_id: String,
        score: u32,
        passed: bool,
        conductor: String,
        review_decision: Option<ReviewDecision>,
        feedback: Option<FeedbackCategory>,
    },
}

/// A conductor that executes a specific type of task autonomously.
#[async_trait]
pub trait TaskConductor: Send + Sync {
    /// Human-readable name of the conductor
    fn conductor_name(&self) -> &str;

    /// Categories of tasks this conductor can handle
    fn capable_categories(&self) -> Vec<String>;

    /// Execute the task
    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError>;

    /// Cancel the execution of a specific job
    async fn cancel(&self, _job_id: &str) -> Result<(), AiomeError> {
        Ok(()) // Default implementation: do nothing
    }
}

/// SOUL.md と EVOLVING_SOUL.md を読み込み、soul_hash を計算する。
/// AppState::get_system_soul_hash() と同じハッシュロジックを使用する。
#[tracing::instrument(skip_all, fields(has_path = soul_path.is_some()))]
pub(crate) async fn compute_soul_hash(soul_path: &Option<std::path::PathBuf>) -> String {
    let path = match soul_path {
        Some(p) => p,
        None => return "unknown".to_string(),
    };
    shared::soul_hash::compute_from_path(path).await
}
