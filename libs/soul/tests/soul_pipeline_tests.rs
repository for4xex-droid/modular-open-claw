/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use soul::adapter::SoulDomainAdapter;
use soul::defense::{Defense, DefenseAction, DefenseTrigger};
use soul::engine::SamsaraEngine;
use soul::error::SoulError;
use soul::instinct::Instinct;
use soul::model::{AgentSoul, Experience};
use soul::pipeline::SoulPipeline;

struct MockAdapter;
impl SoulDomainAdapter for MockAdapter {
    fn to_experience(&self, _event: &dyn Any) -> Experience {
        Experience::default()
    }
    fn distillation_system_prompt(&self) -> &str {
        "mock"
    }
    fn predict_outcome(&self, soul: &AgentSoul, exp: &Experience) -> f64 {
        if exp.outcome_valence == 0.0 {
            return 0.0;
        }
        if let Some(dm) = soul.predictive_model.domains.get(&exp.domain) {
            dm.prediction_accuracy * exp.outcome_valence.signum()
        } else {
            soul.predictive_model.global_surprise_sensitivity * 0.5 * exp.outcome_valence.signum()
        }
    }
    fn execute_defense<'a>(
        &'a self,
        _action: &'a DefenseAction,
        _context: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
    fn embed_experience<'a>(
        &'a self,
        _exp: &'a Experience,
    ) -> Pin<Box<dyn Future<Output = Vec<f32>> + Send + 'a>> {
        // Return dummy embedding for semantic tests
        Box::pin(async { vec![1.0, 0.0] })
    }
}

struct MockEngine;
impl SamsaraEngine for MockEngine {
    fn distill<'a>(
        &'a self,
        _soul: &'a AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<Instinct, SoulError>> + Send + 'a>> {
        Box::pin(async { Ok(Instinct::default()) })
    }
    fn rebirth<'a>(
        &'a self,
        mut soul: AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
        Box::pin(async move {
            soul.generation += 1;
            Ok(soul)
        })
    }
}

#[tokio::test]
async fn test_l1_tag_rejection() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());

    // Add a defense matching "bad_word"
    soul.defenses.push(Defense {
        id: "d1".into(),
        trigger: DefenseTrigger::Tag("bad_word".into()),
        action: DefenseAction::Reject,
        origin_experience_id: "e1".into(),
        intensity: 1.0,
        created_at: "now".into(),
    });

    let exp = Experience {
        content: "this is a bad_word".into(),
        domain: "chat".into(),
        ..Default::default()
    };

    let result = pipeline.process_experience(&mut soul, exp).await.unwrap();
    // Should be rejected by L1, return None, buffer shouldn't grow
    assert!(result.is_none());
    assert_eq!(soul.experience_buffer.len(), 0);
}

#[tokio::test]
async fn test_l2_plasticity_update() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());
    assert_eq!(soul.predictive_model.domains.len(), 0);

    let exp = Experience {
        domain: "chat".into(),
        outcome_valence: 1.0,
        original_prediction: 0.0,
        ..Default::default()
    };

    let _ = pipeline.process_experience(&mut soul, exp).await.unwrap();

    // DomainModel should be created and updated
    let dm = soul.predictive_model.domains.get("chat").unwrap();
    assert_eq!(dm.experience_count, 1);
}

#[tokio::test]
async fn test_l3_shock_triggers_rebirth() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());

    let exp = Experience {
        domain: "chat".into(),
        outcome_valence: 2.0, // High surprise to trigger shock (actual - predicted)
        original_prediction: -1.0,
        ..Default::default()
    };

    // First process will update plasticity and last_surprise = 3.0 > 0.8
    // L3 triggers rebirth and returns new soul (generation + 1)
    let new_soul_opt = pipeline.process_experience(&mut soul, exp).await.unwrap();
    assert!(new_soul_opt.is_some());
    let new_soul = new_soul_opt.unwrap();
    assert_eq!(new_soul.generation, 2);
}

#[tokio::test]
async fn test_buffer_rotation() {
    let mut soul = AgentSoul::new("test".into());
    for _ in 0..1001 {
        soul.push_experience(Experience::default());
    }
    assert_eq!(
        soul.experience_buffer.len(),
        AgentSoul::MAX_EXPERIENCE_BUFFER / 2
    );
}

