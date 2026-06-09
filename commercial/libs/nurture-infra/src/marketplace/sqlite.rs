/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use commerce_protocol::commodity::{CommodityKind, ItemDescriptor, PriceTag};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub struct SQLiteMarketplace {
    pool: SqlitePool,
}

impl SQLiteMarketplace {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_item(&self, id: &Uuid) -> Result<ItemDescriptor, NurtureError> {
        let row = sqlx::query(
            "SELECT id, kind, name, description, price_coins, creator_id, created_at, metadata, 
             sale_mode, drm_enabled, subscription_interval_days, subscription_price_coins, content_hash 
             FROM nurture_items WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| NurtureError::Infrastructure(format!("DBエラー: {}", e)))?;

        match row {
            Some(row) => {
                let kind_str: String = row.get("kind");
                let creator_id_str: String = row.get("creator_id");
                let metadata_str: String = row.get("metadata");

                let kind: CommodityKind = serde_json::from_str(&format!("\"{}\"", kind_str))
                    .map_err(|e| {
                        NurtureError::Infrastructure(format!("商品種別パースエラー: {}", e))
                    })?;

                let creator_id = Uuid::parse_str(&creator_id_str).map_err(|e| {
                    NurtureError::Infrastructure(format!("クリエイターIDパースエラー: {}", e))
                })?;

                let metadata = serde_json::from_str(&metadata_str).map_err(|e| {
                    NurtureError::Infrastructure(format!("メタデータパースエラー: {}", e))
                })?;

                let sale_mode_str: String = row.get("sale_mode");
                let sale_mode = if sale_mode_str == "Subscription" {
                    commerce_protocol::offer::SaleMode::Subscription {
                        interval_days: row
                            .try_get::<i32, _>("subscription_interval_days")
                            .unwrap_or(30)
                            .try_into()
                            .unwrap_or(30u32),
                        price_coins: row
                            .try_get::<i64, _>("subscription_price_coins")
                            .unwrap_or(0)
                            .max(0) as u64,
                    }
                } else {
                    commerce_protocol::offer::SaleMode::Instant
                };

                Ok(ItemDescriptor {
                    id: *id,
                    kind,
                    name: row.get("name"),
                    description: row.get("description"),
                    price: PriceTag::Fixed({
                        let v: i64 = row.get("price_coins");
                        u64::try_from(v).map_err(|_| {
                            NurtureError::Infrastructure(format!("price_coins が負の値です: {}", v))
                        })?
                    }),
                    creator_id: ActorId(creator_id),
                    sale_mode,
                    drm_enabled: row.get::<i32, _>("drm_enabled") != 0,
                    created_at: row.get("created_at"),
                    metadata,
                    content_hash: row.try_get("content_hash").ok(),
                })
            }
            None => Err(NurtureError::ItemNotFound(*id)),
        }
    }

