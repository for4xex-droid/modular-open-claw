/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use async_trait::async_trait;

use crate::error::AiomeError;

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
    async fn create_verification_session(&self, user_id: &str) -> Result<EkycSession, AiomeError>;
    /// 検証ステータスを確認
    async fn check_status(&self, user_id: &str) -> Result<bool, AiomeError>;
}

/// eKYCセッションの永続化インターフェース
#[async_trait]
pub trait EkycSessionStore: Send + Sync {
    /// セッションIDを保存する (1ユーザー1セッション)
    async fn save(&self, user_id: &str, session_id: &str) -> Result<(), AiomeError>;
    /// 保存されているセッションIDを取得する
    async fn get_session_id(&self, user_id: &str) -> Result<Option<String>, AiomeError>;
}
