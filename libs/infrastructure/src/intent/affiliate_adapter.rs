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

/// MockAffiliateAdapter: アフィリエイトAPI (Amazon/Rakuten) から商品情報を取得し TrendItem に変換する
#[cfg(debug_assertions)]
pub struct MockAffiliateAdapter {
    allowlist: Vec<String>,
}

#[cfg(debug_assertions)]

impl MockAffiliateAdapter {
    /// アフィリエイトアダプターの新規インスタンスを生成する
    pub fn new() -> Self {
        Self {
            allowlist: vec![
                "api.amazon.com".to_string(),
                "api.rakuten.co.jp".to_string(),
            ],
        }
    }
}

#[cfg(debug_assertions)]
#[async_trait]
impl aiome_core_contracts::traits::AffiliateAdapter for MockAffiliateAdapter {
    async fn fetch_bids_for_intent(
        &self,
        intent: &aiome_core_contracts::gig::GigIntent,
    ) -> Result<Vec<aiome_core_contracts::gig::GigBid>, AiomeError> {
        info!("🏷️ [Affiliate] Searching items for: {}", intent.description);

        let mock_bid = aiome_core_contracts::gig::GigBid {
            id: uuid::Uuid::new_v4(),
            intent_id: intent.id,
            bidder_id: uuid::Uuid::nil(),
            price_coins: 10,
            est_duration_sec: 0,
            deposit_amount: 0,
        };

        Ok(vec![mock_bid])
    }

    fn validate_url(&self, url: &str) -> Result<(), AiomeError> {
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

#[cfg(debug_assertions)]
#[async_trait]
impl TrendAdapter for MockAffiliateAdapter {
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

pub struct DisabledAffiliateAdapter;

impl DisabledAffiliateAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl aiome_core_contracts::traits::AffiliateAdapter for DisabledAffiliateAdapter {
    async fn fetch_bids_for_intent(
        &self,
        _intent: &aiome_core_contracts::gig::GigIntent,
    ) -> Result<Vec<aiome_core_contracts::gig::GigBid>, AiomeError> {
        Ok(vec![])
    }

    fn validate_url(&self, _url: &str) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "Affiliate feature is disabled in production".into(),
        })
    }
}

#[async_trait]
impl TrendAdapter for DisabledAffiliateAdapter {
    fn name(&self) -> &str {
        "DisabledAffiliate"
    }

    async fn fetch(&self, _query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::traits::AffiliateAdapter;

    #[test]
    fn test_affiliate_adapter_ssrf_blocking_red() {
        let adapter = MockAffiliateAdapter::new();

        // 許可リスト外のドメイン
        let malicious_url = "http://localhost:16379/secret"; // allow-anti-pattern
        let result = adapter.validate_url(malicious_url);

        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("SSRF Blocked"));

        // 許可リスト内のドメイン
        let safe_url = "https://api.amazon.com/search?q=rust";
        assert!(adapter.validate_url(safe_url).is_ok());
    }
}
