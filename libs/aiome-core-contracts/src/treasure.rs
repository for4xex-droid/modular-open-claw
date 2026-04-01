/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 宝箱（レコメンド）のアイテム
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TreasureItem {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub url: String,
    pub price_coins: Option<u64>,
    pub category: String,
    pub score: f32,               // Somatic/Resonance score
    pub disclosure_label: String, // ステマ規制対応ラベル (e.g., "AI推薦 / 広告")
}

/// 宝箱フィードバック
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TreasureFeedback {
    pub item_id: Uuid,
    pub action: String, // "view", "click", "buy"
    pub metadata: Option<serde_json::Value>,
}
