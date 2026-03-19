/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::llm_provider::LlmProvider;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

/// `HeartbeatWakeupService` 構造体
pub struct HeartbeatWakeupService {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    semaphore: Arc<Semaphore>,
    workspace_dir: std::path::PathBuf,
}

impl HeartbeatWakeupService {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        semaphore: Arc<Semaphore>,
        workspace_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            provider,
            semaphore,
            workspace_dir,
        }
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
