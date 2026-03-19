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

/// インターフェース: ユーザー検証 (eKYC) エンジン
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
    /// Stripe Identity Client を初期化する
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

        // FIXME: async-stripe 0.41.0 において IdentityVerificationSession の正確な型や機能が不足しているため、
        // 現状は HTTP Client による直接の API コールまたは、外部サービス連携を意図したダミーURLを返す。
        // （Phase 8.1 実装では、Stripe API への直接のリクエスト処理を reqwest 経由で行うことが推奨されます）
        warn!("Stripe eKYC API wrapping is currently falling back to mock behavior due to missing structs in async-stripe.");

        Ok(format!(
            "https://verify.stripe.com/sessions/mock-{}",
            user_id
        ))
    }

    async fn check_status(&self, session_id: &str) -> anyhow::Result<bool> {
        // FIXME: 同様にステータスチェックも一旦ダミー実装
        info!(
            "✅ [eKYC] User verified successfully (Mocked check for {}).",
            session_id
        );
        Ok(true)
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
