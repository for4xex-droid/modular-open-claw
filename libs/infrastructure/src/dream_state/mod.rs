/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::trend_sonar::ExternalTrendSonar;
use aiome_core_contracts::traits::{JobQueue, TaskRegistry};
use rand::Rng;
use std::error::Error;
use std::sync::Arc;
use tracing::info;

use crate::aegis::incident_repo::IncidentRepository;
use crate::job_queue::CostOps;
use crate::llm::cost_breaker::CostCircuitBreaker;
use aiome_core_contracts::events::CoreEvent;
use tokio::sync::broadcast;

// Declare submodules
mod aegis;
mod biome;
mod communication;
mod crisis_guardian;
mod exploration;
mod observability;
mod reflection;
mod scientific;
mod tests;

/// HotSwap request structure (Phase 2)
#[derive(Debug, Clone, PartialEq)]
pub struct HotSwapRequest {
    pub incident_id: String,
    pub skill_name: String,
    pub patch_code: String,
}

/// Consolidated dream output structure
#[derive(Debug, Clone, PartialEq)]
pub struct DreamResult {
    pub insight: Option<String>,
    pub hot_swaps: Vec<HotSwapRequest>,
}

impl DreamResult {
    pub fn from_insight(insight: String) -> Self {
        Self {
            insight: Some(insight),
            hot_swaps: Vec::new(),
        }
    }
}

/// `DreamState` 構造体
pub struct DreamState {
    llm: Arc<dyn aiome_core::llm_provider::LlmProvider>,
    eval_logger: Option<Arc<crate::llm::evaluation_logger::EvaluationLogger>>,
    incident_repo: Option<Arc<IncidentRepository>>,
    event_sender: Option<broadcast::Sender<CoreEvent>>,
    biome_engine: Option<Arc<tokio::sync::RwLock<biome_engine::BiomeEngine>>>,
    soul_store: Option<Arc<crate::soul_store::UniversalSoulStore>>,
    cost_ops: Option<Arc<dyn CostOps>>,
}

impl DreamState {
    pub const MAX_CORE_MEMORY: usize = 50;

    /// 新しいインスタンスを生成する
    pub fn new(llm: Arc<dyn aiome_core::llm_provider::LlmProvider>) -> Self {
        Self {
            llm,
            eval_logger: None,
            incident_repo: None,
            event_sender: None,
            biome_engine: None,
            soul_store: None,
            cost_ops: None,
        }
    }

    pub fn with_cost_ops(mut self, ops: Arc<dyn CostOps>) -> Self {
        self.cost_ops = Some(ops);
        self
    }

    pub fn with_eval_logger(
        mut self,
        logger: Arc<crate::llm::evaluation_logger::EvaluationLogger>,
    ) -> Self {
        self.eval_logger = Some(logger);
        self
    }

    pub fn with_incident_repo(mut self, repo: Arc<IncidentRepository>) -> Self {
        self.incident_repo = Some(repo);
        self
    }

    pub fn with_event_sender(mut self, sender: broadcast::Sender<CoreEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    pub fn with_biome_engine(
        mut self,
        engine: Arc<tokio::sync::RwLock<biome_engine::BiomeEngine>>,
    ) -> Self {
        self.biome_engine = Some(engine);
        self
    }

    pub fn with_soul_store(mut self, store: Arc<crate::soul_store::UniversalSoulStore>) -> Self {
        self.soul_store = Some(store);
        self
    }

    /// 「夢想状態（Dream State）」を実行する。
    /// キューが空の時に、自発的なトレンド探索や過去の失敗への内省を行う。
    pub async fn dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
        level: i32,
    ) -> Result<Option<DreamResult>, Box<dyn Error + Send + Sync>> {
        // Layer 1: CostCircuitBreaker
        if let Some(ref ops) = self.cost_ops {
            let breaker = CostCircuitBreaker::new(ops.clone(), 10.0);
            if let Err(e) = breaker.enforce().await {
                tracing::warn!("🚨 [DreamState] Cost limit reached, skipping dream: {}", e);
                return Ok(None);
            }
        }

        info!(
            "💤 [DreamState] AI (Lv{}) is entering a contemplative Dream State...",
            level
        );

        // 1. Preemption Check: キューに仕事があるなら即座に起きる
        let pending = job_queue.get_pending_job_count().await?;
        if pending > 0 {
            info!("💤 [DreamState] Real tasks detected. Terminating dream and waking up.");
            return Ok(None);
        }

        // 2. Decide Dream Type
        let rand_val = rand::thread_rng().gen_range(0..100);

        // Level-based Behavioral Shift: Probability of communicative dream increases with level
        let comm_prob = ((level - 1) * 5).clamp(0, 50);
        let sci_prob = if level >= 5 { 20 } else { 0 };
        // Observability dreams get a dedicated 15% slot when eval_logger is connected
        let obs_prob = if self.eval_logger.is_some() { 15 } else { 0 };
        let aegis_prob = if self.incident_repo.is_some() { 10 } else { 0 };
        let biome_prob = if self.biome_engine.is_some() { 10 } else { 0 };

        let insight = if rand_val < comm_prob as i64 {
            self.communicative_dream(job_queue)
                .await?
                .map(DreamResult::from_insight)
        } else if rand_val < (comm_prob + sci_prob) as i64 {
            self.scientific_dream(job_queue)
                .await?
                .map(DreamResult::from_insight)
        } else if rand_val < (comm_prob + sci_prob + obs_prob) as i64 {
            self.observability_dream()
                .await?
                .map(DreamResult::from_insight)
        } else if rand_val < (comm_prob + sci_prob + obs_prob + aegis_prob) as i64 {
            self.aegis_sentinel_dream().await?
        } else if rand_val < (comm_prob + sci_prob + obs_prob + aegis_prob + biome_prob) as i64 {
            self.biome_evolution_dream(job_queue)
                .await?
                .map(DreamResult::from_insight)
        } else if rand_val % 2 == 0 {
            self.explorative_dream(job_queue, trend_sonar)
                .await?
                .map(DreamResult::from_insight)
        } else {
            self.reflective_dream(job_queue)
                .await?
                .map(DreamResult::from_insight)
        };

        Ok(insight)
    }
}
