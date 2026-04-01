/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use crate::defense::DefenseAction;
use crate::error::SoulError;
use crate::model::{AgentSoul, Experience};

/// ドメインアダプター（各アプリケーションが実装）
pub trait SoulDomainAdapter: Send + Sync {
    /// 生イベント → Experience 変換
    fn to_experience(&self, raw: &dyn Any) -> Experience;

    /// 蒸留時の LLM プロンプト（ドメイン固有）
    fn distillation_system_prompt(&self) -> &str;

    /// DefenseAction をドメイン固有の副作用に変換
    fn execute_defense<'a>(
        &'a self,
        action: &'a DefenseAction,
        context: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), SoulError>> + Send + 'a>>;

    /// 予測モデルの予測値を算出（ドメイン固有のメトリクス）
    fn predict_outcome(&self, soul: &AgentSoul, context: &Experience) -> f64;

    /// 本能に刻印する埋め込みベクトルを生成（非同期）
    fn embed_experience<'a>(
        &'a self,
        _exp: &'a Experience,
    ) -> Pin<Box<dyn Future<Output = Vec<f32>> + Send + 'a>> {
        Box::pin(async { Vec::new() })
    }
}