    pub async fn search_items(
        &self,
        query: String,
        limit: u32,
    ) -> Result<Vec<ItemDescriptor>, NurtureError> {
        // LIKE句のワイルドカードをエスケープ (🔴 D5 解決)
        let escaped_query = query.replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{}%", escaped_query);

        // MarketSearchRequest.limit を反映 (🔴 D6 解決)
        let sql =
            "SELECT id, kind, name, description, price_coins, creator_id, created_at, metadata,
             sale_mode, drm_enabled, subscription_interval_days, subscription_price_coins, content_hash 
             FROM nurture_items 
             WHERE name LIKE ? ESCAPE '\\' OR description LIKE ? ESCAPE '\\' 
             ORDER BY created_at DESC LIMIT ?";

        let rows = sqlx::query(sql)
            .bind(&pattern)
            .bind(&pattern)
            .bind(i64::from(limit.min(100))) // DOS 防止 + パラメータバインド
            .fetch_all(&self.pool)
            .await
            .map_err(|e| NurtureError::Infrastructure(format!("DBエラー: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let kind_str: String = row.get("kind");
            let creator_id_str: String = row.get("creator_id");
            let metadata_str: String = row.get("metadata");

            let id = Uuid::parse_str(&id_str).map_err(|e| {
                NurtureError::Infrastructure(format!("アイテムIDパースエラー: {}", e))
            })?;

            let kind: CommodityKind =
                serde_json::from_str(&format!("\"{}\"", kind_str)).map_err(|e| {
                    NurtureError::Infrastructure(format!("商品種別パースエラー: {}", e))
                })?;
            let creator_id = Uuid::parse_str(&creator_id_str).map_err(|e| {
                NurtureError::Infrastructure(format!("クリエイターIDパースエラー: {}", e))
            })?;

            let metadata = serde_json::from_str(&metadata_str).map_err(|e| {
                NurtureError::Infrastructure(format!("メタデータパースエラー: {}", e))
            })?;

            let sale_mode_str: String = row.get("sale_mode");
            let sale_mode = if sale_mode_str == "Subscription" {
                commerce_protocol::offer::SaleMode::Subscription {
                    interval_days: row
                        .try_get::<i32, _>("subscription_interval_days")
                        .unwrap_or(30)
                        .try_into()
                        .unwrap_or(30u32),
                    price_coins: row
                        .try_get::<i64, _>("subscription_price_coins")
                        .unwrap_or(0)
                        .max(0) as u64,
                }
            } else {
                commerce_protocol::offer::SaleMode::Instant
            };

            items.push(ItemDescriptor {
                id,
                kind,
                name: row.get("name"),
                description: row.get("description"),
                price: PriceTag::Fixed({
                    let v: i64 = row.get("price_coins");
                    u64::try_from(v).map_err(|_| {
                        NurtureError::Infrastructure(format!("price_coins が負の値です: {}", v))
                    })?
                }),
                creator_id: ActorId(creator_id),
                sale_mode,
                drm_enabled: row.get::<i32, _>("drm_enabled") != 0,
                created_at: row.get("created_at"),
                metadata,
                content_hash: row.try_get("content_hash").ok(),
            });
        }
        Ok(items)
    }

    pub async fn create_item(&self, item: &ItemDescriptor) -> Result<(), NurtureError> {
        let kind_str = match &item.kind {
            CommodityKind::VrmAvatar => "VrmAvatar",
            CommodityKind::ClothingPart => "ClothingPart",
            CommodityKind::Accessory => "Accessory",
            CommodityKind::WasmSkill => "WasmSkill",
            CommodityKind::KnowledgePack => "KnowledgePack",
            CommodityKind::Expression => "Expression",
            CommodityKind::VoiceModel => "VoiceModel",
            CommodityKind::KarmaPackage => "KarmaPackage",
            CommodityKind::AutomationBlueprint => "AutomationBlueprint",
            CommodityKind::LoraAdapter => "LoraAdapter",
            CommodityKind::GeneticBlueprint => "GeneticBlueprint",
            CommodityKind::BiomeEnvironment => "BiomeEnvironment",
        };

        let price_coins: i64 = match item.price {
            PriceTag::Free => 0,
            PriceTag::Fixed(p) => i64::try_from(p).map_err(|_| {
                NurtureError::Infrastructure(format!(
                    "price_coins が i64 の範囲を超えています: {}",
                    p
                ))
            })?,
            PriceTag::Negotiable { min, .. } => i64::try_from(min).map_err(|_| {
                NurtureError::Infrastructure(format!(
                    "price_coins (min) が i64 の範囲を超えています: {}",
                    min
                ))
            })?,
        };

        let (sale_mode, sub_interval, sub_price) = match &item.sale_mode {
            commerce_protocol::offer::SaleMode::Instant => ("Instant", None, None),
            commerce_protocol::offer::SaleMode::Subscription {
                interval_days,
                price_coins,
            } => {
                let days = i32::try_from(*interval_days).map_err(|_| {
                    NurtureError::Infrastructure(format!(
                        "subscription interval_days が i32 の範囲を超えています: {}",
                        interval_days
                    ))
                })?;
                let sub_price = i64::try_from(*price_coins).map_err(|_| {
                    NurtureError::Infrastructure(format!(
                        "subscription price_coins が i64 の範囲を超えています: {}",
                        price_coins
                    ))
                })?;
                ("Subscription", Some(days), Some(sub_price))
            }
        };

        let metadata_str =
            serde_json::to_string(&item.metadata).unwrap_or_else(|_| "{}".to_string());

        sqlx::query(
            "INSERT INTO nurture_items (
                id, kind, name, description, price_coins, creator_id, created_at, metadata, 
                sale_mode, drm_enabled, subscription_interval_days, subscription_price_coins, content_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.to_string())
        .bind(kind_str)
        .bind(&item.name)
        .bind(&item.description)
        .bind(price_coins)
        .bind(item.creator_id.0.to_string())
        .bind(item.created_at)
        .bind(metadata_str)
        .bind(sale_mode)
        .bind(i32::from(item.drm_enabled))
        .bind(sub_interval)
        .bind(sub_price)
        .bind(&item.content_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            NurtureError::Infrastructure(format!("アイテムの作成に失敗しました: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("マイグレーションの実行に失敗しました");
        pool
    }

    #[tokio::test]
    async fn test_marketplace_get_and_search() {
        let pool = setup_db().await;
        let mp = SQLiteMarketplace::new(pool.clone());
        let item_id = Uuid::new_v4();
        let creator_id = Uuid::new_v4();

        // データの挿入
        sqlx::query(
            "INSERT INTO nurture_items (id, kind, name, description, price_coins, creator_id, created_at, metadata, sale_mode, drm_enabled)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(item_id.to_string())
        .bind("VrmAvatar")
        .bind("特製アバター 100%")
        .bind("説明文です")
        .bind(500i64)
        .bind(creator_id.to_string())
        .bind(Utc::now())
        .bind("{}")
        .bind("Instant")
        .bind(0i32)
        .execute(&pool).await.unwrap();

        // 取得テスト
        let item = mp.get_item(&item_id).await.unwrap();
        assert_eq!(item.name, "特製アバター 100%");
        assert_eq!(item.price, PriceTag::Fixed(500));

        // 検索テスト (エスケープ確認)
        let search_results = mp.search_items("100%".to_string(), 10).await.unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, item_id);
    }

    #[tokio::test]
    async fn test_negative_price_clamping() {
        // [Reflexion Sprint A v1] 回帰テスト: 負の price_coins を安全にキャストできるか
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE nurture_items (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                price_coins INTEGER,
                creator_id TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                metadata TEXT,
                sale_mode TEXT NOT NULL,
                drm_enabled INTEGER NOT NULL,
                subscription_interval_days INTEGER,
                subscription_price_coins BIGINT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mp = SQLiteMarketplace::new(pool.clone());
        let item_id = Uuid::new_v4();

        // 負の価格を持つアイテムを直接DBに挿入
        sqlx::query(
            "INSERT INTO nurture_items (id, kind, name, description, price_coins, creator_id, created_at, metadata, sale_mode, drm_enabled)
             VALUES (?, 'VrmAvatar', 'Negative Price', 'Desc', -500, ?, ?, '{}', 'Instant', 0)"
        )
        .bind(item_id.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(Utc::now())
        .execute(&pool).await.unwrap();

        let result = mp.get_item(&item_id).await;
        // 負の価格は fail-closed でエラーとして拒否される
        assert!(
            result.is_err(),
            "負の price_coins は Infrastructure エラーを返すべき"
        );
    }
}
