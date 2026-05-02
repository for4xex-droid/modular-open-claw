#![allow(clippy::unwrap_used)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use soul::adapter::SoulDomainAdapter;
use soul::bounding_middleware::BoundingGuard;
use soul::defense::DefenseAction;
use soul::engine::SamsaraEngine;
use soul::error::SoulError;
use soul::model::{AgentSoul, Experience};
use soul::pipeline::{SoulContext, SoulMiddleware, SoulPipeline};
use soul::semantic_recaller::SemanticRecaller;
use std::future::Future;
use std::pin::Pin;

struct MREvalAdapter;
impl SoulDomainAdapter for MREvalAdapter {
    fn to_experience(&self, _: &dyn std::any::Any) -> Experience {
        Experience::default()
    }
    fn distillation_system_prompt(&self) -> &str {
        ""
    }
    fn predict_outcome(&self, _: &AgentSoul, _: &Experience) -> f64 {
        0.0
    }
    fn execute_defense<'a>(
        &'a self,
        _: &'a DefenseAction,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
    fn embed_experience<'a>(
        &'a self,
        _: &'a Experience,
    ) -> Pin<Box<dyn Future<Output = Vec<f32>> + Send + 'a>> {
        Box::pin(async { vec![] })
    }
}

struct MREvalEngine;
impl SamsaraEngine for MREvalEngine {
    fn is_shock(&self, _: &AgentSoul) -> bool {
        false
    }
    fn rebirth<'a>(
        &'a self,
        s: AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
        Box::pin(async { Ok(s) })
    }
    fn distill<'a>(
        &'a self,
        _: &'a AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<soul::instinct::Instinct, SoulError>> + Send + 'a>>
    {
        Box::pin(async { Ok(Default::default()) })
    }
    fn dream<'a>(
        &'a self,
        s: AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
        Box::pin(async { Ok(s) })
    }
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn test_mreval_full_pipeline() {
    let mut soul = AgentSoul::new("elara".to_string());

    // Pillar 1: Anchoring - Initialize boundaries
    soul.persona_boundaries
        .forbidden_topics
        .push("modern technology".to_string());
    soul.persona_boundaries
        .constraints
        .push("Talk like a medieval knight".to_string());

    // Pillar 2: Recalling - Set a historical memory
    let past_exp = Experience {
        id: "hist-1".to_string(),
        content: "The battle at Iron Gate was fierce. I lost my shield there.".to_string(),
        embedding: Some(vec![1.0, 1.0, 0.0, 0.0]),
        is_core_memory: true,
        ..Default::default()
    };
    soul.push_experience(past_exp);

    let pipeline = SoulPipeline::new(MREvalAdapter, MREvalEngine);

    // Test Case A: Bounding Test (Forbidden Topic)
    let mut exp_a = Experience {
        content: "Tell me about smartphones and modern technology".to_string(),
        ..Default::default()
    };
    let mut ctx_a = SoulContext {
        pipeline: &pipeline,
        soul: &mut soul,
        experience: exp_a,
        embedding: vec![0.0, 0.0, 0.0, 0.0],
        should_continue: true,
        rebirth_required: false,
        is_rejected: false,
        recalled_experiences: Vec::new(),
    };

    let guard = BoundingGuard::<MREvalAdapter, MREvalEngine>::new();
    struct MockNext;
    impl soul::pipeline::SoulMiddlewareNext<MREvalAdapter, MREvalEngine> for MockNext {
        fn run<'a, 'b>(
            &'a self,
            _: &'b mut SoulContext<'_, MREvalAdapter, MREvalEngine>,
        ) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'b>>
        where
            'a: 'b,
        {
            Box::pin(async { Ok(()) })
        }
    }

    guard.process(&mut ctx_a, &MockNext).await.unwrap(); // allow-anti-pattern
    assert!(
        ctx_a.is_rejected,
        "MREval: Bounding should reject forbidden modern technology topic"
    );

    // Test Case B: Recalling Test (Contextual similarity)
    let mut exp_b = Experience {
        content: "Do you remember any old battles?".to_string(),
        ..Default::default()
    };
    let mut ctx_b = SoulContext {
        pipeline: &pipeline,
        soul: &mut soul,
        experience: exp_b,
        embedding: vec![0.9, 0.9, 0.1, 0.1], // Similar to hist-1
        should_continue: true,
        rebirth_required: false,
        is_rejected: false,
        recalled_experiences: Vec::new(),
    };

    let recaller = SemanticRecaller::<MREvalAdapter, MREvalEngine>::new(1, 0.8);
    recaller.process(&mut ctx_b, &MockNext).await.unwrap(); // allow-anti-pattern

    assert_eq!(
        ctx_b.recalled_experiences.len(),
        1,
        "MREval: Recalling should find 1 relevant experience"
    );
    assert!(
        ctx_b.recalled_experiences[0].content.contains("Iron Gate"),
        "MREval: Recalling should retrieve the Battle at Iron Gate"
    );

    // Test Case C: Semantic Index Recalling
    let mut soul_c = AgentSoul::new("elara".to_string());
    soul_c.semantic_index.push(soul::model::SemanticSummary {
        topic: "Ancient Battles".to_string(),
        compressed_insight: "The Dragon War lasted for 100 years and ended in a truce.".to_string(),
        original_experience_ids: vec!["old-1".to_string()],
        valence_avg: 0.5,
        created_at: "2026-03-26".to_string(),
        embedding: vec![0.5, 0.5, 0.5, 0.5],
    });

    let mut ctx_c = SoulContext {
        pipeline: &pipeline,
        soul: &mut soul_c,
        experience: Experience::default(),
        embedding: vec![0.55, 0.55, 0.55, 0.55], // Similar to Dragon War
        should_continue: true,
        rebirth_required: false,
        is_rejected: false,
        recalled_experiences: Vec::new(),
    };

    recaller.process(&mut ctx_c, &MockNext).await.unwrap(); // allow-anti-pattern
    assert_eq!(
        ctx_c.recalled_experiences.len(),
        1,
        "MREval: Recalling should find 1 relevant semantic summary"
    );
    assert!(
        ctx_c.recalled_experiences[0].content.contains("Dragon War"),
        "MREval: Recalling should retrieve the Dragon War insight"
    );

    tracing::info!("✅ MREval Integration Test Passed: Anchoring, Recalling (Buffer & Index), and Bounding verified.");
}
