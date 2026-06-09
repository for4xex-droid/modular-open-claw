/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use crate::identity::ActorId;
use crate::offer::SaleMode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommodityKind {
    VrmAvatar,
    ClothingPart,
    Accessory,
    WasmSkill,
    KnowledgePack,
    Expression,
    VoiceModel,
    KarmaPackage,
    AutomationBlueprint,
    LoraAdapter,
    GeneticBlueprint,
    BiomeEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDescriptor {
    pub id: Uuid,
    pub kind: CommodityKind,
    pub name: String,
    pub description: String,
    pub price: PriceTag,
    pub creator_id: ActorId,
    pub sale_mode: SaleMode,
    pub drm_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceTag {
    Fixed(u64),
    Negotiable { min: u64, max: u64 },
    Free,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_adapter_serialization() {
        // この時点では LoraAdapter が定義されていないため、コンパイルエラー (RED) になります
        let kind = CommodityKind::LoraAdapter;
        let serialized = serde_json::to_string(&kind).unwrap();
        assert_eq!(serialized, "\"LoraAdapter\"");

        let deserialized: CommodityKind = serde_json::from_str("\"LoraAdapter\"").unwrap();
        assert_eq!(deserialized, CommodityKind::LoraAdapter);
    }

    #[test]
    fn test_item_descriptor_content_hash() {
        // この時点では ItemDescriptor に content_hash フィールドが存在しないため、コンパイルエラー (RED) になります
        let item = ItemDescriptor {
            id: Uuid::new_v4(),
            kind: CommodityKind::LoraAdapter,
            name: "Test LoRA".to_string(),
            description: "Test description".to_string(),
            price: PriceTag::Fixed(100),
            creator_id: ActorId(Uuid::new_v4()),
            sale_mode: SaleMode::Instant,
            drm_enabled: false,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
            content_hash: Some(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            ),
        };
        assert_eq!(
            item.content_hash.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }
}
