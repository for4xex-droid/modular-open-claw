//! # TrendSonar — トレンド収集ツール
//!
//! 定時でトレンドキーワードを取得する。
//! 外部への通信はすべて Bastion ShieldClient で検証し、SSRF 攻撃を防止する。

use async_trait::async_trait;
use bastion::net_guard::ShieldClient;
use factory_core::error::FactoryError;
use factory_core::traits::{TrendItem, TrendSource};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// トレンド収集クライアント
pub struct TrendSonarClient {
    /// Bastion ネットワークシールド
    pub shield: Arc<ShieldClient>,
}

impl TrendSonarClient {
    pub fn new(shield: Arc<ShieldClient>) -> Self {
        Self { shield }
    }
}

#[async_trait]
impl TrendSource for TrendSonarClient {
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, FactoryError> {
        let url = "https://trends.google.co.jp/trends/trendingsearches/daily/rss?geo=JP";
        
        tracing::debug!("TrendSonar: Fetching trends from {} with security check...", url);
        
        match self.shield.get(url).await {
            Ok(_res) => {
                tracing::info!("TrendSonar: {} カテゴリのトレンドを取得しました (スタブ応答)", category);
                Ok(vec![
                    TrendItem {
                        keyword: "Mac mini M4 Pro".to_string(),
                        source: "Google Trends".to_string(),
                        score: 100.0,
                    }
                ])
            }
            Err(e) => {
                tracing::error!("TrendSonar: Security or connection error: {}", e);
                Err(FactoryError::Infrastructure {
                    reason: format!("Failed to fetch trends safely: {}", e),
                })
            }
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct TrendArgs {
    /// 検索するトレンドのカテゴリ (例: "tech", "entertainment", "jp_all")
    pub category: String,
}

#[derive(Serialize)]
pub struct TrendOutput {
    pub trends: Vec<TrendItem>,
}

impl Tool for TrendSonarClient {
    const NAME: &'static str = "trend_sonar";
    type Args = TrendArgs;
    type Output = TrendOutput;
    type Error = FactoryError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Google Trends や SNS から最新のトレンドキーワードを取得します。".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TrendArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let trends = self.get_trends(&args.category).await?;
        Ok(TrendOutput { trends })
    }
}
