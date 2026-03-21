/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use async_trait::async_trait;
use secrecy::{ExposeSecret, Secret};
use stripe::Client;
use tracing::{error, info, warn};

/// eKYCセッション情報
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EkycSession {
    /// 検証用URL
    pub url: String,
    /// StripeセッションID (永続化用)
    pub session_id: String,
}

/// インターフェース: ユーザー検証 (eKYC) エンジン
#[async_trait]
pub trait EkycEngine: Send + Sync {
    /// 新しい検証セッションを作成し、検証用URLとセッションIDを返す
    async fn create_verification_session(&self, user_id: &str) -> anyhow::Result<EkycSession>;
    /// 検証ステータスを確認
    async fn check_status(&self, user_id: &str) -> anyhow::Result<bool>;
}

/// Stripe Identity を使用した eKYC 実装
pub struct StripeEkycEngine {
    client: reqwest::Client,
    api_key: Secret<String>,
    return_url: String,
}

impl StripeEkycEngine {
    /// Stripe Identity Client を初期化する
    pub fn new(api_key: Secret<String>, return_url: String, client: reqwest::Client) -> Self {
        Self {
            client,
            api_key,
            return_url,
        }
    }
}

#[async_trait]
impl EkycEngine for StripeEkycEngine {
    async fn create_verification_session(&self, user_id: &str) -> anyhow::Result<EkycSession> {
        info!(
            "💳 [eKYC] Creating real Stripe Identity session for user: {}",
            user_id
        );

        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client
                .post("https://api.stripe.com/v1/identity/verification_sessions")
                .basic_auth(self.api_key.expose_secret(), Some(""))
                .form(&[
                    ("type", "document"),
                    ("return_url", &self.return_url),
                    // Expert Review v3: client_reference_id を使用してフィルタリング可能にする
                    ("client_reference_id", user_id),
                    ("metadata[user_id]", user_id),
                ])
                .send()
        )
        .await
        .map_err(|_| anyhow::anyhow!("Stripe API timeout"))??;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            error!("💳 [eKYC] Stripe API error: {}", error_text);
            return Err(anyhow::anyhow!("Stripe API error: {}", error_text));
        }

        let json: serde_json::Value = resp.json().await?;
        let url = json
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Stripe response missing 'url'"))?;
        
        let session_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Stripe response missing 'id'"))?;

        Ok(EkycSession {
            url: url.to_string(),
            session_id: session_id.to_string(),
        })
    }

    async fn check_status(&self, user_id: &str) -> anyhow::Result<bool> {
        info!("💳 [eKYC] Checking verification status for user: {}", user_id);

        // Expert Review v3: client_reference_id でフィルタリング (Stripe API は metadata フィルタ非対応)
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client
                .get("https://api.stripe.com/v1/identity/verification_sessions")
                .basic_auth(self.api_key.expose_secret(), Some(""))
                .query(&[
                    ("limit", "1"),
                    ("client_reference_id", user_id), 
                ])
                .send()
        )
        .await
        .map_err(|_| anyhow::anyhow!("Stripe API timeout"))??;

        if !resp.status().is_success() {
            return Ok(false);
        }

        let json: serde_json::Value = resp.json().await?;
        let is_verified = json
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|s| s.get("status"))
            .map(|s| s == "verified")
            .unwrap_or(false);

        Ok(is_verified)
    }
}

/// 開発・テスト用のモック
pub struct MockEkycEngine;

#[async_trait]
impl EkycEngine for MockEkycEngine {
    async fn create_verification_session(&self, _user_id: &str) -> anyhow::Result<EkycSession> {
        Ok(EkycSession {
            url: "https://example.com/mock-verify-success".to_string(),
            session_id: "vs_mock_123".to_string(),
        })
    }

    async fn check_status(&self, user_id: &str) -> anyhow::Result<bool> {
        if user_id.contains("unverified") {
            Ok(false)
        } else {
            Ok(true)
        }
    }
}
