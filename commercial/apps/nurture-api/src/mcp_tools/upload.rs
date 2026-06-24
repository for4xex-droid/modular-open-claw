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

use crate::state::SharedState;
use chrono::Utc;
use commerce_protocol::commodity::{CommodityKind, ItemDescriptor, PriceTag};
use nurture_bridge::db::DatabasePool;
use sqlx::Row;

/// 種の名前のバリデーション (Guardrail Layer 0)
pub fn validate_species_name(name: &str) -> bool {
    let len = name.chars().count();
    if !(3..=32).contains(&len) {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
}
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use commerce_protocol::offer::SaleMode;
use nurture_infra::csam::ScanVerdict;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

/// GlassWorm(不可視文字を用いたプロンプトインジェクション)を防ぐための前処理
pub fn strip_invisible_unicode<'a>(input: &'a str) -> Cow<'a, str> {
    let has_invisible = input.chars().any(|c| {
        matches!(
            c,
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | // Zero-width spaces and BOM
            '\u{202A}'..='\u{202E}' | // BIDI Formatting
            '\u{2066}'..='\u{2069}' | // BIDI Isolate
            '\u{E0000}'..='\u{E007F}' // Tags block
        )
    });

    if !has_invisible {
        return Cow::Borrowed(input);
    }

    Cow::Owned(
        input
            .chars()
            .filter(|&c| {
                !matches!(
                    c,
                    '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' |
                    '\u{202A}'..='\u{202E}' |
                    '\u{2066}'..='\u{2069}' |
                    '\u{E0000}'..='\u{E007F}'
                )
            })
            .collect(),
    )
}

#[derive(Deserialize, Debug, Clone)]
pub struct UploadRequest {
    pub creator_id: Uuid,
    pub kind: String, // "VrmAvatar", etc
    pub name: String,
    pub description: String,
    pub price_coins: u64,
    pub content: String, // Stringified JSON or b64 for CSAM scan
    #[serde(default)]
    pub drm_enabled: bool,
    #[serde(default = "UploadRequest::default_sale_mode")]
    pub sale_mode: String,
    pub idempotency_key: String,
}

