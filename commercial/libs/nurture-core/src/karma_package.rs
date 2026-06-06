/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use chrono::{DateTime, Utc};
use commerce_protocol::identity::ActorId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KarmaPackage {
    pub id: Uuid,
    pub creator_id: ActorId,
    pub specialization: String,
    pub karma_count: u64,
    pub avg_weight: f64,
    pub domains: Vec<String>,
    pub price_coins: u64,
    pub drm_enabled: bool,
    pub created_at: DateTime<Utc>,
}
