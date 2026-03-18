use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SomaticMarker {
    pub id: String,
    pub embedding: Vec<f32>,
    pub valence: f64,    // -1.0 to 1.0 (displeasure to pleasure)
    pub arousal: f64,    // 0.0 to 1.0 (calm to highly aroused)
    pub intensity: f64,  // time-decayed intensity
    pub created_at: String,
}

impl SomaticMarker {
    pub fn resonance(&self, input_embedding: &[f32]) -> f64 {
        let similarity = math_utils::cosine_similarity(&self.embedding, input_embedding);
        similarity * self.intensity * self.arousal
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
