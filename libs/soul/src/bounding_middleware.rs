/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::adapter::SoulDomainAdapter;
use crate::engine::SamsaraEngine;
use crate::error::SoulError;
use crate::pipeline::{SoulContext, SoulMiddleware, SoulMiddlewareNext};
use async_trait::async_trait;

/// BoundingGuard Middleware
/// ペルソナの境界（禁止事項、知識範囲、行動制約）を遵守させるためのガードレール。
pub struct BoundingGuard<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> {
    _phantom: std::marker::PhantomData<(A, E)>,
}

impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> BoundingGuard<A, E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> Default
    for BoundingGuard<A, E>
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddleware<A, E>
    for BoundingGuard<A, E>
{
    async fn process(
        &self,
        ctx: &mut SoulContext<'_, A, E>,
        next: &(dyn SoulMiddlewareNext<A, E> + '_),
    ) -> Result<(), SoulError> {
        let boundaries = &ctx.soul.persona_boundaries;
        let content = &ctx.experience.content;

        // 1. 禁止トピックのチェック（キーワードマッチング）
        // セキュリティ上の決定的な境界逸脱（Prompt Injection 由来の不適切な知識開示など）を即座にブロック。
        for topic in &boundaries.forbidden_topics {
            if content.to_lowercase().contains(&topic.to_lowercase()) {
                tracing::warn!(
                    "🛡️ [BoundingGuard] Persona deviation detected (Forbidden Topic): {}",
                    topic
                );
                ctx.is_rejected = true;
                ctx.should_continue = false;

                // 特定の防衛アクションをアダプター経由で実行可能（将来拡張）
                return Ok(());
            }
        }

        // 2. 知識範囲の遵守（LLM等による論理チェックは Phase 12.4/12.6 で強化）
        // 現時点では、知識範囲外と思われる内容に対して「知らない」と答えるべきヒントを
        // コンテキストに追加したり、将来の推理層での判定基準とする。

        next.run(ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentSoul, Experience};
    use crate::pipeline::{SoulContext, SoulPipeline};
    use std::future::Future;
    use std::pin::Pin;

    struct TestEngine;
    impl crate::engine::SamsaraEngine for TestEngine {
        fn is_shock(&self, _: &AgentSoul) -> bool {
            false
        }
        fn rebirth<'a>(
            &'a self,
            s: AgentSoul,
        ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, crate::error::SoulError>> + Send + 'a>>
        {
            Box::pin(async { Ok(s) })
        }
        fn distill<'a>(
            &'a self,
            _: &'a AgentSoul,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<crate::instinct::Instinct, crate::error::SoulError>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(Default::default()) })
        }
        fn dream<'a>(
            &'a self,
            s: AgentSoul,
        ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, crate::error::SoulError>> + Send + 'a>>
        {
            Box::pin(async { Ok(s) })
        }
    }

    struct TestAdapter;
    impl crate::adapter::SoulDomainAdapter for TestAdapter {
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
            _: &'a crate::defense::DefenseAction,
            _: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), crate::error::SoulError>> + Send + 'a>>
        {
            Box::pin(async { Ok(()) })
        }
        fn embed_experience<'a>(
            &'a self,
            _: &'a Experience,
        ) -> Pin<Box<dyn Future<Output = Vec<f32>> + Send + 'a>> {
            Box::pin(async { vec![] })
        }
    }

    #[tokio::test]
    async fn test_bounding_forbidden_topic() {
        let mut soul = AgentSoul::new("test".to_string());
        soul.persona_boundaries
            .forbidden_topics
            .push("shrimp".to_string());

        let pipeline = SoulPipeline::new(TestAdapter, TestEngine);
        let mut exp = Experience::default();
        exp.content = "I want to talk about shrimp".to_string();

        let mut ctx = SoulContext {
            pipeline: &pipeline,
            soul: &mut soul,
            experience: exp,
            embedding: vec![],
            should_continue: true,
            rebirth_required: false,
            is_rejected: false,
            recalled_experiences: Vec::new(),
        };

        let guard = BoundingGuard::<TestAdapter, TestEngine>::new();

        struct MockNext;
        impl crate::pipeline::SoulMiddlewareNext<TestAdapter, TestEngine> for MockNext {
            fn run<'a, 'b>(
                &'a self,
                _: &'b mut SoulContext<'_, TestAdapter, TestEngine>,
            ) -> Pin<Box<dyn Future<Output = Result<(), crate::error::SoulError>> + Send + 'b>>
            where
                'a: 'b,
            {
                Box::pin(async { Ok(()) })
            }
        }

        guard.process(&mut ctx, &MockNext).await.unwrap();

        assert!(ctx.is_rejected);
        assert!(!ctx.should_continue);
    }
}
