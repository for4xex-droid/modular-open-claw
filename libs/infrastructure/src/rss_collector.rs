/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::{NewsService, TrendItem, TrendSource};
use async_trait::async_trait;
use regex::Regex;
use serde_json;
use sqlx::Row;
use std::sync::Arc;
use tracing::{info, warn};

#[async_trait]
pub trait TrendCacheRepository: Send + Sync + std::fmt::Debug {
    async fn get_cached_trend(&self, url: &str) -> Result<Option<Vec<TrendItem>>, AiomeError>;
    async fn set_cached_trend(
        &self,
        url: &str,
        items: &[TrendItem],
        ttl_sec: i64,
    ) -> Result<(), AiomeError>;
}

#[derive(Debug)]
pub struct SqlTrendCacheRepository {
    pool: crate::db::DatabasePool,
}

impl SqlTrendCacheRepository {
    pub fn new(pool: crate::db::DatabasePool) -> Self {
        Self { pool }
    }
}

/// トレンドソースとしての RSS コレクター
/// 複数のRSSフィードから最新記事を取得し、キーワードを抽出する。
#[derive(Debug)]
pub struct RssCollector {
    client: reqwest::Client,
    cache: Arc<dyn TrendCacheRepository>,
}

impl RssCollector {
    /// RssCollector の新規インスタンスを生成する
    pub fn new(cache: Arc<dyn TrendCacheRepository>) -> Self {
        Self {
            client: aiome_core::http::get_http_client().clone(),
            cache,
        }
    }

    /// フィードから生のXMLを取得し、簡易パースする
    async fn fetch_items(&self, url: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // L2 Cache (Trend Cache) の確認
        if let Some(cached) = self.cache.get_cached_trend(url).await? {
            return Ok(cached);
        }

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to fetch RSS feed: {}", e),
            })?;

        let xml = resp.text().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to read RSS text: {}", e),
        })?;

        // 簡易正規表現パース (RSS 2.0 / Atom 共通項)
        let title_re = Regex::new(r"<title>(.*?)</title>").unwrap(); // allow-anti-pattern
        let mut items = Vec::new();

        for cap in title_re.captures_iter(&xml) {
            let title_raw = cap[1].trim();
            let title = crate::trend_sonar::sanitize_snippet(title_raw);
            if title.is_empty() || title == "RSS" {
                continue;
            }

            items.push(TrendItem {
                keyword: title,
                source: "RSS".to_string(),
                score: 1.0, // RSSは一律1.0（後のOracleで重み付け）
            });
        }

        // Cache into Trend Cache (TTL 1時間)
        self.cache.set_cached_trend(url, &items, 3600).await?;

        Ok(items)
    }
}

#[async_trait]
impl TrendCacheRepository for SqlTrendCacheRepository {
    async fn get_cached_trend(&self, url: &str) -> Result<Option<Vec<TrendItem>>, AiomeError> {
        let content_opt = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let row = sqlx::query("SELECT content FROM trend_cache WHERE source_url = ? AND expires_at > datetime('now')")
                    .bind(url)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
                row.map(|r| r.get::<String, _>(0))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let row = sqlx::query(
                    "SELECT content FROM trend_cache WHERE source_url = $1 AND expires_at > NOW()",
                )
                .bind(url)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
                row.map(|r| r.get::<String, _>(0))
            }
        };

        if let Some(content) = content_opt {
            let items: Vec<TrendItem> =
                serde_json::from_str(&content).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Cache decode failed: {}", e),
                })?;
            return Ok(Some(items));
        }
        Ok(None)
    }

    async fn set_cached_trend(
        &self,
        url: &str,
        items: &[TrendItem],
        ttl_sec: i64,
    ) -> Result<(), AiomeError> {
        let json = serde_json::to_string(items).unwrap(); // allow-anti-pattern
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query("INSERT OR REPLACE INTO trend_cache (source_url, content, expires_at) VALUES (?, ?, datetime('now', '+' || ? || ' seconds'))")
                    .bind(url)
                    .bind(&json)
                    .bind(ttl_sec)
                    .execute(p)
                    .await
                    .map(|_| ())
            }
            crate::db::DatabasePool::Postgres(p) => {
                let interval = format!("{} seconds", ttl_sec);
                sqlx::query("INSERT INTO trend_cache (source_url, content, expires_at) VALUES ($1, $2, NOW() + $3::interval) ON CONFLICT (source_url) DO UPDATE SET content = EXCLUDED.content, expires_at = EXCLUDED.expires_at")
                    .bind(url)
                    .bind(&json)
                    .bind(interval)
                    .execute(p)
                    .await
                    .map(|_| ())
            }
        }
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

#[async_trait]
impl NewsService for RssCollector {
    async fn fetch_latest(&self, query: &str) -> Result<Vec<serde_json::Value>, AiomeError> {
        let items = self.fetch(query).await?;
        let values = items
            .into_iter()
            .map(|it| {
                serde_json::json!({
                    "title": it.keyword,
                    "source": it.source,
                    "relevance": it.score,
                })
            })
            .collect();
        Ok(values)
    }
}

use crate::trend_sonar::TrendAdapter;

#[async_trait]
impl TrendAdapter for RssCollector {
    fn name(&self) -> &str {
        "RSS"
    }

    async fn fetch(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // 設定からカテゴリに応じたRSS URLを取得する想定 (現在はモックでGoogle News)
        let urls = match category {
            "tech" => {
                vec!["https://news.google.com/rss/search?q=technology&hl=ja&gl=JP&ceid=JP:ja"]
            }
            "business" => {
                vec!["https://news.google.com/rss/search?q=business&hl=ja&gl=JP&ceid=JP:ja"]
            }
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_sanitization_unit() {
        let xml = r#"<rss><channel>
            <item><title><b>Trending</b> &amp; News</title></item>
            <item><title>Visit https://malicious.link/trap</title></item>
            <item><title>  Clean   Title  </title></item>
        </channel></rss>"#;

        let title_re = regex::Regex::new(r"<title>(.*?)</title>").unwrap(); // allow-anti-pattern
        let mut items = Vec::new();
        for cap in title_re.captures_iter(&xml) {
            let title_raw = cap[1].trim();
            let title = crate::trend_sonar::sanitize_snippet(title_raw);
            if !title.is_empty() && title != "RSS" {
                items.push(title);
            }
        }

        assert!(items.contains(&"Trending & News".to_string()));
        assert!(items.contains(&"Visit".to_string()));
        assert!(items.contains(&"Clean Title".to_string()));
    }

    #[test]
    fn test_rss_sanitization_xss_bypass() {
        let xss_inputs = vec![
            ("<script>alert(1)</script>", ""),
            ("<img src=x onerror=alert(1)>", ""),
            ("<scr<script>ipt>alert(1)</script>", ""),
            ("<a href=\"javascript:alert(1)\">Click me</a>", "Click me"),
            (
                "<div>Safe <b>Bold</b> <iframe src='malicious.com'></iframe></div>",
                "Safe Bold",
            ),
        ];

        for (input, expected) in xss_inputs {
            let sanitized = crate::trend_sonar::sanitize_snippet(input);
            assert_eq!(sanitized, expected, "Failed to sanitize: {}", input);
        }
    }
}
