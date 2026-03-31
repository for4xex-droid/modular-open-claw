/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use async_trait::async_trait;
use soul::adapter::SoulDomainAdapter;
use soul::engine::SamsaraEngine;
use soul::error::SoulError;
use soul::pipeline::{SoulContext, SoulMiddleware, SoulMiddlewareNext};

/// L2.5: Whisper Layer (エージェントの内面的な自問自答)
pub struct WhisperMiddleware<A, E> {
    _phantom: std::marker::PhantomData<(A, E)>,
}

impl<A, E> Default for WhisperMiddleware<A, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A, E> WhisperMiddleware<A, E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static> SoulMiddleware<A, E>
    for WhisperMiddleware<A, E>
{
    async fn process(
        &self,
        ctx: &mut SoulContext<'_, A, E>,
        next: &(dyn SoulMiddlewareNext<A, E> + '_),
    ) -> Result<(), SoulError> {
        // エージェントの「内なる声」をシミュレーション
        // 実際にはここでLLMを用いた内省（Self-Reflection）を行うことも可能
        let content = &ctx.experience.content;

        if !content.is_empty() && !content.contains("Whisper:") {
            let whisper = if ctx.experience.outcome_valence < -0.3 {
                "\nWhisper: Something feels off. I should be more cautious next time."
            } else if ctx.experience.outcome_valence > 0.3 {
                "\nWhisper: This went well. I'm starting to understand this user better."
            } else {
                "\nWhisper: Processing current interaction. Stability is maintained."
            };

            ctx.experience.content.push_str(whisper);
            tracing::info!("🧠 [WhisperMiddleware] Thought appended to experience.");
        }

        // 想起された記憶（Recalled Experiences）の反映
        if !ctx.recalled_experiences.is_empty() {
            let recalled_ids: Vec<String> = ctx
                .recalled_experiences
                .iter()
                .map(|e| e.id.chars().take(8).collect())
                .collect();
            let recall_thought = format!("\nWhisper: Recalling relevant patterns from past interactions: [{}]. Applying these contexts to current response.", recalled_ids.join(", "));
            ctx.experience.content.push_str(&recall_thought);
        }

        next.run(ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soul::model::{AgentSoul, Experience};
    use soul::pipeline::SoulPipeline;
    use std::future::Future;
    use std::pin::Pin;

    struct DummyEngine;
    impl SamsaraEngine for DummyEngine {
        fn is_shock(&self, _: &AgentSoul) -> bool {
            false
        }
        fn distill<'a>(
            &'a self,
            _: &'a AgentSoul,
        ) -> Pin<Box<dyn Future<Output = Result<soul::instinct::Instinct, SoulError>> + Send + 'a>>
        {
            Box::pin(async { Ok(Default::default()) })
        }
        fn rebirth<'a>(
            &'a self,
            soul: AgentSoul,
        ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
            Box::pin(async { Ok(soul) })
        }
        fn dream<'a>(
            &'a self,
            soul: AgentSoul,
        ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
            Box::pin(async { Ok(soul) })
        }
    }

    struct DummyAdapter;
    impl SoulDomainAdapter for DummyAdapter {
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
            _: &'a soul::defense::DefenseAction,
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

    #[tokio::test]
    async fn test_whisper_middleware_appends_thought() {
        let mut soul = AgentSoul::new("whisper-test".into());
        let pipeline = SoulPipeline::new(DummyAdapter, DummyEngine);
        let mut exp = Experience {
            content: "Hello".to_string(),
            ..Default::default()
        };

        // Manual context setup for testing middleware directly
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

        struct MockNext;
        impl<A: SoulDomainAdapter + 'static, E: SamsaraEngine + Send + Sync + 'static>
            SoulMiddlewareNext<A, E> for MockNext
        {
            fn run<'a, 'b>(
                &'a self,
                _ctx: &'b mut SoulContext<'_, A, E>,
            ) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'b>>
            where
                'a: 'b,
            {
                Box::pin(async { Ok(()) })
            }
        }

        let middleware = WhisperMiddleware::<DummyAdapter, DummyEngine>::new();
        let result = middleware.process(&mut ctx, &MockNext).await;

        // RED: Should fail with "not implemented" or similar
        assert!(result.is_ok(), "Middleware should return Ok");
        assert!(
            ctx.experience.content.contains("Whisper:"),
            "Should contain whisper thought"
        );
    }
}
