use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

use crate::adapter::SoulDomainAdapter;
use crate::engine::SamsaraEngine;
use crate::error::SoulError;
use crate::model::{AgentSoul, Experience};

#[async_trait]
pub trait SoulMiddleware<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static>: Send + Sync {
    async fn process(&self, ctx: &mut SoulContext<'_, A, E>, next: &(dyn SoulMiddlewareNext<A, E> + '_)) -> Result<(), SoulError>;
}

pub trait SoulMiddlewareNext<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static>: Send + Sync {
    fn run<'a, 'b>(&'a self, ctx: &'b mut SoulContext<'_, A, E>) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'b>>
    where 'a: 'b;
}

pub struct SoulContext<'a, A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    pub pipeline: &'a SoulPipeline<A, E>,
    pub soul: &'a mut AgentSoul,
    pub experience: Experience,
    pub embedding: Vec<f32>,
    pub should_continue: bool,
    pub rebirth_required: bool,
    pub is_rejected: bool,
}

struct MiddlewareChain<'a, A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    pipeline: &'a SoulPipeline<A, E>,
    middlewares: &'a [Box<dyn SoulMiddleware<A, E>>],
    index: usize,
}

impl<'a, A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddlewareNext<A, E> for MiddlewareChain<'a, A, E> {
    fn run<'b, 'c>(&'b self, ctx: &'c mut SoulContext<'_, A, E>) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'c>>
    where 'b: 'c {
        Box::pin(async move {
            if self.index < self.middlewares.len() {
                let next = MiddlewareChain {
                    pipeline: self.pipeline,
                    middlewares: self.middlewares,
                    index: self.index + 1,
                };
                self.middlewares[self.index].process(ctx, &next).await
            } else {
                Ok(())
            }
        })
    }
}

/// L1: Reactive Layer Middleware
struct ReactiveMiddleware<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    _phantom: std::marker::PhantomData<(A, E)>,
}
#[async_trait]
impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddleware<A, E> for ReactiveMiddleware<A, E> {
    async fn process(&self, ctx: &mut SoulContext<'_, A, E>, next: &(dyn SoulMiddlewareNext<A, E> + '_)) -> Result<(), SoulError> {
        if let Some(action) = ctx.pipeline.is_rejected_by_reactive_layer(ctx.soul, &ctx.experience, &ctx.embedding) {
            if let Err(e) = ctx.pipeline.adapter.execute_defense(&action, &ctx.experience.content).await {
                tracing::warn!("⚠️ [SoulPipeline] Failed to execute defense action: {}", e);
            }
            ctx.should_continue = false;
            return Ok(());
        }
        next.run(ctx).await
    }
}

/// L2: Deliberative Layer Middleware
struct DeliberativeMiddleware<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    _phantom: std::marker::PhantomData<(A, E)>,
}
#[async_trait]
impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddleware<A, E> for DeliberativeMiddleware<A, E> {
    async fn process(&self, ctx: &mut SoulContext<'_, A, E>, next: &(dyn SoulMiddlewareNext<A, E> + '_)) -> Result<(), SoulError> {
        let prediction = ctx.pipeline.adapter.predict_outcome(ctx.soul, &ctx.experience);
        
        let somatic_bias: f64 = if ctx.soul.somatic_markers.is_empty() || ctx.embedding.is_empty() {
            0.0
        } else {
            let sum: f64 = ctx.soul
                .somatic_markers
                .iter()
                .map(|m| m.resonance(&ctx.embedding))
                .filter(|r| r.abs() > 0.1)
                .sum();
            sum / (ctx.soul.somatic_markers.len() as f64)
        };

        ctx.experience.original_prediction = prediction + (somatic_bias * 0.3);

        ctx.soul.predictive_model.update_plasticity(
            &ctx.experience.domain,
            ctx.experience.outcome_valence,
            ctx.experience.original_prediction,
        );

        ctx.soul.attachment.update_from_experience(ctx.experience.outcome_valence);
        ctx.soul.push_experience(ctx.experience.clone());

        next.run(ctx).await
    }
}

