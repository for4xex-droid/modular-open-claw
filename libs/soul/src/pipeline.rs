use std::future::Future;
use std::pin::Pin;

use crate::adapter::SoulDomainAdapter;
use crate::engine::SamsaraEngine;
use crate::error::SoulError;
use crate::model::{AgentSoul, Experience};

/// 3層パイプライン統合
pub struct SoulPipeline<A: SoulDomainAdapter, E: SamsaraEngine + Send + Sync> {
    pub adapter: A,
    pub engine: E,
}

impl<A: SoulDomainAdapter, E: SamsaraEngine + Send + Sync> SoulPipeline<A, E> {
    pub fn new(adapter: A, engine: E) -> Self {
        Self { adapter, engine }
    }

    /// L1: Reactive Layer（Learning 0/I）
    pub fn is_rejected_by_reactive_layer(
        &self,
        soul: &AgentSoul,
        exp: &Experience,
        exp_embedding: &[f32],
    ) -> Option<crate::defense::DefenseAction> {
        // L1: defense mechanisms. Trigger matching.
        use crate::defense::DefenseTrigger;

        for defense in &soul.defenses {
            let matched = match &defense.trigger {
                DefenseTrigger::Tag(tag) => exp.content.contains(tag) || exp.domain.contains(tag),
                DefenseTrigger::Semantic {
                    embedding,
                    threshold,
                } => {
                    if exp_embedding.is_empty() {
                        false
                    } else {
                        crate::somatic::math_utils::cosine_similarity(embedding, exp_embedding)
                            > *threshold
                    }
                }
                DefenseTrigger::Compound(_) => {
                    tracing::debug!(
                        "🛡️ [SoulPipeline] Compound trigger not yet implemented in reactive layer"
                    );
                    false
                }
            };

            if matched {
                tracing::info!(
                    "🛡️ [SoulPipeline] Experience rejected by Reactive Layer (Defense: {})",
                    defense.id
                );
                return Some(defense.action.clone());
            }
        }
        None
    }

    /// パイプライン全体を実行
    pub fn process_experience<'a>(
        &'a self,
        soul: &'a mut AgentSoul,
        exp: Experience,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AgentSoul>, SoulError>> + Send + 'a>> {
        Box::pin(async move {
            // Pre-fetch embedding ONCE to avoid duplicate API calls (Step 5 Optimization)
            let exp_embedding = self.adapter.embed_experience(&exp).await;

            // L1: Reactive Layer Check
            if let Some(action) = self.is_rejected_by_reactive_layer(soul, &exp, &exp_embedding) {
                // Execute defense action (DS-1 fixed)
                if let Err(e) = self.adapter.execute_defense(&action, &exp.content).await {
                    tracing::warn!("⚠️ [SoulPipeline] Failed to execute defense action: {}", e);
                }
                return Ok(None);
            }

            // Calculate prediction dynamically (Step 3)
            let prediction = self.adapter.predict_outcome(soul, &exp);
            let mut experience = exp;
            experience.original_prediction = prediction;

            // L2: Deliberative Layer
            soul.predictive_model.update_plasticity(
                &experience.domain,
                experience.outcome_valence,
                experience.original_prediction,
            );
            soul.attachment.interaction_count = soul.attachment.interaction_count.saturating_add(1);

            soul.push_experience(experience.clone());

            // L1.5: Somatic Marking (感情の刻印) - Step 3 & 5
            if experience.outcome_valence.abs() > 0.3 {
                let marker = crate::somatic::SomaticMarker::new_clamped(
                    uuid::Uuid::new_v4().to_string(),
                    exp_embedding, // Use the pre-fetched embedding (RTT=0)
                    experience.outcome_valence,
                    experience.outcome_valence.abs(), // arousal = intensity of emotion
                    1.0,                              // Fresh intensity
                    experience.timestamp.clone(),
                );
                soul.somatic_markers.push(marker);

                // Max 100 markers, rotate oldest 50
                if soul.somatic_markers.len() > 100 {
                    soul.somatic_markers.drain(0..50);
                }
            }

            soul.compute_hash();

            // L3: Meta-cognitive Layer
            if self.engine.is_shock(soul) {
                // Samsara Triggered
                let new_soul = self.engine.rebirth(soul.clone()).await?;
                return Ok(Some(new_soul));
            }

            Ok(None)
        })
    }
}
