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
    lora_service: Option<Arc<crate::lora_training::LoraTrainingService>>,
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
            lora_service: None,
        }
    }

    /// ScoreTracker を紐付ける
    /// ScoreTracker と LoraService を紐付ける
    pub fn with_evolution_tools(
        mut self,
        tracker: Arc<ScoreTracker>,
        registry: Arc<dyn AgentEvolver>,
        lora_service: Option<Arc<crate::lora_training::LoraTrainingService>>,
    ) -> Self {
        self.score_tracker = Some(tracker);
        self.agent_evolver = Some(registry);
        self.lora_service = lora_service;
        self
    }

    /// `run_wakeup_ping` を実行する
    pub async fn run_wakeup_ping(&self) -> Option<String> {
        let filename = "HEARTBEAT.md";
        let target_path = self.workspace_dir.join(filename);
        let content = fs::read(target_path)
            .map(|bytes| String::from_utf8_lossy(&bytes[..bytes.len().min(5000)]).to_string())
            .unwrap_or_default();

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
                                "📉 [Heartbeat] Score plateau detected for '{}'. Evaluating autonomous LoRA...",
                                report.metric_name
                            );

                            // Check Cooldown File
                            let cooldown_file =
                                self.workspace_dir.join("last_lora_trigger.timestamp");
                            let can_trigger = if cooldown_file.exists() {
                                if let Ok(meta) = fs::metadata(&cooldown_file) {
                                    if let Ok(modified) = meta.modified() {
                                        if let Ok(elapsed) = modified.elapsed() {
                                            // Cooldown: 24 hours
                                            elapsed.as_secs() > 86400
                                        } else {
                                            false
                                        } // Clock went backwards
                                    } else {
                                        false
                                    } // Fail-safe
                                } else {
                                    false
                                } // Fail-safe
                            } else {
                                true
                            };

                            if can_trigger {
                                if let Some(ref lora) = self.lora_service {
                                    info!("🚀 [Heartbeat] Triggering Autonomous LoRA Training due to plateau!");
                                    let mut config =
                                        crate::lora_training::LoraTrainingConfig::default();
                                    config.base_model = "autonomous-recovery".into();
                                    config.dataset_path = "workspace/datasets/auto_exp".into();
                                    config.output_dir = "workspace/output".into();
                                    config.vault_path = "workspace/vault/auto_recovery".into();

                                    let lora_clone = lora.clone();
                                    tokio::spawn(async move {
                                        let dummy_id = format!(
                                            "auto_recovery_{}",
                                            chrono::Utc::now().timestamp()
                                        );
                                        let cancel_token =
                                            tokio_util::sync::CancellationToken::new();
                                        if let Err(e) = lora_clone
                                            .start_training(&dummy_id, config, cancel_token)
                                            .await
                                        {
                                            tracing::error!("❌ [Heartbeat] Autonomous LoRA Training failed: {:?}", e);
                                        } else {
                                            tracing::info!("✅ [Heartbeat] Autonomous LoRA Training completed successfully.");
                                        }
                                    });
                                    // Touch cooldown file
                                    if let Err(e) =
                                        fs::write(&cooldown_file, chrono::Utc::now().to_rfc3339())
                                    {
                                        tracing::error!("🚨 [Heartbeat] CRITICAL: Failed to write cooldown file at {:?}. Error: {:?}", cooldown_file, e);
                                    }
                                } else {
                                    warn!("⚠️ [Heartbeat] LoraTrainingService not configured. Skipping autonomous training.");
                                }
                            } else {
                                info!("⏳ [Heartbeat] LoRA training is on cooldown. Skipping.");
                            }
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
                        // RT-5 Sanitization: Drop any dangerous patterns (shell, markdown injection)
                        let lower = reply.to_lowercase();
                        if lower.contains("curl ")
                            || lower.contains("wget ")
                            || lower.contains("bash")
                            || lower.contains("sudo ")
                            || lower.contains("eval")
                            || lower.contains("rm -rf")
                        {
                            warn!("🚨 [Heartbeat] Blocked generated text containing potential shell commands or dangerous keywords.");
                            None
                        } else {
                            // Strip markdown code blocks just in case
                            Some(reply.replace('`', ""))
                        }
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
