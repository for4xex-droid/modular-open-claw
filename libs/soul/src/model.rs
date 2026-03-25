/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

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

    // LoRA / Model Fine-tuning integration (G-13 / Phase 10.1b)
    #[serde(default)]
    pub lora_adapter_path: Option<String>,
    #[serde(default)]
    pub lora_base_model: Option<String>,
    #[serde(default)]
    pub lora_hash: Option<String>,

    // Phase 15-B: Dark pattern guardrail memory
    #[serde(default)]
    pub last_begging_at: Option<chrono::DateTime<chrono::Utc>>,
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
            lora_adapter_path: None,
            lora_base_model: None,
            lora_hash: None,
            last_begging_at: None,
        };
        soul.compute_hash();
        soul
    }

    pub fn compute_hash(&mut self) -> String {
        let payload = format!(
            "{}:{}:{}:{}:{}:{}",
            self.id,
            self.generation,
            self.instinct.hash,
            self.attachment.interaction_count,
            self.predictive_model.global_surprise_sensitivity,
            self.lora_hash.as_deref().unwrap_or("none")
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

    /// RS-3: Temporal decay mechanism for old memories and defenses
    pub fn apply_temporal_decay(&mut self) {
        let decay_rate = 0.995;
        let death_threshold = 0.2;

        self.defenses.iter_mut().for_each(|d| {
            d.intensity *= decay_rate;
        });
        self.defenses.retain(|d| d.intensity > death_threshold);

        self.somatic_markers.iter_mut().for_each(|m| {
            m.intensity *= decay_rate;
        });
        self.somatic_markers
            .retain(|m| m.intensity > death_threshold);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defense::{Defense, DefenseAction, DefenseTrigger};
    use crate::somatic::SomaticMarker;

    #[test]
    fn test_temporal_decay() {
        let mut soul = AgentSoul::new("test-decay".to_string());

        let mut d1 = Defense {
            id: "d1".to_string(),
            trigger: DefenseTrigger::Tag("foo".to_string()),
            action: DefenseAction::Hesitate(1.0),
            origin_experience_id: "e1".to_string(),
            intensity: 1.0,
            created_at: Utc::now().to_rfc3339(),
        };

        let mut d2 = d1.clone();
        d2.id = "d2".to_string();
        d2.intensity = 0.201; // barely above threshold

        soul.defenses.push(d1);
        soul.defenses.push(d2);

        soul.apply_temporal_decay();

        assert_eq!(soul.defenses.len(), 1); // d2 should be removed (0.201 * 0.995 < 0.2)
        assert!(soul.defenses[0].intensity < 1.0); // d1 should be decayed
    }

    #[test]
    fn test_soul_hash_change() {
        let mut soul = AgentSoul::new("test-hash".to_string());
        let hash1 = soul.soul_hash.clone();

        soul.generation += 1;
        let hash2 = soul.compute_hash();
        assert_ne!(hash1, hash2, "Hash must change when generation increases");

        soul.attachment.interaction_count += 10;
        let hash3 = soul.compute_hash();
        assert_ne!(
            hash2, hash3,
            "Hash must change when interaction count increases"
        );

        soul.lora_hash = Some("sha256:abcd".into());
        let hash4 = soul.compute_hash();
        assert_ne!(hash3, hash4, "Hash must change when lora_hash changes");
    }
}