#[tokio::test]
async fn test_predict_outcome_and_metrics_logic() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());
    let initial_hash = soul.soul_hash.clone();

    let exp_neutral = Experience {
        domain: "chat".into(),
        outcome_valence: 0.0,
        ..Default::default()
    };

    // First, test neutral outcome returns 0.0 prediction and updates hash/count
    pipeline
        .process_experience(&mut soul, exp_neutral)
        .await
        .unwrap();
    assert_eq!(soul.attachment.interaction_count, 1);
    assert_ne!(soul.soul_hash, initial_hash);

    let dm = soul.predictive_model.domains.get("chat").unwrap();
    // prediction was 0.0, outcome was 0.0. surprise = 0.0
    // prediction accuracy should improve
    assert!(dm.prediction_accuracy > 0.5);

    // Test known domain with positive valence
    let exp_positive = Experience {
        domain: "chat".into(),
        outcome_valence: 0.8,
        ..Default::default()
    };
    let pre_count = soul.attachment.interaction_count;
    let old_hash = soul.soul_hash.clone();
    pipeline
        .process_experience(&mut soul, exp_positive)
        .await
        .unwrap();
    assert_eq!(soul.attachment.interaction_count, pre_count + 1);
    assert_ne!(soul.soul_hash, old_hash);
}

#[tokio::test]
async fn test_somatic_marker_generation() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());

    // Low valence should not generate marker
    let exp_low = Experience {
        outcome_valence: 0.1,
        ..Default::default()
    };
    pipeline
        .process_experience(&mut soul, exp_low)
        .await
        .unwrap();
    assert_eq!(soul.somatic_markers.len(), 0);

    // High valence should generate marker
    let exp_high = Experience {
        outcome_valence: -0.8,
        ..Default::default()
    };
    pipeline
        .process_experience(&mut soul, exp_high)
        .await
        .unwrap();
    assert_eq!(soul.somatic_markers.len(), 1);
    assert_eq!(soul.somatic_markers[0].valence, -0.8);
    assert_eq!(soul.somatic_markers[0].arousal, 0.8);
}

#[tokio::test]
async fn test_l1_semantic_rejection() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());

    // Add semantic defense with identical dummy embedding
    soul.defenses.push(Defense {
        id: "d2".into(),
        trigger: DefenseTrigger::Semantic {
            embedding: vec![1.0, 0.0],
            threshold: 0.9,
        },
        action: DefenseAction::Reject,
        origin_experience_id: "e1".into(),
        intensity: 1.0,
        created_at: "now".into(),
    });

    let exp = Experience {
        content: "test payload".into(),
        domain: "chat".into(),
        ..Default::default()
    };

    let result = pipeline.process_experience(&mut soul, exp).await.unwrap();
    // Similarity will be 1.0 > 0.9, should be rejected
    assert!(result.is_none());
    assert_eq!(soul.experience_buffer.len(), 0);
}

#[tokio::test]
async fn test_execute_defense_hesitate() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());

    soul.defenses.push(Defense {
        id: "d3".into(),
        trigger: DefenseTrigger::Tag("hesitate_me".into()),
        action: DefenseAction::Hesitate(1.0),
        origin_experience_id: "e1".into(),
        intensity: 1.0,
        created_at: "now".into(),
    });

    let exp = Experience {
        content: "hesitate_me".into(),
        domain: "chat".into(),
        ..Default::default()
    };

    // Make sure process_experience completes successfully but returns None (bypassed)
    let result = pipeline.process_experience(&mut soul, exp).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_anamnesis_persists_across_rebirth() {
    let pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    let mut soul = AgentSoul::new("test".into());
    soul.anamnesis.narrative_self = Some("I am a test soul.".into());

    let exp = Experience {
        domain: "chat".into(),
        outcome_valence: 2.0, // Induce shock -> rebirth
        original_prediction: -1.0,
        ..Default::default()
    };

    let new_soul_opt = pipeline.process_experience(&mut soul, exp).await.unwrap();
    assert!(new_soul_opt.is_some());
    let new_soul = new_soul_opt.unwrap();

    // Anamnesis should be inherited
    assert_eq!(
        new_soul.anamnesis.narrative_self.as_deref(),
        Some("I am a test soul.")
    );
}

#[tokio::test]
async fn test_custom_middleware_injection() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TraceMiddleware {
        executed: Arc<AtomicBool>,
    }
    #[async_trait::async_trait]
    impl<A: soul::adapter::SoulDomainAdapter + 'static, E: soul::engine::SamsaraEngine + Send + Sync + 'static> soul::pipeline::SoulMiddleware<A, E> for TraceMiddleware {
        async fn process(&self, ctx: &mut soul::pipeline::SoulContext<'_, A, E>, next: &(dyn soul::pipeline::SoulMiddlewareNext<A, E> + '_)) -> Result<(), soul::error::SoulError> {
            self.executed.store(true, Ordering::SeqCst);
            next.run(ctx).await
        }
    }

    let executed = Arc::new(AtomicBool::new(false));
    let middleware = TraceMiddleware { executed: executed.clone() };

    let mut pipeline = SoulPipeline::new(MockAdapter, MockEngine);
    // RED: This method does not exist yet!
    pipeline.add_middleware(Box::new(middleware));

    let mut soul = AgentSoul::new("test".into());
    let exp = Experience::default();

    let _ = pipeline.process_experience(&mut soul, exp).await.unwrap();

    assert!(executed.load(Ordering::SeqCst), "Custom middleware should have been executed");
}
