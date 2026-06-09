/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SomaticMarker {
    pub id: String,
    pub embedding: Vec<f32>,
    pub valence: f64,   // -1.0 to 1.0 (displeasure to pleasure)
    pub arousal: f64,   // 0.0 to 1.0 (calm to highly aroused)
    pub intensity: f64, // time-decayed intensity
    pub created_at: String,
    #[serde(default)]
    pub is_permanent: bool,
}

impl SomaticMarker {
    pub fn resonance(&self, input_embedding: &[f32]) -> f64 {
        let similarity = math_utils::cosine_similarity(&self.embedding, input_embedding);

        // R-5: Clamp bounded values just in case
        let _v = self.valence.clamp(-1.0, 1.0);
        let a = self.arousal.clamp(0.0, 1.0);
        let i = self.intensity.clamp(0.0, 1.0);

        similarity * i * a
    }

    pub fn new_clamped(
        id: String,
        embedding: Vec<f32>,
        valence: f64,
        arousal: f64,
        intensity: f64,
        created_at: String,
    ) -> Self {
        Self {
            id,
            embedding,
            valence: valence.clamp(-1.0, 1.0),
            arousal: arousal.clamp(0.0, 1.0),
            intensity: intensity.clamp(0.0, 1.0),
            created_at,
            is_permanent: false,
        }
    }
}

pub mod math_utils {
    pub fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f64 {
        if v1.is_empty() || v2.is_empty() || v1.len() != v2.len() {
            return 0.0;
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_v1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_v2: f32 = v2.iter().map(|b| b * b).sum::<f32>().sqrt();
        if norm_v1 == 0.0 || norm_v2 == 0.0 {
            return 0.0;
        }
        (dot_product / (norm_v1 * norm_v2)) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((math_utils::cosine_similarity(&v1, &v2) - 1.0).abs() < f64::EPSILON);

        let v3 = vec![0.0, 1.0, 0.0];
        assert!((math_utils::cosine_similarity(&v1, &v3)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_somatic_resonance() {
        let marker = SomaticMarker {
            id: "1".into(),
            embedding: vec![1.0, 1.0],
            valence: 0.5,
            arousal: 0.8,
            intensity: 0.9,
            created_at: "test".into(),
            is_permanent: false,
        };

        let resonance = marker.resonance(&[1.0, 1.0]);
        // similarity is 1.0, so resonance = 1.0 * 0.9 * 0.8 = 0.72
        assert!((resonance - 0.72).abs() < 1e-5);
    }
}
