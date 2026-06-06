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
pub struct PromotionCriteria {
    pub min_karma_count: u64,
    pub min_avg_weight: f64,
    pub required_specialization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotedClone {
    pub id: Uuid,
    pub parent_actor_id: ActorId,
    pub promoted_actor_id: ActorId,
    pub specialization: String,
    pub karma_count: u64,
    pub promoted_at: DateTime<Utc>,
}

// 実際の昇格ロジックは CloneManager 内部または AppService で実装するが
// ここではデータ構造を定義
