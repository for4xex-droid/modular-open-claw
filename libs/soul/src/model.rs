use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::somatic::SomaticMarker;
use crate::defense::Defense;
use crate::predictive::PredictiveModel;
use crate::attachment::AttachmentModel;
use crate::instinct::Instinct;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSoul {
    pub id: String,
    pub generation: u32,
    pub soul_hash: String,

    // L1: Reactive
    pub somatic_markers: Vec<SomaticMarker>,
    pub defenses: Vec<Defense>,

    // L2: Deliberative
    pub predictive_model: PredictiveModel,
    pub attachment: AttachmentModel,

    // L3: Meta-cognitive
    pub instinct: Instinct,
    pub experience_buffer: Vec<Experience>,
}

impl AgentSoul {
    pub fn compute_hash(&mut self) -> String {
        let payload = format!(
            "{}:{}:{}:{}:{}",
            self.id,
            self.generation,
            self.instinct.hash,
            self.attachment.interaction_count,
            self.predictive_model.global_surprise_sensitivity
        );
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        self.soul_hash = hash.clone();
        hash
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub domain: String,
    pub content: String,
    pub outcome_valence: f64, // -1.0 to 1.0 (actual outcome)
    pub timestamp: String,
    pub original_prediction: f64, // the model's prediction before outcome
}

impl Default for Experience {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            domain: "general".to_string(),
            content: String::new(),
            outcome_valence: 0.0,
            timestamp: Utc::now().to_rfc3339(),
            original_prediction: 0.0,
        }
    }
}
