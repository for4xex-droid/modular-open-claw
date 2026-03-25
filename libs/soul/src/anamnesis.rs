/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnamnesisProfile {
    // 早期不適応スキーマ (Early Maladaptive Schemas)
    pub core_schemas: std::collections::HashMap<String, f64>,
    // ナラティブ・アイデンティティ (Narrative Identity)
    pub narrative_self: Option<String>,
}
