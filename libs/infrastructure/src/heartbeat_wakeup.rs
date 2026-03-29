/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::score_tracker::ScoreTracker;
use aiome_contracts::traits::AgentEvolver;
use aiome_core::llm_provider::LlmProvider;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

/// `HeartbeatWakeupService` 構造体
pub struct HeartbeatWakeupService {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    semaphore: Arc<Semaphore>,
    workspace_dir: PathBuf,
    score_tracker: Option<Arc<ScoreTracker>>,
    agent_evolver: Option<Arc<dyn AgentEvolver>>,
}

impl HeartbeatWakeupService {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        semaphore: Arc<Semaphore>,
        workspace_dir: PathBuf,
    ) -> Self {
        Self {
            provider,
            semaphore,
            workspace_dir,
            score_tracker: None,
            agent_evolver: None,
        }
    }

    /// ScoreTracker を紐付ける
    pub fn with_score_tracker(
        mut self,
        tracker: Arc<ScoreTracker>,
        registry: Arc<dyn AgentEvolver>,
    ) -> Self {
        self.score_tracker = Some(tracker);
        self.agent_evolver = Some(registry);
        self
    }

    /// `run_wakeup_ping` を実行する
    pub async fn run_wakeup_ping(&self) -> Option<String> {
        let filename = "HEARTBEAT.md";
        let target_path = self.workspace_dir.join(filename);
        let content = fs::read_to_string(&target_path).unwrap_or_default();

        // G-24: もしコンテンツが空、または実効性のない場合は早期リターン
        if content.trim().is_empty() || self.is_effectively_empty(&content) {
            return None;
        }

        // Phase 3D: Capture daily score snapshot
        // RT-1 FIX: Wrap in timeout to guarantee Heartbeat liveness even if
        // TimesFM sidecar is unresponsive or DB is locked.
        if let (Some(tracker), Some(registry)) = (&self.score_tracker, &self.agent_evolver) {
            let snapshot_future = async {
                if let Err(e) = tracker.record_daily_snapshot(registry).await {
                    warn!("⚠️ [Heartbeat] Failed to record score snapshot: {:?}", e);
                } else {
                    match tracker.detect_plateau("exp", 14).await {
                        Ok(Some(report)) if report.is_stagnating => {
                            warn!(
                                "📉 [Heartbeat] Score plateau detected for '{}'",
                                report.metric_name
                            );
                        }
                        Err(e) => {
                            warn!("⚠️ [Heartbeat] Plateau detection failed: {:?}", e);
                        }
                        _ => {}
                    }
                }
            };
            match tokio::time::timeout(std::time::Duration::from_secs(15), snapshot_future).await {
                Ok(()) => {}
                Err(_) => {
                    warn!("⏰ [Heartbeat] ScoreTracker timed out after 15s. Sidecar may be unresponsive.");
                }
            }
        }

        // Phase 1 Flaw 4 Defense: Use try_acquire to avoid blocking
        if let Ok(_permit) = self.semaphore.try_acquire() {
            info!(
                "💓 [Heartbeat] Triggering Wakeup Ping with context (len: {})...",
                content.len()
            );

            // Phase 1 Flaw 7 Defense: Optimized prompt for strict token usage
            let preamble = "あなたは自律OSのHeartbeat監視役です。
HEARTBEAT.mdを確認し、緊急のタスクやユーザーへの報告事項があるか判断してください。
【超重要】緊急性が低い、または保留中のタスクのみの場合は、必ず 'HEARTBEAT_OK' とだけ答えること。
冗長な挨拶や確認は不要です。";

            let prompt = format!(
                "Current Workspace Heartbeat Context:\n---\n{}\n---\nIs there any immediate action required? Reply either 'HEARTBEAT_OK' or the specific proactive recommendation.",
                content
            );

            match self.provider.complete(&prompt, Some(preamble)).await {
                Ok(resp) => {
                    let reply = resp.content.trim();
                    if reply == "HEARTBEAT_OK" || reply.is_empty() {
                        info!("💓 [Heartbeat] System state: OK");
                        None
                    } else {
                        info!("💓 [Heartbeat] Proactive Talk generated.");
                        Some(reply.to_string())
                    }
                }
                Err(e) => {
                    warn!("⚠️ [Heartbeat] LLM completion failed: {:?}", e);
                    None
                }
            }
        } else {
            info!("💤 [Heartbeat] LLM busy, skipping wakeup ping.");
            None
        }
    }

    fn is_effectively_empty(&self, content: &str) -> bool {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip markdown header lines (# followed by space or EOL, ## etc)
            if let Some(after_hash) = trimmed.strip_prefix('#') {
                if after_hash.is_empty()
                    || after_hash
                        .chars()
                        .next()
                        .map(|c| c.is_whitespace())
                        .unwrap_or(false)
                {
                    continue;
                }
            }
            // Skip empty markdown list items like "- [ ]" or "* [ ]" or just "- "
            if (trimmed.starts_with("- [ ]")
                || trimmed.starts_with("* [ ]")
                || trimmed.starts_with("+ [ ]"))
                && trimmed.len() <= 5
            {
                continue;
            }
            if trimmed == "- " || trimmed == "* " || trimmed == "+ " {
                continue;
            }
            // Found actionable content
            return false;
        }
        true
    }
}
