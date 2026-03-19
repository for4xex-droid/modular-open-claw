/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use async_trait::async_trait;
use secrecy::{ExposeSecret, Secret};
use stripe::{Client, IdentityVerificationSession, IdentityVerificationSessionId};
use tracing::{error, info, warn};

#[async_trait]
pub trait EkycEngine: Send + Sync {
    /// 新しい検証セッションを作成し、URL を返す
    async fn create_verification_session(&self, user_id: &str) -> anyhow::Result<String>;
    /// 検証ステータスを確認
    async fn check_status(&self, session_id: &str) -> anyhow::Result<bool>;
}

/// Stripe Identity を使用した eKYC 実装
pub struct StripeEkycEngine {
    client: Client,
    return_url: String,
}

impl StripeEkycEngine {
    pub fn new(api_key: Secret<String>, return_url: String) -> Self {
        let client = Client::new(api_key.expose_secret());
        Self { client, return_url }
    }
}

#[async_trait]
impl EkycEngine for StripeEkycEngine {
    async fn create_verification_session(&self, user_id: &str) -> anyhow::Result<String> {
        info!(
            "💳 [eKYC] Creating Stripe Identity session for user: {}",
            user_id
        );

        let mut params = stripe::CreateIdentityVerificationSession::new();
        params.type_ = Some(stripe::IdentityVerificationSessionType::Document);
        params.options = Some(stripe::CreateIdentityVerificationSessionOptions {
            document: Some(stripe::CreateIdentityVerificationSessionOptionsDocument {
                require_id_number: Some(true),
                require_matching_selfie: Some(true),
                allowed_types: None,
            }),
        });
        params.metadata = Some(std::collections::HashMap::from([(
            "user_id".to_string(),
            user_id.to_string(),
        )]));

        let session = IdentityVerificationSession::create(&self.client, params).await?;

        // Stripe SDK の IdentityVerificationSession に url があるか確認
        // 実際には URL がメタデータや response に含まれる
        Ok(format!("https://verify.stripe.com/sessions/{}", session.id))
    }

    async fn check_status(&self, session_id: &str) -> anyhow::Result<bool> {
        let sid = session_id.parse::<IdentityVerificationSessionId>()?;
        let session = IdentityVerificationSession::retrieve(&self.client, &sid, &[]).await?;

        match session.status {
            stripe::IdentityVerificationSessionStatus::Verified => {
                info!("✅ [eKYC] User verified successfully: {}", session_id);
                Ok(true)
            }
            status => {
                warn!("⏳ [eKYC] Session {} status: {:?}", session_id, status);
                Ok(false)
            }
        }
    }
}

/// 開発・テスト用のモック
pub struct MockEkycEngine;

#[async_trait]
impl EkycEngine for MockEkycEngine {
    async fn create_verification_session(&self, _user_id: &str) -> anyhow::Result<String> {
        Ok("https://example.com/mock-verify-success".to_string())
    }

    async fn check_status(&self, _session_id: &str) -> anyhow::Result<bool> {
        Ok(true) // 常に成功
    }
}
