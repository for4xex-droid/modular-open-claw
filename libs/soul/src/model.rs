/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
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

    #[serde(default)]
    pub last_begging_at: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(default)]
    pub semantic_index: Vec<SemanticSummary>,

    #[serde(default)]
    pub persona_boundaries: PersonaBoundaries,
}

impl AgentSoul {
    pub const MAX_CORE_MEMORY: usize = 50;

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
            semantic_index: Vec::new(),
            persona_boundaries: PersonaBoundaries::default(),
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
        // 1. Quota check for core memory (RED TEAM PATCH)
        if exp.is_core_memory {
            let core_count = self
                .experience_buffer
                .iter()
                .filter(|e| e.is_core_memory)
                .count();

            if core_count >= Self::MAX_CORE_MEMORY {
                // Find and demote the oldest core memory
                if let Some(oldest_core) =
                    self.experience_buffer.iter_mut().find(|e| e.is_core_memory)
                {
                    oldest_core.is_core_memory = false;
                    tracing::debug!(
                        "🧠 [AgentSoul] Core memory quota reached. Demoting oldest: {}",
                        oldest_core.id
                    );
                }
            }
        }

        self.experience_buffer.push(exp);

        // 2. Buffer size check with Core Memory Protection
        if self.experience_buffer.len() > Self::MAX_EXPERIENCE_BUFFER {
            let keep_len = Self::MAX_EXPERIENCE_BUFFER / 2;
            let mut to_remove = self.experience_buffer.len() - keep_len;

            // Remove non-core memories from the front first
            let mut new_buffer = Vec::with_capacity(self.experience_buffer.len());
            for e in self.experience_buffer.drain(..) {
                if to_remove > 0 && !e.is_core_memory {
                    to_remove -= 1;
                    continue;
                }
                new_buffer.push(e);
            }
            self.experience_buffer = new_buffer;

            // Emergency fallback: If still over limit (all core), FIFO the rest
            if self.experience_buffer.len() > Self::MAX_EXPERIENCE_BUFFER {
                let drain_count = self.experience_buffer.len() - Self::MAX_EXPERIENCE_BUFFER;
                self.experience_buffer.drain(0..drain_count);
            }
        }
    }

    /// Retrieve all core memories currently in the buffer
    pub fn core_memories(&self) -> Vec<&Experience> {
        self.experience_buffer
            .iter()
            .filter(|e| e.is_core_memory)
            .collect()
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
    #[serde(default)]
    pub is_core_memory: bool,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSummary {
    pub topic: String,
    pub compressed_insight: String,
    pub original_experience_ids: Vec<String>,
    pub valence_avg: f64,
    pub created_at: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonaBoundaries {
    pub forbidden_topics: Vec<String>,
    pub knowledge_scope: String,
    pub constraints: Vec<String>,
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
            is_core_memory: false,
            embedding: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defense::{Defense, DefenseAction, DefenseTrigger};

    #[test]
    fn test_temporal_decay() {
        let mut soul = AgentSoul::new("test-decay".to_string());

        let d1 = Defense {
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

    #[test]
    fn test_core_memory_protection() {
        let mut soul = AgentSoul::new("test-core".to_string());

        // 1. 最初の一件を核記憶として追加
        let mut core_exp = Experience::default();
        core_exp.is_core_memory = true;
        let core_id = core_exp.id.clone();
        soul.push_experience(core_exp);

        // 2. さらに 1000 件の通常記憶を追加してバッファを溢れさせる
        for i in 0..1000 {
            let mut exp = Experience::default();
            exp.content = format!("Normal experience {}", i);
            soul.push_experience(exp);
        }

        // RED 予測: 現状のの実装では FIFO で削除されるため、core_id は存在しないはず
        let found = soul.experience_buffer.iter().any(|e| e.id == core_id);
        assert!(found, "Core memory must be protected from FIFO pruning");
    }

    #[test]
    fn test_core_memory_quota() {
        let mut soul = AgentSoul::new("test-quota".to_string());

        // 1. Quota (50) いっぱいに核記憶を追加
        for i in 0..AgentSoul::MAX_CORE_MEMORY {
            let mut exp = Experience::default();
            exp.id = format!("core-{}", i);
            exp.is_core_memory = true;
            soul.push_experience(exp);
        }

        // 2. 51件目の核記憶を追加
        let mut overflow_exp = Experience::default();
        overflow_exp.id = "overflow".to_string();
        overflow_exp.is_core_memory = true;
        soul.push_experience(overflow_exp);

        // 3. 核記憶の総数が Quota を超えていないことを確認
        let core_count = soul
            .experience_buffer
            .iter()
            .filter(|e| e.is_core_memory)
            .count();
        assert!(
            core_count <= AgentSoul::MAX_CORE_MEMORY,
            "Core memory count must respect quota"
        );

        // 4. 最古の核記憶が降格（または削除）されていることを確認
        let oldest_found = soul
            .experience_buffer
            .iter()
            .any(|e| e.id == "core-0" && e.is_core_memory);
        assert!(
            !oldest_found,
            "Oldest core memory should be demoted/removed when quota is exceeded"
        );
    }
}
