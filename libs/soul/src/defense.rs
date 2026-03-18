use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defense {
    pub id: String,
    pub trigger: DefenseTrigger,
    pub action: DefenseAction,
    pub origin_experience_id: String,
    pub intensity: f64, // 0.0 - 1.0
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefenseTrigger {
    Tag(String),
    Semantic { embedding: Vec<f32>, threshold: f64 },
    Compound(Vec<DefenseTrigger>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefenseAction {
    Reject,
    Warn,
    RequireEscrow,
    Hesitate(f64),
    Deflect,
    Custom(String),
}
