/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::db::{DatabasePool, DatabaseTransaction};
use aiome_core::error::AiomeError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// MCP サーバー (stdio/sse)
    McpServer,
}

impl AsRef<str> for AssetType {
    fn as_ref(&self) -> &str {
        match self {
            AssetType::VoiceModel => "voice",
            AssetType::LoRA => "lora",
            AssetType::Inochi2D => "inochi2d",
            AssetType::Plugin => "plugin",
            AssetType::McpServer => "mcp",
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
    /// 追加のメタデータ (JSON) — MCP 構成等
    pub metadata: Option<serde_json::Value>,
}

/// Phase 10: Registry
///
/// クリエイターがアップロードしたアセット（ボイス、LoRA等）のインデックスと
/// マニフェスト（SkillManifestの上位層）を管理するシステム。
pub struct RegistryManager {
    pool: DatabasePool,
}

impl RegistryManager {
    /// RegistryManager の初期化
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    /// アセットのメタデータを登録する
    pub async fn register_asset(&self, manifest: AssetManifest) -> Result<(), AiomeError> {
        let type_str = manifest.asset_type.as_ref();

        let q = format!(
            "INSERT INTO asset_registry (id, creator_id, asset_type, name, description, price_coins, metadata) VALUES ({}, {}, {}, {}, {}, {}, {})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6)
        );

        crate::sql_exec!(
            &self.pool,
            &q,
            manifest.id.to_string(),
            manifest.creator_id.to_string(),
            type_str,
            &manifest.name,
            &manifest.description,
            manifest.price_coins as i64,
            manifest.metadata.map(|m| m.to_string())
        )?;

        tracing::info!(
            "📦 [Registry] Successfully registered asset: {}",
            manifest.id
        );
        Ok(())
    }

    /// アセットのメタデータを取得する
    pub async fn get_asset(&self, asset_id: Uuid) -> Result<AssetManifest, AiomeError> {
        let q = format!(
            "SELECT id, creator_id, asset_type, name, description, price_coins, metadata FROM asset_registry WHERE id = {}",
            self.pool.ph(0)
        );

        let row: (String, String, String, String, String, i64, Option<String>) = crate::sql_fetch_one!(
            &self.pool,
            (String, String, String, String, String, i64, Option<String>),
            &q,
            asset_id.to_string()
        )?;

        let asset_type = match row.2.as_str() {
            "voice" => AssetType::VoiceModel,
            "lora" => AssetType::LoRA,
            "inochi2d" => AssetType::Inochi2D,
            "mcp" => AssetType::McpServer,
            _ => AssetType::Plugin,
        };

        Ok(AssetManifest {
            id: Uuid::parse_str(&row.0).unwrap_or(asset_id),
            creator_id: Uuid::parse_str(&row.1).unwrap_or_default(),
            asset_type,
            name: row.3,
            description: row.4,
            price_coins: row.5 as u64,
            metadata: row.6.and_then(|m| serde_json::from_str(&m).ok()),
        })
    }

    /// 指定した種別のアセット一覧を取得する (scope: "public" | "owned")
    pub async fn list_assets_by_type(
        &self,
        asset_type: AssetType,
        agent_id: Option<Uuid>,
        scope: &str,
    ) -> Result<Vec<AssetManifest>, AiomeError> {
        let type_str = asset_type.as_ref();

        let rows: Vec<(String, String, String, String, String, i64, Option<String>)> = if scope
            == "owned"
        {
            if let Some(agent) = agent_id {
                let q = format!(
                    r#"
                    SELECT DISTINCT a.id, a.creator_id, a.asset_type, a.name, a.description, a.price_coins, a.metadata
                    FROM asset_registry a
                    LEFT JOIN licenses l ON a.id = l.asset_id AND l.agent_id = {0} AND l.status = 'active'
                    WHERE a.asset_type = {1} AND (a.creator_id = {2} OR l.id IS NOT NULL)
                    "#,
                    self.pool.ph(0),
                    self.pool.ph(1),
                    self.pool.ph(2)
                );
                crate::sql_fetch_all!(
                    &self.pool,
                    (String, String, String, String, String, i64, Option<String>),
                    &q,
                    agent.to_string(),
                    type_str,
                    agent.to_string()
                )?
            } else {
                return Err(AiomeError::Infrastructure {
                    reason: "agent_id is required for owned scope".into(),
                });
            }
        } else {
            let q = format!(
                "SELECT id, creator_id, asset_type, name, description, price_coins, metadata FROM asset_registry WHERE asset_type = {}",
                self.pool.ph(0)
            );
            crate::sql_fetch_all!(
                &self.pool,
                (String, String, String, String, String, i64, Option<String>),
                &q,
                type_str
            )?
        };

        let assets = rows
            .into_iter()
            .map(|row| AssetManifest {
                id: Uuid::parse_str(&row.0).unwrap_or_default(),
                creator_id: Uuid::parse_str(&row.1).unwrap_or_default(),
                asset_type: asset_type.clone(),
                name: row.3,
                description: row.4,
                price_coins: row.5 as u64,
                metadata: row.6.and_then(|m| serde_json::from_str(&m).ok()),
            })
            .collect();
        Ok(assets)
    }

    /// エージェントがアセットを所有しているか確認する
    pub async fn check_ownership(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
    ) -> Result<bool, AiomeError> {
        let q_creator = format!(
            "SELECT COUNT(*) FROM asset_registry WHERE id = {0} AND creator_id = {1}",
            self.pool.ph(0),
            self.pool.ph(1)
        );

        let is_creator: (i64,) = crate::sql_fetch_one!(
            &self.pool,
            (i64,),
            &q_creator,
            asset_id.to_string(),
            agent_id.to_string()
        )
        .unwrap_or((0,));

        if is_creator.0 > 0 {
            return Ok(true);
        }

        let q_license = format!(
            "SELECT COUNT(*) FROM licenses WHERE agent_id = {0} AND asset_id = {1} AND status = 'active'",
            self.pool.ph(0), self.pool.ph(1)
        );

        let license_count: (i64,) = crate::sql_fetch_one!(
            &self.pool,
            (i64,),
            &q_license,
            agent_id.to_string(),
            asset_id.to_string()
        )
        .unwrap_or((0,));

        if license_count.0 > 0 {
            return Ok(true);
        }

        Ok(false)
    }

    /// デジタルアセットのライセンスを付与する
    pub async fn grant_license(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
        original_event_id: String,
    ) -> Result<(), AiomeError> {
        let q = format!(
            "INSERT INTO licenses (id, agent_id, asset_id, original_event_id, status) VALUES ({}, {}, {}, {}, 'active')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );

        crate::sql_exec!(
            &self.pool,
            &q,
            Uuid::new_v4().to_string(),
            agent_id.to_string(),
            asset_id.to_string(),
            original_event_id
        )?;

        tracing::info!(
            "🎫 [Registry] Granted license to agent {} for asset {}",
            agent_id,
            asset_id
        );
        Ok(())
    }

    /// デジタルアセットのライセンスをトランザクション内で付与する (Phase 11: 単一トランザクション同期)
    pub async fn grant_license_with_tx(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        agent_id: Uuid,
        asset_id: Uuid,
        original_event_id: String,
    ) -> Result<(), AiomeError> {
        let q = format!(
            "INSERT INTO licenses (id, agent_id, asset_id, original_event_id, status) VALUES ({}, {}, {}, {}, 'active')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );

        match tx {
            DatabaseTransaction::Sqlite(itx) => sqlx::query(&q)
                .bind(Uuid::new_v4().to_string())
                .bind(agent_id.to_string())
                .bind(asset_id.to_string())
                .bind(original_event_id)
                .execute(&mut **itx)
                .await
                .map(|_| ())
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            DatabaseTransaction::Postgres(itx) => sqlx::query(&q)
                .bind(Uuid::new_v4().to_string())
                .bind(agent_id.to_string())
                .bind(asset_id.to_string())
                .bind(original_event_id)
                .execute(&mut **itx)
                .await
                .map(|_| ())
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };

        tracing::info!(
            "🎫 [Registry] Granted license to agent {} for asset {} (tx)",
            agent_id,
            asset_id
        );
        Ok(())
    }
}

/// MCP サーバー管理用の拡張
impl RegistryManager {
    /// MCP サーバーを登録する (便利メソッド)
    pub async fn register_mcp_server(
        &self,
        creator_id: Uuid,
        name: &str,
        description: &str,
        config: serde_json::Value,
    ) -> Result<Uuid, AiomeError> {
        let asset_id = Uuid::new_v4();
        let manifest = AssetManifest {
            id: asset_id,
            creator_id,
            asset_type: AssetType::McpServer,
            name: name.to_string(),
            description: description.to_string(),
            price_coins: 0,
            metadata: Some(config),
        };

        self.register_asset(manifest).await?;
        Ok(asset_id)
    }

    /// 登録済みの MCP サーバー一覧を取得する
    pub async fn list_mcp_servers(&self) -> Result<Vec<AssetManifest>, AiomeError> {
        self.list_assets_by_type(AssetType::McpServer, None, "public")
            .await
    }

    /// (P-8) 登録済みの MCP サーバーエントリーを全てクリアする（ゴーストエントリ防止用）
    pub async fn clear_mcp_servers(&self) -> Result<(), AiomeError> {
        let q = format!(
            "DELETE FROM asset_registry WHERE asset_type = {}",
            self.pool.ph(0)
        );
        crate::sql_exec!(&self.pool, &q, AssetType::McpServer.as_ref())?;

        tracing::info!("🧹 [Registry] Cleared all previously registered MCP servers.");
        Ok(())
    }
}

#[cfg(test)]
impl RegistryManager {
    /// テスト用に Pool を取得する
    pub fn get_pool_for_test(&self) -> &sqlx::SqlitePool {
        self.pool.get_sqlite_pool_or_err().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db_for_registry() -> DatabasePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db_pool = DatabasePool::Sqlite(pool);

        crate::sql_exec!(
            &db_pool,
            r#"
            CREATE TABLE asset_registry (
                id TEXT PRIMARY KEY,
                creator_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                price_coins INTEGER NOT NULL DEFAULT 0,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .unwrap();

        crate::sql_exec!(
            &db_pool,
            r#"
            CREATE TABLE stripe_webhook_events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                metadata TEXT,
                processed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .unwrap();

        crate::sql_exec!(
            &db_pool,
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
        .unwrap();

        db_pool
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
            metadata: None,
        };

        registry.register_asset(manifest).await.unwrap();
        let fetched = registry.get_asset(asset_id).await.unwrap();
        assert_eq!(fetched.name, "Premium Voice");
        assert_eq!(fetched.price_coins, 500);
    }

    #[tokio::test]
    async fn test_registry_register_mcp_server() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);

        let asset_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-everything"],
            "env": {
                "DEBUG": "true"
            }
        });

        let manifest = AssetManifest {
            id: asset_id,
            creator_id: Uuid::new_v4(),
            asset_type: AssetType::McpServer,
            name: "Everything Server".into(),
            description: "A test MCP server".into(),
            price_coins: 0,
            metadata: Some(metadata.clone()),
        };

        // 1. 登録 (現時点では metadata は無視されるはず)
        registry.register_asset(manifest).await.unwrap();

        // 2. 取得
        let fetched = registry.get_asset(asset_id).await.unwrap();

        // 3. 検証 (RED: metadata は None のままのはず)
        assert_eq!(fetched.asset_type.as_ref(), "mcp");
        assert_eq!(
            fetched.metadata,
            Some(metadata),
            "Metadata should be preserved"
        );
    }

    #[tokio::test]
    async fn test_registry_check_ownership() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        let agent_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        // 購入前
        assert!(!registry.check_ownership(agent_id, asset_id).await.unwrap());

        // ライセンスの付与 (正当な所有権確立)
        registry
            .grant_license(agent_id, asset_id, "evt_test_ownership".to_string())
            .await
            .unwrap();

        // 購入後
        assert!(registry.check_ownership(agent_id, asset_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_registry_check_ownership_denial() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);
        let agent_id = Uuid::new_v4();
        let asset_id = Uuid::new_v4();

        // ライセンスがない場合は拒否
        assert!(!registry.check_ownership(agent_id, asset_id).await.unwrap());
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
        registry
            .grant_license(agent_id, asset_id, "evt_test_grant".to_string())
            .await
            .unwrap();

        // 購入後
        assert!(registry.check_ownership(agent_id, asset_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_registry_clear_mcp_servers() {
        let pool = setup_db_for_registry().await;
        let registry = RegistryManager::new(pool);

        // 1. MCPサーバーと通常アセットを登録
        registry
            .register_mcp_server(
                Uuid::new_v4(),
                "ga4",
                "Google Analytics",
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let asset_id = Uuid::new_v4();
        let manifest = AssetManifest {
            id: asset_id,
            creator_id: Uuid::new_v4(),
            asset_type: AssetType::VoiceModel,
            name: "Other Asset".into(),
            description: "Keep me".into(),
            price_coins: 0,
            metadata: None,
        };
        registry.register_asset(manifest).await.unwrap();

        assert_eq!(registry.list_mcp_servers().await.unwrap().len(), 1);

        // 2. クリア実行
        registry.clear_mcp_servers().await.unwrap();

        // 3. MCPは0件、通常アセットは1件残っていることを確認
        assert_eq!(registry.list_mcp_servers().await.unwrap().len(), 0);
        let assets = registry
            .list_assets_by_type(AssetType::VoiceModel, None, "public")
            .await
            .unwrap();
        assert_eq!(assets.len(), 1, "Non-MCP assets should not be cleared");
    }
}
