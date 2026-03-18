use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use soul::adapter::SoulDomainAdapter;
use soul::defense::DefenseAction;
use soul::error::SoulError;
use soul::model::{AgentSoul, Experience};

/// 自動補完構造体
pub struct CoreDomainAdapter {
    job_queue: std::sync::Arc<dyn aiome_core::traits::JobQueue>,
    embedding_provider: Option<std::sync::Arc<dyn aiome_core::llm_provider::EmbeddingProvider>>,
}

impl CoreDomainAdapter {
    /// 自動補完関数
    pub fn new(
        job_queue: std::sync::Arc<dyn aiome_core::traits::JobQueue>,
        embedding_provider: Option<std::sync::Arc<dyn aiome_core::llm_provider::EmbeddingProvider>>,
    ) -> Self {
        Self {
            job_queue,
            embedding_provider,
        }
    }
}

impl SoulDomainAdapter for CoreDomainAdapter {
    fn to_experience(&self, _event: &dyn Any) -> Experience {
        Experience::default()
    }

    fn distillation_system_prompt(&self) -> &str {
        "You are the Soul Engine distillator. Summarize the agent's experiences."
    }

    fn predict_outcome(&self, soul: &AgentSoul, exp: &Experience) -> f64 {
        if exp.outcome_valence == 0.0 {
            // Neutral events trigger no prediction bias, avoiding artificial surprise
            return 0.0;
        }

        if let Some(dm) = soul.predictive_model.domains.get(&exp.domain) {
            // Predict based on past accuracy, assuming the outcome retains the same direction (valence sign)
            let sign = exp.outcome_valence.signum();
            dm.prediction_accuracy * sign
        } else {
            // Unknown domain: neutrality with built-in sensitivity matching the direction
            soul.predictive_model.global_surprise_sensitivity * 0.5 * exp.outcome_valence.signum()
        }
    }

    fn embed_experience<'a>(
        &'a self,
        exp: &'a Experience,
    ) -> Pin<Box<dyn Future<Output = Vec<f32>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(ep) = &self.embedding_provider {
                match ep.embed(&exp.content, false).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("⚠️ [CoreDomainAdapter] Embedding failed: {:?}", e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        })
    }

    fn execute_defense<'a>(
        &'a self,
        action: &'a DefenseAction,
        context: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'a>> {
        Box::pin(async move {
            tracing::info!(
                "🛡️ [CoreDomainAdapter] Executing defense action: {:?}",
                action
            );

            match action {
                DefenseAction::Reject => {
                    // Create a dynamic ImmuneRule to block similar patterns in the future
                    let rule = aiome_core::contracts::ImmuneRule {
                        id: format!("auto-reject-{}", uuid::Uuid::new_v4()),
                        pattern: context.chars().take(200).collect::<String>(),
                        severity: 100,
                        action: "Block".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        approval_status: aiome_core::contracts::ApprovalState::Approved,
                        lamport_clock: 0,
                        node_id: "local-soul-engine".to_string(),
                        signature: None,
                    };
                    let _ = self.job_queue.store_immune_rule(&rule).await;
                }
                DefenseAction::Warn => {
                    // Record in Evolution Chronicle as a security alert
                    let _ = self
                        .job_queue
                        .record_evolution_event(
                            0, // System level
                            "SecurityAlert",
                            &format!(
                                "Reactive layer WARNING for suspicious activity: {}",
                                context.chars().take(200).collect::<String>()
                            ),
                            None,
                            None,
                        )
                        .await;
                }
                DefenseAction::Hesitate(secs) => {
                    // Inject latency to thwart rapid automated attacks
                    tokio::time::sleep(tokio::time::Duration::from_secs_f64(*secs)).await;
                }
                DefenseAction::RequireEscrow => {
                    let _ = self
                        .job_queue
                        .record_evolution_event(
                            0,
                            "SecurityAlert",
                            &format!(
                                "Reactive layer REQUIRE_ESCROW triggered for: {}",
                                context.chars().take(200).collect::<String>()
                            ),
                            None,
                            None,
                        )
                        .await;
                }
                DefenseAction::Deflect => {
                    let _ = self
                        .job_queue
                        .record_evolution_event(
                            0,
                            "SecurityAlert",
                            &format!(
                                "Reactive layer DEFLECT triggered for: {}",
                                context.chars().take(200).collect::<String>()
                            ),
                            None,
                            None,
                        )
                        .await;
                }
                DefenseAction::Custom(reason) => {
                    let _ = self
                        .job_queue
                        .record_evolution_event(
                            0,
                            "SecurityAlert",
                            &format!(
                                "Reactive layer CUSTOM ({}) triggered for: {}",
                                reason,
                                context.chars().take(200).collect::<String>()
                            ),
                            None,
                            None,
                        )
                        .await;
                }
            }
            Ok(())
        })
    }
}
