/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::SqlitePool;

/// アセット種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    /// TTS向けボイスモデル
    VoiceModel,
    /// LoRA等の追加学習モデル
    LoRA,
    /// Inochi2Dアバターモデル
    Inochi2D,
    /// その他のスキル・プラグイン
    Plugin,
}

impl AsRef<str> for AssetType {
    fn as_ref(&self) -> &str {
        match self {
            AssetType::VoiceModel => "voice",
            AssetType::LoRA => "lora",
            AssetType::Inochi2D => "inochi2d",
            AssetType::Plugin => "plugin",
        }
    }
}

/// Registry 用の Asset メタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// 一意のアセットID
    pub id: Uuid,
    /// 作成者のID
    pub creator_id: Uuid,
    /// アセットの種別
    pub asset_type: AssetType,
    /// アセット名
    pub name: String,
    /// アセットの詳細説明
    pub description: String,
    /// 価格（コイン換算）
    pub price_coins: u64,
}

/// Phase 10: Registry
///
/// クリエイターがアップロードしたアセット（ボイス、LoRA等）のインデックスと
/// マニフェスト（SkillManifestの上位層）を管理するシステム。
pub struct RegistryManager {
    pool: SqlitePool,
}

impl RegistryManager {
    /// RegistryManager の初期化
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// アセットのメタデータを登録する
    pub async fn register_asset(&self, manifest: AssetManifest) -> Result<(), AiomeError> {
        let type_str = manifest.asset_type.as_ref();
        
        let result = sqlx::query(
            r#"
            INSERT INTO asset_registry (id, creator_id, asset_type, name, description, price_coins)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(manifest.id.to_string())
        .bind(manifest.creator_id.to_string())
        .bind(type_str)
        .bind(&manifest.name)
        .bind(&manifest.description)
        .bind(manifest.price_coins as i64)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                tracing::info!("📦 [Registry] Successfully registered asset: {}", manifest.id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ [Registry] Failed to register asset: {}", e);
                Err(AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }

    /// アセットのメタデータを取得する
    pub async fn get_asset(&self, asset_id: Uuid) -> Result<AssetManifest, AiomeError> {
        let result = sqlx::query_as::<_, (String, String, String, String, String, i64)>(
            r#"
            SELECT id, creator_id, asset_type, name, description, price_coins
            FROM asset_registry
            WHERE id = ?
            "#
        )
        .bind(asset_id.to_string())
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(Some(row)) => {
                let asset_type = match row.2.as_str() {
                    "voice" => AssetType::VoiceModel,
                    "lora" => AssetType::LoRA,
                    "inochi2d" => AssetType::Inochi2D,
                    _ => AssetType::Plugin,
                };

                Ok(AssetManifest {
                    id: Uuid::parse_str(&row.0).unwrap_or(asset_id),
                    creator_id: Uuid::parse_str(&row.1).unwrap_or_default(),
                    asset_type,
                    name: row.3,
                    description: row.4,
                    price_coins: row.5 as u64,
                })
            }
            Ok(None) => Err(AiomeError::ArtifactNotFound {
                path: format!("Asset {}", asset_id),
            }),
            Err(e) => Err(AiomeError::Infrastructure { reason: e.to_string() }),
        }
    }

    /// 指定した種別のアセット一覧を取得する (scope: "public" | "owned")
    pub async fn list_assets_by_type(&self, asset_type: AssetType, agent_id: Option<Uuid>, scope: &str) -> Result<Vec<AssetManifest>, AiomeError> {
        let type_str = asset_type.as_ref();
        
        let rows_result = if scope == "owned" {
            if let Some(agent) = agent_id {
                sqlx::query_as::<_, (String, String, String, String, String, i64)>(
                    r#"
                    SELECT DISTINCT a.id, a.creator_id, a.asset_type, a.name, a.description, a.price_coins
                    FROM asset_registry a
                    LEFT JOIN licenses l ON a.id = l.asset_id AND l.agent_id = ? AND l.status = 'active'
                    WHERE a.asset_type = ? AND (a.creator_id = ? OR l.id IS NOT NULL)
                    "#
                )
                .bind(agent.to_string())
                .bind(type_str)
                .bind(agent.to_string())
                .fetch_all(&self.pool)
                .await
            } else {
                return Err(AiomeError::Infrastructure { reason: "agent_id is required for owned scope".into() });
            }
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, i64)>(
                r#"
                SELECT id, creator_id, asset_type, name, description, price_coins
                FROM asset_registry
                WHERE asset_type = ?
                "#
            )
            .bind(type_str)
            .fetch_all(&self.pool)
            .await
        };

        match rows_result {
            Ok(rows) => {
                let assets = rows.into_iter().map(|row| {
                    AssetManifest {
                        id: Uuid::parse_str(&row.0).unwrap_or_default(),
                        creator_id: Uuid::parse_str(&row.1).unwrap_or_default(),
                        asset_type: asset_type.clone(),
                        name: row.3,
                        description: row.4,
                        price_coins: row.5 as u64,
                    }
                }).collect();
                Ok(assets)
            }
            Err(e) => Err(AiomeError::Infrastructure { reason: e.to_string() }),
        }
    }

    /// エージェントがアセットを所有しているか確認する
    pub async fn check_ownership(&self, agent_id: Uuid, asset_id: Uuid) -> Result<bool, AiomeError> {
        // 0. Creator は無条件で所有 (§LDR-3)
        let is_creator: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM asset_registry WHERE id = ? AND creator_id = ?"
        )
        .bind(asset_id.to_string())
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,));

        if is_creator.0 > 0 {
            return Ok(true);
        }

        // Phase 11: Dual-read (licenses優先、フォールバック: stripe_webhook_events)
        
        // 1. licenses テーブルを先にチェック
        let license_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM licenses WHERE agent_id = ? AND asset_id = ? AND status = 'active'"
        )
        .bind(agent_id.to_string())
        .bind(asset_id.to_string())
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,)); // テーブルが存在しない場合（マイグレーション前）はフォールバックへ

        if license_count.0 > 0 {
            return Ok(true);
        }

        // 2. フォールバック: stripe_webhook_events (移行期間中のみ)
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM stripe_webhook_events 
            WHERE event_type = 'checkout.session.completed' 
            AND json_extract(metadata, '$.agent_id') = ? 
            AND json_extract(metadata, '$.asset_id') = ?
            "#
        )
        .bind(agent_id.to_string())
        .bind(asset_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(count.0 > 0)
    }

    /// デジタルアセットのライセンスを付与する
    pub async fn grant_license(&self, agent_id: Uuid, asset_id: Uuid, original_event_id: String) -> Result<(), AiomeError> {
        let result = sqlx::query(
            r#"
            INSERT INTO licenses (id, agent_id, asset_id, original_event_id, status)
            VALUES (?, ?, ?, ?, 'active')
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(agent_id.to_string())
        .bind(asset_id.to_string())
        .bind(original_event_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                tracing::info!("🎫 [Registry] Granted license to agent {} for asset {}", agent_id, asset_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ [Registry] Failed to grant license: {}", e);
                Err(AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }

    /// デジタルアセットのライセンスをトランザクション内で付与する (Phase 11: 単一トランザクション同期)
    pub async fn grant_license_with_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        agent_id: Uuid,
        asset_id: Uuid,
        original_event_id: String,
    ) -> Result<(), AiomeError> {
        let result = sqlx::query(
            r#"
            INSERT INTO licenses (id, agent_id, asset_id, original_event_id, status)
            VALUES (?, ?, ?, ?, 'active')
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(agent_id.to_string())
        .bind(asset_id.to_string())
        .bind(original_event_id)
        .execute(&mut **tx)
        .await;

        match result {
            Ok(_) => {
                tracing::info!("🎫 [Registry] Granted license to agent {} for asset {} (tx)", agent_id, asset_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ [Registry] Failed to grant license (tx): {}", e);
                Err(AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }
}

#[cfg(test)]
impl RegistryManager {
    /// テスト用に Pool を取得する
    pub fn get_pool_for_test(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db_for_registry() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE asset_registry (
                id TEXT PRIMARY KEY,
                creator_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                price_coins INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE stripe_webhook_events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                metadata TEXT,
                processed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE licenses (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                original_event_id TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_registry_register_and_get_asset() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        
        let asset_id = Uuid::new_v4();
        let manifest = AssetManifest {
            id: asset_id,
            creator_id: Uuid::new_v4(),
            asset_type: AssetType::VoiceModel,
            name: "Premium Voice".into(),
            description: "High quality voice model".into(),
            price_coins: 500,
        };

        registry.register_asset(manifest).await.unwrap();
        let fetched = registry.get_asset(asset_id).await.unwrap();
        assert_eq!(fetched.name, "Premium Voice");
        assert_eq!(fetched.price_coins, 500);
    }

    #[tokio::test]
    async fn test_registry_check_ownership() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        let agent_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        // 購入前
        assert!(!registry.check_ownership(agent_id, asset_id).await.unwrap());

        // ダミーWebhookイベントの挿入 (所有権の記録)
        sqlx::query(
            "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)"
        )
        .bind("evt_test_ownership")
        .bind("checkout.session.completed")
        .bind(format!(r#"{{"agent_id": "{}", "asset_id": "{}"}}"#, agent_id, asset_id))
        .execute(&registry.pool)
        .await
        .unwrap();

        // 購入後
        assert!(registry.check_ownership(agent_id, asset_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_registry_check_ownership_false_positive_prevention() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        
        let target_agent = Uuid::new_v4();
        let target_asset = Uuid::new_v4();
        
        // ターゲットに酷似したID（一部が共通する文字列など）を挿入
        // 実際には UUID なので確率は低いが、LIKE 検索の脆弱性をエミュレート
        let sibling_agent = target_agent.to_string();
        let partial_agent = &sibling_agent[..8]; // 先頭部分のみ
        
        sqlx::query(
            "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)"
        )
        .bind("evt_false_positive")
        .bind("checkout.session.completed")
        .bind(format!(r#"{{"agent_id": "{}", "asset_id": "{}"}}"#, partial_agent, target_asset))
        .execute(&registry.pool)
        .await
        .unwrap();

        // LIKE %partial% だと target_agent (完全なUUID) にマッチしてしまう可能性がある
        // 実際には UUID 同士だが、文字列として検索しているため危険。
        // 現在の実装が LIKE %agent_id% なので、DB内の partial_agent に %target_agent% はマッチしないが、
        // 逆（DBに長いIDがあり、短いIDで検索）だとマッチする。
        
        // 正しい検証: 
        // 検索値: "123"
        // DB値: "12345"
        // LIKE %123% は true になる。これがバイパスのリスク。
        
        let short_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let long_id_contained = "00000000-0000-0000-0000-000000000001-suffix"; // 非UUIDだがDBには入る
        
        sqlx::query(
            "INSERT INTO stripe_webhook_events (event_id, event_type, metadata) VALUES (?, ?, ?)"
        )
        .bind("evt_bypass")
        .bind("checkout.session.completed")
        .bind(format!(r#"{{"agent_id": "{}", "asset_id": "{}"}}"#, long_id_contained, target_asset))
        .execute(&registry.pool)
        .await
        .unwrap();
        
        // short_id で検索すると、long_id_contained に LIKE でマッチしてしまう
        let result = registry.check_ownership(short_id, target_asset).await.unwrap();
        
        // 本来は false であるべき（IDが完全一致していないため）
        // 現状の実装 (LIKE) では true になってしまうはず = RED
        assert!(!result, "LIKE search should not match partial IDs (Bypass Risk)");
    }

    #[tokio::test]
    async fn test_registry_grant_license_and_check_ownership() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        let agent_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        // 購入前
        assert!(!registry.check_ownership(agent_id, asset_id).await.unwrap());

        // 新しいメソッド: ライセンスの付与 (まだ未実装なのでコンパイルエラーになるかパニックするはず)
        registry.grant_license(agent_id, asset_id, "evt_test_grant".to_string()).await.unwrap();

        // 購入後
        assert!(registry.check_ownership(agent_id, asset_id).await.unwrap());
    }
}

