/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::trend_sonar::TrendAdapter;
use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::TrendItem;
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

    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // TODO: 実装
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
