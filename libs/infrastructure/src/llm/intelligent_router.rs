/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::route_rules::{decide_route, RouteRulesConfig};
use crate::job_queue::CostOps;
use crate::llm::cost_breaker::CostCircuitBreaker;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::llm::{
    LlmRouteDecision, RESOLVED_MODEL_KEY, RESOLVED_PROVIDER_KEY, ROUTE_MODE_KEY, ROUTE_REASON_KEY,
    ROUTE_TIER_KEY, ROUTE_TIER_LOCKED_KEY,
};
use aiome_core_contracts::task_tier::TaskTier;
use async_trait::async_trait;
use shared::config::LlmRouteMode;
use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

/// Rule-based tier selector for the chat LLM path (ADR-058).
pub struct IntelligentRouter {
    mode: LlmRouteMode,
    budget_degrade: bool,
    rules_config: RouteRulesConfig,
    fast_chain: Arc<dyn LlmProvider + Send + Sync>,
    fast_chain_degraded: Arc<dyn LlmProvider + Send + Sync>,
    smart_chain: Arc<dyn LlmProvider + Send + Sync>,
    cost_ops: Arc<dyn CostOps>,
    default_cost_limit_usd: f64,
}

impl fmt::Debug for IntelligentRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntelligentRouter")
            .field("mode", &self.mode)
            .field("budget_degrade", &self.budget_degrade)
            .finish_non_exhaustive()
    }
}

impl IntelligentRouter {
    pub fn new(
        mode: LlmRouteMode,
        budget_degrade: bool,
        short_prompt_chars: usize,
        fast_chain: Arc<dyn LlmProvider + Send + Sync>,
        fast_chain_degraded: Arc<dyn LlmProvider + Send + Sync>,
        smart_chain: Arc<dyn LlmProvider + Send + Sync>,
        cost_ops: Arc<dyn CostOps>,
        default_cost_limit_usd: f64,
    ) -> Self {
        Self {
            mode,
            budget_degrade,
            rules_config: RouteRulesConfig { short_prompt_chars },
            fast_chain,
            fast_chain_degraded,
            smart_chain,
            cost_ops,
            default_cost_limit_usd,
        }
    }

