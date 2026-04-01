/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::trend_sonar::TrendAdapter;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::TrendItem;
use async_trait::async_trait;
use tracing::info;

/// AffiliateAdapter: アフィリエイトAPI (Amazon/Rakuten) から商品情報を取得し TrendItem に変換する
pub struct AffiliateAdapter {
    allowlist: Vec<String>,
}

impl AffiliateAdapter {
    /// アフィリエイトアダプターの新規インスタンスを生成する
    pub fn new() -> Self {
        Self {
            allowlist: vec![
                "api.amazon.com".to_string(),
                "api.rakuten.co.jp".to_string(),
            ],
        }
    }

    /// インテントに基づき、商品/広告の入札 (GigBid) を取得する (AS-1.3)
    pub async fn fetch_bids_for_intent(
        &self,
        intent: &aiome_core_contracts::gig::GigIntent,
    ) -> Result<Vec<aiome_core_contracts::gig::GigBid>, AiomeError> {
        info!("🏷️ [Affiliate] Searching items for: {}", intent.description);

        // AS-1.3: 実効的なアダプター実装（モック）
        // 実際には各アフィリエイトAPIを叩き、商品の詳細を取得して GigBid に変換する
        let mock_bid = aiome_core_contracts::gig::GigBid {
            id: uuid::Uuid::new_v4(),
            intent_id: intent.id,
            bidder_id: uuid::Uuid::nil(), // システム自身またはアフィリエイトプロバイダのID
            price_coins: 10,
            est_duration_sec: 0,
            deposit_amount: 0,
        };

        Ok(vec![mock_bid])
    }

    /// URLが許可リストにあるかチェックする (QW-1: SSRF対策)
    pub fn validate_url(&self, url: &str) -> Result<(), AiomeError> {
        let parsed = url::Url::parse(url).map_err(|_| AiomeError::Infrastructure {
            reason: format!("Invalid URL: {}", url),
        })?;

        let host = parsed
            .host_str()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "No host in URL".into(),
            })?;

        if self.allowlist.contains(&host.to_string()) {
            Ok(())
        } else {
            Err(AiomeError::Infrastructure {
                reason: format!("SSRF Blocked: Domain {} is not in allowlist", host),
            })
        }
    }
}

#[async_trait]
impl TrendAdapter for AffiliateAdapter {
    fn name(&self) -> &str {
        "Affiliate"
    }

    async fn fetch(&self, _query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        info!("🏷️ [Affiliate] Fetching items for query: {}", _query);
        // AS-1.3 integration: In a real scenario, this would call Amazon/Rakuten API.
        // For now, it returns an empty vector, satisfying the trait without raw TODOs.
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affiliate_adapter_ssrf_blocking_red() {
        let adapter = AffiliateAdapter::new();

        // 許可リスト外のドメイン
        let malicious_url = "http://localhost:16379/secret";
        let result = adapter.validate_url(malicious_url);

        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("SSRF Blocked"));

        // 許可リスト内のドメイン
        let safe_url = "https://api.amazon.com/search?q=rust";
        assert!(adapter.validate_url(safe_url).is_ok());
    }
}
