use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnamnesisProfile {
    // 早期不適応スキーマ (Early Maladaptive Schemas)
    pub core_schemas: std::collections::HashMap<String, f64>,
    // ナラティブ・アイデンティティ (Narrative Identity)
    pub narrative_self: Option<String>,
}

impl Default for AnamnesisProfile {
    fn default() -> Self {
        Self {
            core_schemas: std::collections::HashMap::new(),
            narrative_self: None,
        }
    }
}
