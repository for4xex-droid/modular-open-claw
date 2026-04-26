/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// RLMからの構造化レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmResponse {
    /// 生成されたテキスト内容
    pub content: String,
    /// 再帰推論の深さ
    pub recursion_depth: usize,
    /// 消費されたコスト (USD)
    pub cost_usd: f64,
}

/// RLMリクエストの構成
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RlmConfig {
    /// 最大再帰深さ
    pub max_depth: usize,
    /// 予算上限 (USD)
    pub max_budget_usd: f64,
}

/// 再帰的言語モデル (RLM) プロバイダーのインターフェース
#[async_trait]
pub trait RlmProvider: Send + Sync + Debug {
    /// 再帰推論を実行する
    async fn deep_complete(
        &self,
        prompt: &str,
        config: RlmConfig,
    ) -> Result<RlmResponse, AiomeError>;

    /// 接続テスト
    async fn test_connection(&self) -> Result<(), AiomeError>;

    /// プロバイダー名を取得（デバッグ用）
    fn name(&self) -> &str;
}
