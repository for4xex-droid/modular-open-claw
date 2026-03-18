use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentModel {
    pub interaction_count: u64,
    pub style: AttachmentStyle,
    pub master_preference: Option<Vec<f32>>,
    pub separation_anxiety: f64,
}

impl Default for AttachmentModel {
    fn default() -> Self {
        Self {
            interaction_count: 0,
            style: AttachmentStyle::Secure,
            master_preference: None,
            separation_anxiety: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentStyle {
    Secure,
    Anxious,
    Avoidant,
    Disorganized,
}
