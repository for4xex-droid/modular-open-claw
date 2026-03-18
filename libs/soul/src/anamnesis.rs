use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnamnesisProfile {
    // 早期不適応スキーマ、ナラティブ・アイデンティティなどが入る
    // Phase 2 or later implementation.
}

impl Default for AnamnesisProfile {
    fn default() -> Self {
        Self {}
    }
}
