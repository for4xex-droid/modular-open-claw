/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! A2A gRPC Protocol Bindings
//!
//! Exposes protobuf definitions for Shadow Clone internal communication.

#[cfg(feature = "grpc")]
pub mod internal {
    tonic::include_proto!("aiome.a2a.internal.v1");
}

use crate::error::AiomeError;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

pub mod agent_card;

/// Task execution request towards a shadow clone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskRequest {
    pub job_id: String,
    pub prompt_b64: String,
    pub artifact_path: Option<String>,
    pub agent_yaml_b64: String,
    pub auth_token: String,
    pub proof_of_intent: Option<String>,
    pub sender_did: Option<String>,
}

/// Task progress report arriving via gRPC streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskProgress {
    pub message: String,
    pub percent: u32,
    pub is_completed: bool,
    pub is_failed: bool,
    pub result: Option<String>,
    pub error: Option<String>,
    pub result_hash: Option<String>,
}

/// Agent-to-Agent communication client interface
#[async_trait]
pub trait A2aClient: Send + Sync {
    /// Executes a task autonomously within the shadow clone and returns a live stream of progress.
    async fn execute_task(
        &self,
        request: A2aTaskRequest,
    ) -> Result<BoxStream<'static, Result<A2aTaskProgress, AiomeError>>, AiomeError>;

    /// Request task cancellation
    async fn cancel_task(&self, job_id: &str) -> Result<(), AiomeError>;
}
