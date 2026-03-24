/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::LlmProvider;
use aiome_contracts::traits::{Job, JobQueue, JobStatus, Publisher};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::info;

/// `mock_x` モジュール
#[cfg(any(test, debug_assertions))]
pub mod mock_x;

/// [B-2] Publish Pipeline Orchestrator
/// 各種パブリッシャーを管理し、ジョブステータスに基づいて配信を実行する。
pub struct PublishPipeline {
    publishers: Vec<Box<dyn Publisher>>,
}

impl PublishPipeline {
    /// 新しいインスタンスを生成する
    pub fn new(publishers: Vec<Box<dyn Publisher>>) -> Self {
        Self { publishers }
    }

    /// `run_job` を実行する
    pub async fn run_job(
        &self,
        platform: &str,
        content: &str,
        media_paths: &[PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        let publisher = self
            .publishers
            .iter()
            .find(|p| p.platform_name() == platform)
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: format!("Publisher not found for platform: {}", platform),
            })?;

        info!("📤 [PublishPipeline] Publishing to {}...", platform);
        publisher.publish(content, media_paths, metadata).await
    }
}
