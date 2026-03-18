use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnamnesisProfile {
    // 早期不適応スキーマ (Early Maladaptive Schemas)
    pub core_schemas: std::collections::HashMap<String, f64>,
    // ナラティブ・アイデンティティ (Narrative Identity)
    pub narrative_self: Option<String>,
}