impl UploadRequest {
    fn default_sale_mode() -> String {
        "instant".to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UploadResponse {
    pub item_id: String,
}

pub async fn handle_upload(
    state: SharedState,
    payload: UploadRequest,
) -> Result<UploadResponse, NurtureError> {
    let store = state.idempotency.clone();
    if let Some(cached_res) = store.get_response(&payload.idempotency_key).await? {
        if let Some(res) = cached_res {
            return serde_json::from_str(&res.body).map_err(|e| {
                NurtureError::Infrastructure(format!("冪等性レスポンス復元失敗: {}", e))
            });
        } else {
            return Err(NurtureError::IdempotencyConflict {
                key: payload.idempotency_key,
            });
        }
    }

    store
        .reserve_key(&payload.idempotency_key, chrono::Duration::minutes(30))
        .await?;

    let item_uuid = Uuid::new_v4();

    let sanitized_name = strip_invisible_unicode(&payload.name).into_owned();
    let sanitized_description = strip_invisible_unicode(&payload.description).into_owned();

    if sanitized_name.trim().is_empty() {
        return Err(NurtureError::PolicyViolation(
            "Item name is required".to_string(),
        ));
    }
    if sanitized_description.trim().is_empty() {
        return Err(NurtureError::PolicyViolation(
            "Item description is required".to_string(),
        ));
    }

    let mut content_val = serde_json::from_str::<serde_json::Value>(&payload.content)
        .unwrap_or_else(|_| serde_json::json!({ "text": payload.content }));

    if let serde_json::Value::Object(ref mut map) = content_val {
        map.insert(
            "actor_id".to_string(),
            serde_json::json!(payload.creator_id.to_string()),
        );
        map.insert("kind".to_string(), serde_json::json!(payload.kind.clone()));
    } else {
        content_val = serde_json::json!({
            "content": content_val,
            "actor_id": payload.creator_id.to_string(),
            "kind": payload.kind.clone()
        });
    }

    match state.csam_pipeline.run_all(&item_uuid, &content_val).await {
        Ok(ScanVerdict::Safe) => {
            tracing::info!("✅ [Upload] CSAM scan approved for item {}", item_uuid);
        }
        Ok(ScanVerdict::Rejected { reason, layer, .. }) => {
            tracing::error!(
                "🚨 [Upload] CSAM violation at layer '{}': {}",
                layer,
                reason
            );
            return Err(NurtureError::CsamRejected {
                item_id: item_uuid,
                reason: format!("[{}] {}", layer, reason),
            });
        }
        Err(e) => {
            tracing::error!("❌ [Upload] CSAM scan error: {}", e);
            return Err(NurtureError::Infrastructure(format!(
                "CSAM scan error: {}",
                e
            )));
        }
    }

    let kind = match payload.kind.as_str() {
        "VrmAvatar" => CommodityKind::VrmAvatar,
        "ClothingPart" => CommodityKind::ClothingPart,
        "Accessory" => CommodityKind::Accessory,
        "WasmSkill" => CommodityKind::WasmSkill,
        "KnowledgePack" => CommodityKind::KnowledgePack,
        "Expression" => CommodityKind::Expression,
        "VoiceModel" => CommodityKind::VoiceModel,
        "KarmaPackage" => CommodityKind::KarmaPackage,
        "AutomationBlueprint" => CommodityKind::AutomationBlueprint,
        "LoraAdapter" => CommodityKind::LoraAdapter,
        "GeneticBlueprint" => CommodityKind::GeneticBlueprint,
        "BiomeEnvironment" => CommodityKind::BiomeEnvironment,
        unknown => {
            tracing::error!("❌ [Upload] Unknown CommodityKind '{}'", unknown);
            return Err(NurtureError::PolicyViolation(format!(
                "Unknown item kind: {}",
                unknown
            )));
        }
    };

    // GeneticBlueprint または BiomeEnvironment の場合の特別ルール (Pro 限定ゲート & 殿堂入り制限 & Guardrail Layer 0)
    if kind == CommodityKind::GeneticBlueprint || kind == CommodityKind::BiomeEnvironment {
        if !validate_species_name(&sanitized_name) {
            return Err(NurtureError::PolicyViolation(
                "Invalid species name: must be 3-32 characters and contain only alphanumeric characters, spaces, hyphens, or underscores".to_string()
            ));
        }

        let plan_id = match &state.pool {
            DatabasePool::Sqlite(p) => {
                let row_opt = sqlx::query(
                    "SELECT plan_id FROM nurture_subscriptions WHERE actor_id = ? AND status = 'active'",
                )
                .bind(payload.creator_id.to_string())
                .fetch_optional(p)
                .await
                .map_err(|e| NurtureError::Infrastructure(format!("サブスクリプション確認失敗: {}", e)))?;
                if let Some(row) = row_opt {
                    let pid: String = row.try_get("plan_id").map_err(|e| {
                        NurtureError::Infrastructure(format!("plan_id 取得失敗: {}", e))
                    })?;
                    Ok(Some(pid))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Postgres(p) => {
                let row_opt = sqlx::query(
                    "SELECT plan_id FROM nurture_subscriptions WHERE actor_id = $1 AND status = 'active'",
                )
                .bind(payload.creator_id.to_string())
                .fetch_optional(p)
                .await
                .map_err(|e| NurtureError::Infrastructure(format!("サブスクリプション確認失敗: {}", e)))?;
                if let Some(row) = row_opt {
                    let pid: String = row.try_get("plan_id").map_err(|e| {
                        NurtureError::Infrastructure(format!("plan_id 取得失敗: {}", e))
                    })?;
                    Ok(Some(pid))
                } else {
                    Ok(None)
                }
            }
        }?;

        let plan_id = match plan_id {
            Some(pid) => pid,
            None => {
                return Err(NurtureError::PolicyViolation(
                    "Pro membership is required to upload Biome assets".to_string(),
                ));
            }
        };

        let max_limit = match plan_id.as_str() {
            "si_pro" => 20,
            "fe_pro" => 5,
            _ => {
                return Err(NurtureError::PolicyViolation(
                    "Pro membership is required to upload Biome assets".to_string(),
                ));
            }
        };

        let count = match &state.pool {
            DatabasePool::Sqlite(p) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) as cnt FROM nurture_items WHERE creator_id = ? AND (kind = 'GeneticBlueprint' OR kind = 'BiomeEnvironment')"
                )
                .bind(payload.creator_id.to_string())
                .fetch_one(p)
                .await
                .map_err(|e| NurtureError::Infrastructure(format!("アップロード数カウント失敗: {}", e)))?;
                let count: i64 = row
                    .try_get("cnt")
                    .map_err(|e| NurtureError::Infrastructure(format!("cnt 取得失敗: {}", e)))?;
                Ok(count)
            }
            DatabasePool::Postgres(p) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) as cnt FROM nurture_items WHERE creator_id = $1 AND (kind = 'GeneticBlueprint' OR kind = 'BiomeEnvironment')"
                )
                .bind(payload.creator_id.to_string())
                .fetch_one(p)
                .await
                .map_err(|e| NurtureError::Infrastructure(format!("アップロード数カウント失敗: {}", e)))?;
                let count: i64 = row
                    .try_get("cnt")
                    .map_err(|e| NurtureError::Infrastructure(format!("cnt 取得失敗: {}", e)))?;
                Ok(count)
            }
        }?;

        if count >= max_limit {
            return Err(NurtureError::PolicyViolation(format!(
                "Upload limit reached for your membership plan (max {} items)",
                max_limit
            )));
        }
    }

    let parsed_sale_mode = if payload.sale_mode.starts_with("subscription:") {
        let parts: Vec<&str> = payload.sale_mode.split(':').collect();
        let days: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
        SaleMode::Subscription {
            interval_days: days,
            price_coins: payload.price_coins,
        }
    } else {
        SaleMode::Instant
    };

    let mut metadata_obj = serde_json::Map::new();

    if let serde_json::Value::Object(map) = &content_val {
        if let Some(val) = map.get("is_humanoid") {
            metadata_obj.insert("is_humanoid".to_string(), val.clone());
        }
    }

    if kind == CommodityKind::LoraAdapter {
        let content_obj = match &content_val {
            serde_json::Value::Object(map) => map,
            _ => {
                return Err(NurtureError::PolicyViolation(
                    "Content must be a JSON object for LoraAdapter".to_string(),
                ))
            }
        };

        let model_family = content_obj
            .get("model_family")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NurtureError::PolicyViolation(
                    "model_family is required for LoraAdapter".to_string(),
                )
            })?;
        let base_model = content_obj
            .get("base_model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NurtureError::PolicyViolation("base_model is required for LoraAdapter".to_string())
            })?;
        let adapter_path = content_obj
            .get("adapter_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NurtureError::PolicyViolation(
                    "adapter_path is required for LoraAdapter".to_string(),
                )
            })?;

        let clean_family = strip_invisible_unicode(model_family).into_owned();
        let clean_base = strip_invisible_unicode(base_model).into_owned();
        let clean_adapter = strip_invisible_unicode(adapter_path).into_owned();

        metadata_obj.insert("model_family".to_string(), serde_json::json!(clean_family));
        metadata_obj.insert("base_model".to_string(), serde_json::json!(clean_base));
        metadata_obj.insert("adapter_path".to_string(), serde_json::json!(clean_adapter));
    }

    let item = ItemDescriptor {
        id: item_uuid,
        kind,
        name: sanitized_name,
        description: sanitized_description,
        price: PriceTag::Fixed(payload.price_coins),
        creator_id: ActorId(payload.creator_id),
        sale_mode: parsed_sale_mode,
        drm_enabled: payload.drm_enabled,
        created_at: Utc::now(),
        metadata: serde_json::Value::Object(metadata_obj),
        content_hash: None,
    };

    let data = payload.content.as_bytes();
    if let Err(e) = state
        .asset_storage
        .put_asset(&ActorId(payload.creator_id), &item_uuid, data)
        .await
    {
        tracing::error!("❌ [Upload] Failed to store asset: {}", e);
        return Err(NurtureError::Infrastructure(format!(
            "Storage error: {}",
            e
        )));
    }

    match state.marketplace.create_item(&item).await {
        Ok(_) => {
            let resp = UploadResponse {
                item_id: item_uuid.to_string(),
            };
            let resp_json = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());

            if let Err(e) = state
                .idempotency
                .save_response(&payload.idempotency_key, 201, resp_json)
                .await
            {
                tracing::error!("⚠️ [Upload] Failed to save idempotency response: {}", e);
            }

            Ok(resp)
        }
        Err(e) => {
            tracing::error!("❌ [Upload] Failed to create item in marketplace: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commerce_protocol::identity::ActorId;
    use nurture_bridge::auth::MockAuthManager;
    use nurture_bridge::db::DatabasePool;
    use nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore;
    use nurture_bridge::job_queue::UniversalJobQueue;
    use nurture_bridge::traits::JobQueue;
    use nurture_infra::storage::MockAssetStorage;
    use sqlx::SqlitePool;
    use std::sync::Arc;

    async fn setup_state() -> SharedState {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations/sqlite")
            .run(&pool)
            .await
            .unwrap();
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let db_pool = DatabasePool::Sqlite(pool);
        let store = Arc::new(SqliteTrajectoryStore::new(db_pool.clone()));
        let job_queue: Arc<dyn JobQueue> =
            Arc::new(UniversalJobQueue::from_pool(db_pool.clone(), store));

        crate::state::AppState::init(
            db_pool,
            job_queue,
            nurture_core::policy::EconomyPolicy::default(),
            ActorId(Uuid::new_v4()),
            cancel_token,
            "test".to_string().into(),
            None,
            None,
            Arc::new(MockAuthManager::new()),
            "key".to_string().into(),
            Arc::new(MockAssetStorage::new()),
            None,
            "localhost".to_string(),
            "50051".to_string(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_upload_sanitizes_invisible_unicode() {
        let state = setup_state().await;
        let creator_id = Uuid::new_v4();
        sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
            .bind(creator_id.to_string())
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        let req = UploadRequest {
            creator_id,
            kind: "KnowledgePack".to_string(),
            name: "Secret\u{200B} Gift".to_string(),
            description: "A very\u{FEFF} secret gift".to_string(),
            price_coins: 100,
            content: "{}".to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: Uuid::new_v4().to_string(),
        };

        let res = handle_upload(state.clone(), req).await.unwrap();
        let item = state
            .marketplace
            .get_item(&Uuid::parse_str(&res.item_id).unwrap())
            .await
            .unwrap();

        // Assert that the invisible unicode has been stripped
        assert_eq!(item.name, "Secret Gift");
        assert_eq!(item.description, "A very secret gift");
    }

    #[tokio::test]
    async fn test_upload_lora_requires_metadata() {
        let state = setup_state().await;
        let creator_id = Uuid::new_v4();
        sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
            .bind(creator_id.to_string())
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        // Missing model_family
        let req = UploadRequest {
            creator_id,
            kind: "LoraAdapter".to_string(),
            name: "Test LoRA".to_string(),
            description: "LoRA description".to_string(),
            price_coins: 150,
            content: serde_json::json!({
                "base_model": "stable-diffusion-v1-5",
                "adapter_path": "/path/to/lora.safetensors"
            })
            .to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: Uuid::new_v4().to_string(),
        };

        let res = handle_upload(state.clone(), req).await;
        assert!(res.is_err());
        match res.err().unwrap() {
            NurtureError::PolicyViolation(msg) => {
                assert!(msg.contains("model_family is required"));
            }
            other => panic!("Expected PolicyViolation, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_upload_lora_happy_path() {
        let state = setup_state().await;
        let creator_id = Uuid::new_v4();
        sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
            .bind(creator_id.to_string())
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        let req = UploadRequest {
            creator_id,
            kind: "LoraAdapter".to_string(),
            name: "Test\u{200C} LoRA".to_string(),
            description: "LoRA\u{200D} description".to_string(),
            price_coins: 200,
            content: serde_json::json!({
                "model_family": "SD\u{FEFF}1.5",
                "base_model": "stable-diffusion-v1-5",
                "adapter_path": "/path/to/lora.safetensors"
            })
            .to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: Uuid::new_v4().to_string(),
        };

        let res = handle_upload(state.clone(), req).await.unwrap();
        let item = state
            .marketplace
            .get_item(&Uuid::parse_str(&res.item_id).unwrap())
            .await
            .unwrap();

        assert_eq!(item.name, "Test LoRA");
        assert_eq!(item.description, "LoRA description");
        assert_eq!(
            item.metadata.get("model_family").unwrap().as_str().unwrap(),
            "SD1.5"
        );
        assert_eq!(
            item.metadata.get("base_model").unwrap().as_str().unwrap(),
            "stable-diffusion-v1-5"
        );
        assert_eq!(
            item.metadata.get("adapter_path").unwrap().as_str().unwrap(),
            "/path/to/lora.safetensors"
        );
    }

    #[tokio::test]
    async fn test_upload_biome_asset_requires_pro_subscription() {
        let state = setup_state().await;
        let creator_id = Uuid::new_v4();
        sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
            .bind(creator_id.to_string())
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        // サブスクリプションがない状態で GeneticBlueprint をアップロード
        let req = UploadRequest {
            creator_id,
            kind: "GeneticBlueprint".to_string(),
            name: "Species-Alpha".to_string(),
            description: "A genetic blueprint".to_string(),
            price_coins: 100,
            content: "{}".to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: Uuid::new_v4().to_string(),
        };

        let res = handle_upload(state.clone(), req).await;
        assert!(res.is_err());
        match res.err().unwrap() {
            NurtureError::PolicyViolation(msg) => {
                assert!(msg.contains("Pro membership is required"));
            }
            other => panic!("Expected PolicyViolation, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_upload_biome_asset_invalid_name_guardrail() {
        let state = setup_state().await;
        let creator_id = Uuid::new_v4();
        sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
            .bind(creator_id.to_string())
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        // 有効な fe_pro サブスクリプションを追加
        sqlx::query("INSERT INTO nurture_subscriptions (id, actor_id, stripe_subscription_id, plan_id, status, current_period_end) VALUES (?, ?, ?, 'fe_pro', 'active', ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(creator_id.to_string())
            .bind("sub_123")
            .bind(Utc::now() + chrono::Duration::days(30))
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        // 不正な名前（短すぎる）でアップロード
        let req = UploadRequest {
            creator_id,
            kind: "GeneticBlueprint".to_string(),
            name: "Al".to_string(), // 2文字
            description: "A genetic blueprint".to_string(),
            price_coins: 100,
            content: "{}".to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: Uuid::new_v4().to_string(),
        };

        let res = handle_upload(state.clone(), req).await;
        assert!(res.is_err());
        match res.err().unwrap() {
            NurtureError::PolicyViolation(msg) => {
                assert!(msg.contains("Invalid species name"));
            }
            other => panic!("Expected PolicyViolation, got: {:?}", other),
        }

        // 不正な名前（特殊文字）でアップロード
        let req_invalid_chars = UploadRequest {
            creator_id,
            kind: "GeneticBlueprint".to_string(),
            name: "Species<script>".to_string(),
            description: "A genetic blueprint".to_string(),
            price_coins: 100,
            content: "{}".to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: Uuid::new_v4().to_string(),
        };

        let res = handle_upload(state.clone(), req_invalid_chars).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_upload_biome_asset_exceeds_limit() {
        let state = setup_state().await;
        let creator_id = Uuid::new_v4();
        sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
            .bind(creator_id.to_string())
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        // 有効な fe_pro サブスクリプションを追加 (上限5件)
        sqlx::query("INSERT INTO nurture_subscriptions (id, actor_id, stripe_subscription_id, plan_id, status, current_period_end) VALUES (?, ?, ?, 'fe_pro', 'active', ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(creator_id.to_string())
            .bind("sub_123")
            .bind(Utc::now() + chrono::Duration::days(30))
            .execute(state.pool.get_sqlite_pool().unwrap())
            .await
            .unwrap();

        // 5件アップロードする
        for i in 0..5 {
            let req = UploadRequest {
                creator_id,
                kind: "GeneticBlueprint".to_string(),
                name: format!("Species-{}", i),
                description: "A genetic blueprint".to_string(),
                price_coins: 100,
                content: "{}".to_string(),
                drm_enabled: false,
                sale_mode: "instant".to_string(),
                idempotency_key: format!("key-{}", i),
            };
            handle_upload(state.clone(), req).await.unwrap();
        }

        // 6件目は失敗するはず
        let req_exceed = UploadRequest {
            creator_id,
            kind: "GeneticBlueprint".to_string(),
            name: "Species-Exceed".to_string(),
            description: "A genetic blueprint".to_string(),
            price_coins: 100,
            content: "{}".to_string(),
            drm_enabled: false,
            sale_mode: "instant".to_string(),
            idempotency_key: "key-exceed".to_string(),
        };

        let res = handle_upload(state.clone(), req_exceed).await;
        assert!(res.is_err());
        match res.err().unwrap() {
            NurtureError::PolicyViolation(msg) => {
                assert!(msg.contains("Upload limit reached"));
            }
            other => panic!("Expected PolicyViolation, got: {:?}", other),
        }
    }
}
