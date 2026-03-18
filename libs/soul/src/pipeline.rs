use std::future::Future;
use std::pin::Pin;

use crate::error::SoulError;
use crate::model::{AgentSoul, Experience};
use crate::engine::SamsaraEngine;
use crate::adapter::SoulDomainAdapter;

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
    pub fn is_rejected_by_reactive_layer(&self, soul: &AgentSoul, exp: &Experience) -> bool {
        // Evaluate semantic tags and triggers logic...
        let _ = soul;
        let _ = exp;
        false
    }

    /// パイプライン全体を実行
    pub fn process_experience<'a>(
        &'a self,
        soul: &'a mut AgentSoul,
        exp: Experience,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AgentSoul>, SoulError>> + Send + 'a>> {
        Box::pin(async move {
            // L1: Reactive Layer Check
            if self.is_rejected_by_reactive_layer(soul, &exp) {
                // Execute defense action...
                return Ok(None);
            }

            // L2: Deliberative Layer
            soul.predictive_model.update_plasticity(&exp.domain, exp.outcome_valence, exp.original_prediction);
            
            soul.experience_buffer.push(exp);

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
