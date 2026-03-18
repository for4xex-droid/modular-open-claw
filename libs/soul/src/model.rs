use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::attachment::AttachmentModel;
use crate::defense::Defense;
use crate::instinct::Instinct;
use crate::predictive::PredictiveModel;
use crate::somatic::SomaticMarker;

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
    pub anamnesis: crate::anamnesis::AnamnesisProfile,
    pub experience_buffer: Vec<Experience>,
}

impl AgentSoul {
    pub fn new(id: String) -> Self {
        let mut soul = Self {
            id,
            generation: 1,
            soul_hash: String::new(),
            somatic_markers: Vec::new(),
            defenses: Vec::new(),
            predictive_model: PredictiveModel::default(),
            attachment: AttachmentModel::default(),
            instinct: Instinct::default(),
            anamnesis: crate::anamnesis::AnamnesisProfile::default(),
            experience_buffer: Vec::new(),
        };
        soul.compute_hash();
        soul
    }

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

    pub const MAX_EXPERIENCE_BUFFER: usize = 1000;

    /// Add an experience to the buffer, ensuring a maximum bounded size
    pub fn push_experience(&mut self, exp: Experience) {
        self.experience_buffer.push(exp);
        if self.experience_buffer.len() > Self::MAX_EXPERIENCE_BUFFER {
            // Keep the latest half when rotating to avoid frequent rotation cost
            let keep_len = Self::MAX_EXPERIENCE_BUFFER / 2;
            let drain_count = self.experience_buffer.len() - keep_len;
            self.experience_buffer.drain(0..drain_count);
        }
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
