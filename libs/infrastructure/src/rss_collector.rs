/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::SqliteJobQueue;
use aiome_core::error::AiomeError;
use aiome_contracts::traits::{TrendItem, TrendSource};
use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;
use sqlx::Row;
use tracing::{info, warn};

/// トレンドソースとしての RSS コレクター
/// 複数のRSSフィードから最新記事を取得し、キーワードを抽出する。
pub struct RssCollector {
    client: reqwest::Client,
    jq: Arc<SqliteJobQueue>,
}

impl RssCollector {
    pub fn new(jq: Arc<SqliteJobQueue>) -> Self {
        Self {
            client: reqwest::Client::new(),
            jq,
        }
    }

    /// フィードから生のXMLを取得し、簡易パースする
    async fn fetch_items(&self, url: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // L2 Cache (Trend Cache) の確認
        if let Some(cached) = self.get_cached_trend(url).await? {
            return Ok(cached);
        }

        let resp = self.client.get(url).send().await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to fetch RSS feed: {}", e),
            })?;
        
        let xml = resp.text().await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to read RSS text: {}", e),
            })?;

        // 簡易正規表現パース (RSS 2.0 / Atom 共通項)
        let title_re = Regex::new(r"<title>(.*?)</title>").unwrap();
        let mut items = Vec::new();

        for cap in title_re.captures_iter(&xml) {
            let title = cap[1].trim().to_string();
            if title.is_empty() || title == "RSS" { continue; }
            
            items.push(TrendItem {
                keyword: title,
                source: "RSS".to_string(),
                score: 1.0, // RSSは一律1.0（後のOracleで重み付け）
            });
        }

        // Cache into Trend Cache (TTL 1時間)
        self.set_cached_trend(url, &items, 3600).await?;

        Ok(items)
    }

    async fn get_cached_trend(&self, url: &str) -> Result<Option<Vec<TrendItem>>, AiomeError> {
        let row = sqlx::query(
            "SELECT content FROM trend_cache 
             WHERE source_url = ? AND expires_at > datetime('now')"
        )
        .bind(url)
        .fetch_optional(&self.jq.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Trend cache lookup failed: {}", e),
        })?;

        if let Some(row) = row {
            let content: String = row.get(0);
            let items: Vec<TrendItem> = serde_json::from_str(&content)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Cache decode failed: {}", e),
                })?;
            return Ok(Some(items));
        }
        Ok(None)
    }

    async fn set_cached_trend(&self, url: &str, items: &[TrendItem], ttl_sec: i64) -> Result<(), AiomeError> {
        let json = serde_json::to_string(items).unwrap();
        sqlx::query(
            "INSERT OR REPLACE INTO trend_cache (source_url, content, expires_at) 
             VALUES (?, ?, datetime('now', '+' || ? || ' seconds'))"
        )
        .bind(url)
        .bind(json)
        .bind(ttl_sec)
        .execute(&self.jq.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Trend cache write failed: {}", e),
        })?;
        Ok(())
    }
}

#[async_trait]
impl TrendSource for RssCollector {
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        self.fetch(category).await
    }
}

use crate::trend_sonar::TrendAdapter;

#[async_trait]
impl TrendAdapter for RssCollector {
    fn name(&self) -> &str { "RSS" }

    async fn fetch(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // 設定からカテゴリに応じたRSS URLを取得する想定 (現在はモックでGoogle News)
        let urls = match category {
            "tech" => vec!["https://news.google.com/rss/search?q=technology&hl=ja&gl=JP&ceid=JP:ja"],
            "business" => vec!["https://news.google.com/rss/search?q=business&hl=ja&gl=JP&ceid=JP:ja"],
            _ => vec!["https://news.google.com/rss?hl=ja&gl=JP&ceid=JP:ja"],
        };

        let mut all_items = Vec::new();
        for url in urls {
            match self.fetch_items(url).await {
                Ok(mut items) => all_items.append(&mut items),
                Err(e) => warn!("⚠️ [RssCollector] Failed to fetch feed {}: {}", url, e),
            }
        }

        Ok(all_items)
    }
}
