/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::adapter::SoulDomainAdapter;
use crate::engine::SamsaraEngine;
use crate::error::SoulError;
use crate::model::Experience;
use crate::pipeline::{SoulContext, SoulMiddleware, SoulMiddlewareNext};
use crate::somatic::math_utils::cosine_similarity;
use async_trait::async_trait;

/// SemanticRecaller Middleware
/// 会話入力の Embedding と過去体験の Embedding を照合し、関連する記憶を想起コンテキストとして注入する。
pub struct SemanticRecaller<
    A: SoulDomainAdapter + 'static,
    E: SamsaraEngine + Send + Sync + 'static,
> {
    pub max_recall_items: usize,
    pub threshold: f64,
    _phantom: std::marker::PhantomData<(A, E)>,
}

impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static>
    SemanticRecaller<A, E>
{
    pub fn new(max_recall_items: usize, threshold: f64) -> Self {
        Self {
            max_recall_items,
            threshold,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddleware<A, E>
    for SemanticRecaller<A, E>
{
    async fn process(
        &self,
        ctx: &mut SoulContext<'_, A, E>,
        next: &(dyn SoulMiddlewareNext<A, E> + '_),
    ) -> Result<(), SoulError> {
        let query_embedding = &ctx.embedding;
        if query_embedding.is_empty() {
            return next.run(ctx).await;
        }

        let mut matched = Vec::new();

        // 1. experience_buffer 内の過去体験（短期〜中期記憶）を検索
        for exp in &ctx.soul.experience_buffer {
            if let Some(emb) = &exp.embedding {
                let score = cosine_similarity(query_embedding, emb);
                if score > self.threshold {
                    matched.push((score, exp.clone()));
                }
            }
        }

        // 2. semantic_index（圧縮された意味記憶）の検索
        for summary in &ctx.soul.semantic_index {
            let score = cosine_similarity(query_embedding, &summary.embedding);
            if score > self.threshold {
                let exp = Experience {
                    content: format!(
                        "[Core Insight: {}] {}",
                        summary.topic, summary.compressed_insight
                    ),
                    embedding: Some(summary.embedding.clone()),
                    outcome_valence: summary.valence_avg,
                    ..Default::default()
                };
                matched.push((score, exp));
            }
        }

        // スコア降順でソートし、上位 N 件を抽出
        matched.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        ctx.recalled_experiences = matched
            .into_iter()
            .take(self.max_recall_items)
            .map(|(_, exp)| exp)
            .collect();

        if !ctx.recalled_experiences.is_empty() {
            tracing::info!(
                "🧠 [SemanticRecaller] Recalled {} relevant experiences",
                ctx.recalled_experiences.len()
            );
        }

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
    async fn test_semantic_recall() {
        let mut soul = AgentSoul::new("test".to_string());

        // 1. 過去の記憶を準備（ベクトル付き）
        let mut exp1 = Experience::default();
        exp1.content = "I love cats".to_string();
        exp1.embedding = Some(vec![1.0, 0.0]);
        soul.push_experience(exp1);

        let mut exp2 = Experience::default();
        exp2.content = "Space is vast".to_string();
        exp2.embedding = Some(vec![0.0, 1.0]);
        soul.push_experience(exp2);

        // 2. 現在のコンテキスト（"cats" に近いベクトル）
        let pipeline = SoulPipeline::new(TestAdapter, TestEngine);
        let mut ctx = SoulContext {
            pipeline: &pipeline,
            soul: &mut soul,
            experience: Experience::default(),
            embedding: vec![0.9, 0.1], // cats に近い
            should_continue: true,
            rebirth_required: false,
            is_rejected: false,
            recalled_experiences: Vec::new(),
        };

        let recaller = SemanticRecaller::<TestAdapter, TestEngine>::new(1, 0.7);

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

        recaller.process(&mut ctx, &MockNext).await.unwrap(); // allow-anti-pattern

        // 3. 検証
        assert_eq!(ctx.recalled_experiences.len(), 1);
        assert_eq!(ctx.recalled_experiences[0].content, "I love cats");
    }
}