    fn mode_str(&self) -> &'static str {
        match self.mode {
            LlmRouteMode::Legacy => "legacy",
            LlmRouteMode::Rules => "rules",
        }
    }

    async fn resolve_decision(
        &self,
        prompt: &str,
        format: Option<&str>,
        metadata: Option<&HashMap<String, String>>,
    ) -> LlmRouteDecision {
        if self.mode == LlmRouteMode::Legacy {
            return LlmRouteDecision {
                tier: TaskTier::Smart,
                reason_code: "legacy_mode".to_string(),
                reason_detail: "LLM_ROUTE_MODE=legacy always uses Smart chain".to_string(),
            };
        }

        // EntropyGate リトライで確定した tier は予算降格より優先（ADR-058 §6）
        if let Some(meta) = metadata {
            if meta.contains_key(ROUTE_TIER_LOCKED_KEY) {
                return decide_route(prompt, format, Some(meta), &self.rules_config);
            }
        }

        if self.budget_degrade && self.degrade_recommended().await {
            return LlmRouteDecision {
                tier: TaskTier::Fast,
                reason_code: "budget_degrade".to_string(),
                reason_detail: "Cost circuit breaker tripped; forcing Fast tier".to_string(),
            };
        }

        decide_route(prompt, format, metadata, &self.rules_config)
    }

    async fn degrade_recommended(&self) -> bool {
        if !self.budget_degrade {
            return false;
        }
        let breaker = CostCircuitBreaker::new(self.cost_ops.clone(), self.default_cost_limit_usd);
        breaker
            .check_state()
            .await
            .map(|s| s.is_tripped)
            .unwrap_or(false)
    }

    fn chain_for_tier(&self, tier: TaskTier, degraded: bool) -> Arc<dyn LlmProvider + Send + Sync> {
        match tier {
            TaskTier::Fast if degraded => self.fast_chain_degraded.clone(),
            TaskTier::Fast => self.fast_chain.clone(),
            TaskTier::Smart => self.smart_chain.clone(),
        }
    }

    async fn pick_chain(&self, decision: &LlmRouteDecision) -> Arc<dyn LlmProvider + Send + Sync> {
        let degraded = decision.reason_code == "budget_degrade";
        self.chain_for_tier(decision.tier, degraded)
    }

    fn extract_prompt_parts(request: &LlmRequest) -> (String, Option<String>, Option<String>) {
        let mut system = None;
        let mut prompt = String::new();
        for m in &request.messages {
            if m.role == "system" {
                system = Some(m.content.clone());
            } else if m.role == "user" {
                prompt = m.content.clone();
            }
        }
        (prompt, system, request.format.clone())
    }

    fn inject_route_metadata(request: &mut LlmRequest, decision: &LlmRouteDecision, mode: &str) {
        let meta = request.metadata.get_or_insert_with(HashMap::new);
        meta.insert(
            ROUTE_TIER_KEY.to_string(),
            tier_label(decision.tier).to_string(),
        );
        meta.insert(ROUTE_REASON_KEY.to_string(), decision.reason_code.clone());
        meta.insert(ROUTE_MODE_KEY.to_string(), mode.to_string());
        meta.insert(
            ROUTE_TIER_LOCKED_KEY.to_string(),
            tier_label(decision.tier).to_string(),
        );
    }

    fn enrich_response_metadata(
        mut response: LlmResponse,
        chain: &Arc<dyn LlmProvider + Send + Sync>,
        decision: &LlmRouteDecision,
        mode: &str,
    ) -> LlmResponse {
        let meta = response.metadata.get_or_insert_with(HashMap::new);
        meta.insert(
            ROUTE_TIER_KEY.to_string(),
            tier_label(decision.tier).to_string(),
        );
        meta.insert(ROUTE_REASON_KEY.to_string(), decision.reason_code.clone());
        meta.insert(ROUTE_MODE_KEY.to_string(), mode.to_string());
        meta.insert(
            ROUTE_TIER_LOCKED_KEY.to_string(),
            tier_label(decision.tier).to_string(),
        );
        meta.insert(RESOLVED_PROVIDER_KEY.to_string(), chain.name().to_string());
        meta.insert(RESOLVED_MODEL_KEY.to_string(), chain.name().to_string());
        response
    }
}

fn tier_label(tier: TaskTier) -> &'static str {
    match tier {
        TaskTier::Fast => "fast",
        TaskTier::Smart => "smart",
    }
}

