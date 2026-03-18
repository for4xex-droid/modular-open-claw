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

    fn evaluate_trigger(
        trigger: &crate::defense::DefenseTrigger,
        exp: &Experience,
        exp_embedding: &[f32],
    ) -> bool {
        use crate::defense::DefenseTrigger;
        match trigger {
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
            DefenseTrigger::Compound(triggers) => {
                let mut all_matched = true;
                for t in triggers {
                    if !Self::evaluate_trigger(t, exp, exp_embedding) {
                        all_matched = false;
                        break;
                    }
                }
                all_matched && !triggers.is_empty()
            }
        }
    }

    /// L1: Reactive Layer（Learning 0/I）
    pub fn is_rejected_by_reactive_layer(
        &self,
        soul: &AgentSoul,
        exp: &Experience,
        exp_embedding: &[f32],
    ) -> Option<crate::defense::DefenseAction> {
        for defense in &soul.defenses {
            if Self::evaluate_trigger(&defense.trigger, exp, exp_embedding) {
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
            // Apply temporal decay (Step 6)
            soul.apply_temporal_decay();

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

            // RS-2: Somatic Resonance Bias Calculation
            let somatic_bias: f64 = if soul.somatic_markers.is_empty() || exp_embedding.is_empty() {
                0.0
            } else {
                let sum: f64 = soul
                    .somatic_markers
                    .iter()
                    .map(|m| m.resonance(&exp_embedding))
                    .filter(|r| r.abs() > 0.1)
                    .sum();
                sum / (soul.somatic_markers.len() as f64)
            };

            experience.original_prediction = prediction + (somatic_bias * 0.3);

            // L2: Deliberative Layer
            soul.predictive_model.update_plasticity(
                &experience.domain,
                experience.outcome_valence,
                experience.original_prediction,
            );

            // Step 1: dynamic attachment update
            soul.attachment
                .update_from_experience(experience.outcome_valence);

            soul.push_experience(experience.clone());

            // L1.5: Somatic Marking (感情の刻印) - Step 3 & 5
            if experience.outcome_valence.abs() > 0.3 {
                let marker = crate::somatic::SomaticMarker::new_clamped(
                    uuid::Uuid::new_v4().to_string(),
                    exp_embedding.clone(), // Use the pre-fetched embedding (RTT=0)
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

                // Step 4: Somatic Reflex -> Semantic Defense (RS-4)
                if experience.outcome_valence < -0.7 {
                    let trigger = if exp_embedding.is_empty() {
                        crate::defense::DefenseTrigger::Tag(
                            experience.content.chars().take(50).collect(),
                        )
                    } else {
                        crate::defense::DefenseTrigger::Semantic {
                            embedding: exp_embedding,
                            threshold: 0.92,
                        }
                    };

                    let auto_defense = crate::defense::Defense {
                        id: format!("reflex-{}", uuid::Uuid::new_v4()),
                        trigger,
                        action: crate::defense::DefenseAction::Hesitate(2.0),
                        origin_experience_id: experience.id.clone(),
                        intensity: 1.0,
                        created_at: chrono::Utc::now().to_rfc3339(),
                    };
                    tracing::warn!("🛡️ [SoulPipeline] Somatic Reflex triggered! Auto-generating semantic defense: {}", auto_defense.id);
                    soul.defenses.push(auto_defense);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defense::{Defense, DefenseAction, DefenseTrigger};
    use crate::model::AgentSoul;

    #[test]
    fn test_compound_trigger() {
        let soul = AgentSoul::new("test".to_string());
        let exp = Experience {
            id: "e1".to_string(),
            domain: "test".to_string(),
            content: "hello world".to_string(),
            outcome_valence: 0.0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            original_prediction: 0.0,
        };

        let triggers = vec![
            DefenseTrigger::Tag("hello".to_string()),
            DefenseTrigger::Tag("world".to_string()),
        ];
        let compound = DefenseTrigger::Compound(triggers);

        assert!(SoulPipeline::<DummyAdapter, DummyEngine>::evaluate_trigger(
            &compound,
            &exp,
            &[]
        ));

        let triggers_fail = vec![
            DefenseTrigger::Tag("hello".to_string()),
            DefenseTrigger::Tag("missing".to_string()),
        ];
        let compound_fail = DefenseTrigger::Compound(triggers_fail);
        assert!(
            !SoulPipeline::<DummyAdapter, DummyEngine>::evaluate_trigger(&compound_fail, &exp, &[])
        );
    }

    struct DummyEngine;
    impl crate::engine::SamsaraEngine for DummyEngine {
        fn is_shock(&self, _soul: &AgentSoul) -> bool {
            false
        }
        fn distill<'a>(
            &'a self,
            _soul: &'a AgentSoul,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<crate::instinct::Instinct, crate::error::SoulError>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async {
                Ok(crate::instinct::Instinct {
                    rules: vec![],
                    prompt_fragment: String::new(),
                    hash: String::new(),
                })
            })
        }
        fn rebirth<'a>(
            &'a self,
            soul: AgentSoul,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<AgentSoul, crate::error::SoulError>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(soul) })
        }
    }

    struct DummyAdapter;
    impl crate::adapter::SoulDomainAdapter for DummyAdapter {
        fn to_experience(&self, _raw: &dyn std::any::Any) -> Experience {
            Experience::default()
        }
        fn distillation_system_prompt(&self) -> &str {
            ""
        }
        fn predict_outcome(&self, _soul: &AgentSoul, _context: &Experience) -> f64 {
            0.0
        }
        fn execute_defense<'a>(
            &'a self,
            _action: &'a DefenseAction,
            _context: &'a str,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(), crate::error::SoulError>> + Send + 'a>,
        > {
            Box::pin(async { Ok(()) })
        }
        fn embed_experience<'a>(
            &'a self,
            _exp: &'a Experience,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<f32>> + Send + 'a>> {
            Box::pin(async { vec![] })
        }
    }

    #[tokio::test]
    async fn test_somatic_defense_auto_gen() {
        let mut soul = AgentSoul::new("test-auto-gen".to_string());

        let pipeline = SoulPipeline::new(DummyAdapter, DummyEngine);

        let exp = Experience {
            id: "e-shock".to_string(),
            domain: "test".to_string(),
            content: "extremely negative event".to_string(),
            outcome_valence: -0.8, // Triggers reflex
            timestamp: chrono::Utc::now().to_rfc3339(),
            original_prediction: 0.0,
        };

        let _ = pipeline.process_experience(&mut soul, exp.clone()).await;

        let updated_soul = soul;
        // Should have 1 defense auto-generated (Tag fallback because embedding is empty)
        assert_eq!(updated_soul.defenses.len(), 1);

        match &updated_soul.defenses[0].trigger {
            DefenseTrigger::Tag(t) => assert_eq!(t, "extremely negative event"),
            _ => panic!("Expected Tag fallback trigger"),
        }

        match updated_soul.defenses[0].action {
            DefenseAction::Hesitate(_) => (),
            _ => panic!("Expected Hesitate action"),
        }
    }
}