/// L3: Meta-cognitive Layer Middleware
struct MetaCognitiveMiddleware<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    _phantom: std::marker::PhantomData<(A, E)>,
}
#[async_trait]
impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddleware<A, E> for MetaCognitiveMiddleware<A, E> {
    async fn process(&self, ctx: &mut SoulContext<'_, A, E>, next: &(dyn SoulMiddlewareNext<A, E> + '_)) -> Result<(), SoulError> {
        if ctx.pipeline.engine.is_shock(ctx.soul) {
            ctx.rebirth_required = true;
        }
        next.run(ctx).await
    }
}

/// 3層パイプライン統合
pub struct SoulPipeline<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    pub adapter: A,
    pub engine: E,
    middlewares: Vec<Box<dyn SoulMiddleware<A, E>>>,
}

impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulPipeline<A, E> {
    pub fn new(adapter: A, engine: E) -> Self {
        Self { 
            adapter, 
            engine,
            middlewares: vec![
                Box::new(ReactiveMiddleware::<A, E> { _phantom: std::marker::PhantomData }),
                Box::new(DeliberativeMiddleware::<A, E> { _phantom: std::marker::PhantomData }),
                Box::new(MetaCognitiveMiddleware::<A, E> { _phantom: std::marker::PhantomData }),
            ],
        }
    }
    
    // ... (evaluate_trigger, is_rejected_by_reactive_layer continue below)

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
            soul.apply_temporal_decay();

            let exp_embedding = self.adapter.embed_experience(&exp).await;

            let mut ctx = SoulContext {
                pipeline: self,
                soul,
                experience: exp,
                embedding: exp_embedding,
                should_continue: true,
                rebirth_required: false,
                is_rejected: false,
            };

            let chain = MiddlewareChain {
                pipeline: self,
                middlewares: &self.middlewares,
                index: 0,
            };

            chain.run(&mut ctx).await?;

            if !ctx.should_continue {
                return Ok(None);
            }

            // Somatic Marking (感情の刻印) - Logic preserved from original
            if ctx.experience.outcome_valence.abs() > 0.3 {
                let marker = crate::somatic::SomaticMarker::new_clamped(
                    uuid::Uuid::new_v4().to_string(),
                    ctx.embedding.clone(),
                    ctx.experience.outcome_valence,
                    ctx.experience.outcome_valence.abs(),
                    1.0,
                    ctx.experience.timestamp.clone(),
                );
                ctx.soul.somatic_markers.push(marker);

                if ctx.soul.somatic_markers.len() > 100 {
                    ctx.soul.somatic_markers.drain(0..50);
                }

                if ctx.experience.outcome_valence < -0.7 {
                    let trigger = if ctx.embedding.is_empty() {
                        crate::defense::DefenseTrigger::Tag(
                            ctx.experience.content.chars().take(50).collect(),
                        )
                    } else {
                        crate::defense::DefenseTrigger::Semantic {
                            embedding: ctx.embedding.clone(),
                            threshold: 0.92,
                        }
                    };

                    let auto_defense = crate::defense::Defense {
                        id: format!("reflex-{}", uuid::Uuid::new_v4()),
                        trigger,
                        action: crate::defense::DefenseAction::Hesitate(2.0),
                        origin_experience_id: ctx.experience.id.clone(),
                        intensity: 1.0,
                        created_at: chrono::Utc::now().to_rfc3339(),
                    };
                    tracing::warn!("🛡️ [SoulPipeline] Somatic Reflex triggered! Auto-generating semantic defense: {}", auto_defense.id);
                    ctx.soul.defenses.push(auto_defense);
                }
            }

            ctx.soul.compute_hash();

            if ctx.rebirth_required {
                let new_soul = self.engine.rebirth(ctx.soul.clone()).await?;
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