#[async_trait]
impl LlmProvider for IntelligentRouter {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let mut request = LlmRequest {
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };
        if let Some(sys) = system {
            request
                .messages
                .push(aiome_core_contracts::llm::LlmMessage {
                    role: "system".to_string(),
                    content: sys.to_string(),
                    cache: true,
                });
        }
        request
            .messages
            .push(aiome_core_contracts::llm::LlmMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                cache: false,
            });
        self.complete_with_cache(request).await
    }

    async fn complete_with_cache(
        &self,
        mut request: LlmRequest,
    ) -> Result<LlmResponse, AiomeError> {
        let (prompt, _system, format) = Self::extract_prompt_parts(&request);
        let decision = self
            .resolve_decision(&prompt, format.as_deref(), request.metadata.as_ref())
            .await;
        Self::inject_route_metadata(&mut request, &decision, self.mode_str());
        let chain = self.pick_chain(&decision).await;
        let response = chain.complete_with_cache(request).await?;
        Ok(Self::enrich_response_metadata(
            response,
            &chain,
            &decision,
            self.mode_str(),
        ))
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        // Streaming requires Smart chain (BackgroundLlm has no stream implementation).
        self.smart_chain.stream_complete(prompt, system).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.smart_chain.test_connection().await?;
        self.fast_chain.test_connection().await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "IntelligentRouter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::MockLlmProvider;

    fn mock_chain(label: &str) -> Arc<dyn LlmProvider + Send + Sync> {
        Arc::new(MockLlmProvider {
            response: label.to_string(),
            should_fail: false,
        })
    }

    fn test_router(mode: LlmRouteMode) -> IntelligentRouter {
        IntelligentRouter::new(
            mode,
            false,
            512,
            mock_chain("fast"),
            mock_chain("fast_degraded"),
            mock_chain("smart"),
            Arc::new(crate::testing::mock_jq::MockJQ::default()),
            10.0,
        )
    }

    #[tokio::test]
    async fn test_legacy_always_smart() {
        let router = test_router(LlmRouteMode::Legacy);
        let resp = router.complete("hi", None).await.unwrap();
        assert_eq!(resp.content, "smart");
    }

    #[tokio::test]
    async fn test_rules_short_prompt_fast() {
        let router = test_router(LlmRouteMode::Rules);
        let resp = router.complete("hello", None).await.unwrap();
        assert_eq!(resp.content, "fast");
    }

    #[tokio::test]
    async fn test_stream_always_smart() {
        let router = test_router(LlmRouteMode::Rules);
        let mut stream = router.stream_complete("hello", None).await.unwrap();
        use futures::StreamExt;
        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk, "smart");
    }

    #[tokio::test]
    async fn test_locked_metadata_keeps_fast_on_long_prompt() {
        let router = test_router(LlmRouteMode::Rules);
        let mut meta = HashMap::new();
        meta.insert(ROUTE_TIER_LOCKED_KEY.to_string(), "fast".to_string());
        let request = LlmRequest {
            messages: vec![aiome_core_contracts::llm::LlmMessage {
                role: "user".to_string(),
                content: "x".repeat(600),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: Some(meta),
        };
        let resp = router.complete_with_cache(request).await.unwrap();
        assert_eq!(resp.content, "fast");
        assert_eq!(
            resp.metadata
                .as_ref()
                .and_then(|m| m.get(ROUTE_TIER_KEY))
                .map(String::as_str),
            Some("fast")
        );
    }

    #[derive(Debug)]
    struct TrippedCostOps;

    #[async_trait]
    impl CostOps for TrippedCostOps {
        async fn aggregate_cost_hours(&self, _hours: i64) -> Result<f64, AiomeError> {
            Ok(100.0)
        }
        async fn aggregate_cost_days(&self, _days: i64) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn aggregate_cost_by_job(&self, _job_id: &str) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
    }

    #[async_trait]
    impl aiome_core_contracts::traits::SettingsOps for TrippedCostOps {
        async fn do_get_setting(&self, _key: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
        async fn do_set_setting(
            &self,
            _k: &str,
            _v: &str,
            _c: &str,
            _s: bool,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_get_all_settings(
            &self,
        ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
            Ok(vec![])
        }
        async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
            Ok(false)
        }
        async fn set_auto_expression_enabled(&self, _e: bool) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_budget_degrade_forces_fast() {
        let router = IntelligentRouter::new(
            LlmRouteMode::Rules,
            true,
            512,
            mock_chain("fast"),
            mock_chain("fast_degraded"),
            mock_chain("smart"),
            Arc::new(TrippedCostOps),
            10.0,
        );
        let long = "x".repeat(600);
        let resp = router.complete(&long, None).await.unwrap();
        assert_eq!(resp.content, "fast_degraded");
        assert_eq!(
            resp.metadata
                .as_ref()
                .and_then(|m| m.get(ROUTE_REASON_KEY))
                .map(String::as_str),
            Some("budget_degrade")
        );
    }

    #[tokio::test]
    async fn test_sticky_smart_overrides_budget_degrade() {
        let router = IntelligentRouter::new(
            LlmRouteMode::Rules,
            true,
            512,
            mock_chain("fast"),
            mock_chain("fast_degraded"),
            mock_chain("smart"),
            Arc::new(TrippedCostOps),
            10.0,
        );
        let mut meta = HashMap::new();
        meta.insert(ROUTE_TIER_LOCKED_KEY.to_string(), "smart".to_string());
        let request = LlmRequest {
            messages: vec![aiome_core_contracts::llm::LlmMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: Some(meta),
        };
        let resp = router.complete_with_cache(request).await.unwrap();
        assert_eq!(
            resp.content, "smart",
            "Locked Smart tier must win over budget degrade"
        );
        assert_eq!(
            resp.metadata
                .as_ref()
                .and_then(|m| m.get(ROUTE_REASON_KEY))
                .map(String::as_str),
            Some("tier_locked")
        );
    }
}
