/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::score_tracker::ScoreTracker;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::traits::AgentEvolver;
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
    lora_service: Option<Arc<dyn aiome_core_contracts::traits::LoraEngine>>,
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
        lora_service: Option<Arc<dyn aiome_core_contracts::traits::LoraEngine>>,
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
                                    let params = serde_json::json!({
                                        "agent_id": null
                                    });
                                    match lora
                                        .train("autonomous-recovery", "auto_exp", params)
                                        .await
                                    {
                                        Ok(job_id) => {
                                            tracing::info!("✅ [Heartbeat] Autonomous LoRA Training scheduled with Job ID: {}", job_id);
                                        }
                                        Err(e) => {
                                            tracing::error!("❌ [Heartbeat] Autonomous LoRA Training failed to start: {:?}", e);
                                        }
                                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use aiome_core::error::AiomeError;
    use aiome_core_contracts::error::AiomeError as ContractError;
    use aiome_core_contracts::forecast::{
        AnomalyResult, ForecastConfig, ForecastProvider, ForecastResult,
    };
    use aiome_core_contracts::llm::{LlmProvider, LlmResponse, StopReason};
    use aiome_core_contracts::traits::{AgentEvolver, LoraEngine};
    use aiome_core_contracts::AgentStats;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default, Debug)]
    struct MockLoraEngine {
        pub train_called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl LoraEngine for MockLoraEngine {
        async fn complete_with_lora(
            &self,
            _prompt: &str,
            _lora_id: &str,
        ) -> Result<LlmResponse, ContractError> {
            Err(ContractError::Infrastructure {
                reason: "Mock complete_with_lora not implemented".to_string(),
            })
        }

        async fn train(
            &self,
            _base_model: &str,
            _dataset_id: &str,
            _params: serde_json::Value,
        ) -> Result<String, ContractError> {
            self.train_called.store(true, Ordering::SeqCst);
            Ok("job_test".to_string())
        }

        async fn health_check(&self) -> Result<bool, ContractError> {
            Ok(true)
        }
    }

    #[derive(Default, Debug)]
    struct MockLlmProvider;

    #[async_trait]
    impl LlmProvider for MockLlmProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _system_prompt: Option<&str>,
        ) -> Result<LlmResponse, ContractError> {
            Ok(LlmResponse {
                content: "HEARTBEAT_OK".to_string(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn test_connection(&self) -> Result<(), ContractError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "Mock"
        }
    }

    #[derive(Default, Debug)]
    struct MockForecastProvider;

    #[async_trait]
    impl ForecastProvider for MockForecastProvider {
        async fn forecast(
            &self,
            series: Vec<Vec<f64>>,
            horizon: usize,
            _config: ForecastConfig,
        ) -> Result<ForecastResult, AiomeError> {
            // Predict a very small growth (stagnation)
            let point_forecast: Vec<Vec<f64>> = series
                .iter()
                .map(|s| {
                    let last_val = s.last().copied().unwrap_or(0.0);
                    (0..horizon).map(|i| last_val + (i as f64 * 0.01)).collect()
                })
                .collect();
            Ok(ForecastResult {
                point_forecast,
                quantile_forecast: None,
                model_version: "mock".to_string(),
            })
        }
        async fn detect_anomaly(
            &self,
            _historical: Vec<f64>,
            _recent: Vec<f64>,
            _threshold_sigma: f64,
        ) -> Result<AnomalyResult, AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "Mock detect_anomaly not implemented".to_string(),
            })
        }
        fn name(&self) -> &str {
            "MockForecast"
        }
    }

    #[derive(Default, Debug)]
    struct MockAgentEvolver;

    #[async_trait]
    impl AgentEvolver for MockAgentEvolver {
        async fn get_agent_stats(&self) -> Result<AgentStats, ContractError> {
            Ok(AgentStats {
                level: 1,
                resonance: 100,
                creativity: 100,
                exp: 1000,
                fatigue: 0,
            })
        }
        async fn add_resonance(&self, _amount: i32) -> Result<(), ContractError> {
            Ok(())
        }
        async fn add_tech_exp(&self, _amount: i32) -> Result<(), ContractError> {
            Ok(())
        }
        async fn add_creativity(&self, _amount: i32) -> Result<(), ContractError> {
            Ok(())
        }
        async fn sync_samsara_level(
            &self,
        ) -> Result<Option<aiome_core_contracts::contracts::SamsaraEvent>, ContractError> {
            Ok(None)
        }
        async fn record_evolution_event(
            &self,
            _level: i32,
            _event_type: &str,
            _description: &str,
            _inspiration: Option<&str>,
            _karma_json: Option<&str>,
        ) -> Result<(), ContractError> {
            Ok(())
        }
        async fn fetch_evolution_history(
            &self,
            _limit: i64,
        ) -> Result<Vec<serde_json::Value>, ContractError> {
            Ok(vec![])
        }
        async fn record_soul_mutation(
            &self,
            _old_hash: &str,
            _new_hash: &str,
            _reason: &str,
        ) -> Result<(), ContractError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_heartbeat_triggers_train_on_plateau_e2e() -> Result<(), Box<dyn std::error::Error>>
    {
        // 1. Setup in-memory DB
        let sqlite_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await?;
        sqlx::query("CREATE TABLE score_snapshots (snapshot_date TEXT NOT NULL, metric_name TEXT NOT NULL, metric_value REAL NOT NULL, PRIMARY KEY (snapshot_date, metric_name))")
            .execute(&sqlite_pool).await?;

        // 2. Insert 15 days of historical data (simulating a recent plateau)
        for i in 0..15 {
            let date = (chrono::Utc::now() - chrono::Duration::days(15 - i))
                .format("%Y-%m-%d")
                .to_string();
            // Up to day 10, steady growth. Day 10-15, plateau.
            let val = if i <= 10 {
                (i * 10) as f64
            } else {
                100.0 + ((i - 10) as f64 * 0.1)
            };
            sqlx::query("INSERT INTO score_snapshots (snapshot_date, metric_name, metric_value) VALUES (?, ?, ?)")
                .bind(date)
                .bind("exp")
                .bind(val)
                .execute(&sqlite_pool).await?;
        }

        let pool = DatabasePool::Sqlite(sqlite_pool);

        // 3. Setup Mocks
        let mock_forecast = Arc::new(MockForecastProvider::default());
        let tracker = Arc::new(ScoreTracker::new(Some(mock_forecast), pool));
        let mock_lora = Arc::new(MockLoraEngine::default());
        let train_called = mock_lora.train_called.clone();
        let evolver = Arc::new(MockAgentEvolver::default());

        // 4. Setup Service
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path().to_path_buf();

        // Create HEARTBEAT.md to ensure ping triggers
        std::fs::write(
            workspace.join("HEARTBEAT.md"),
            "# Status\nAll systems operational.\nSome actual text here.",
        )?;

        let service = HeartbeatWakeupService::new(
            Arc::new(MockLlmProvider::default()),
            Arc::new(Semaphore::new(1)),
            workspace.clone(),
        )
        .with_evolution_tools(
            tracker.clone(),
            evolver.clone(),
            Some(mock_lora as Arc<dyn LoraEngine>),
        );

        // 5. Execution
        let _ = service.run_wakeup_ping().await;

        // 6. Verification (Positive Test)
        // It should have detected plateau and called train()
        assert!(
            train_called.load(Ordering::SeqCst),
            "LoraEngine::train was NOT called during plateau!"
        );

        // 7. Negative Test: Cooldown constraint
        // Reset call tracker
        train_called.store(false, Ordering::SeqCst);
        let _ = service.run_wakeup_ping().await;
        assert!(
            !train_called.load(Ordering::SeqCst),
            "LoraEngine::train was called repeatedly within cooldown period!"
        );

        Ok(())
    }
}
