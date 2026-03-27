/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use aiome_contracts::llm::{LlmResponse, StopReason};
use aiome_contracts::traits::LoraEngine as LoraEngineTrait;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Phase 10.1b: LoRAモデルのメタデータを管理するエンジン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraModel {
    /// 一意の ID
    pub id: String,
    /// モデルの表示名
    pub name: String,
    /// モデルファイルのハッシュ値
    pub lora_hash: String,
    /// ベースモデル名 (例: stable-diffusion-v1-5)
    pub base_model: String,
    /// ファイルシステム上のパス
    pub file_path: String,
}

/// LoRAエンジン - ロードされたモデル群を管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraEngine {
    /// ロード済みのLoRAモデル一覧
    pub models: Vec<LoraModel>,
}

impl Default for LoraEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LoraEngine {
    /// 新規LoRAエンジンを生成する
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    /// ハッシュからLoRAモデルを検索する
    pub fn find_by_hash(&self, hash: &str) -> Result<LoraModel, AiomeError> {
        self.models
            .iter()
            .find(|m| m.lora_hash == hash)
            .cloned()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: format!("LoRA model with hash {} not found", hash),
            })
    }
}

#[async_trait]
impl LoraEngineTrait for LoraEngine {
    async fn complete_with_lora(
        &self,
        prompt: &str,
        lora_id: &str,
    ) -> Result<LlmResponse, AiomeError> {
        // Find by ID or Hash
        let _model = self
            .models
            .iter()
            .find(|m| m.id == lora_id || m.lora_hash == lora_id)
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: format!("LoRA model {} not found", lora_id),
            })?;

        // Placeholder implementation for AppState stabilization
        Ok(LlmResponse {
            content: format!("[LoRA: {}] Generated response for: {}", lora_id, prompt),
            stop_reason: StopReason::EndTurn,
            reasoning: Some("Mock LoRA generation".into()),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_find_by_hash_green() {
        let mut engine = LoraEngine::new();
        let test_model = LoraModel {
            id: "m1".into(),
            name: "My Voice".into(),
            lora_hash: "sha256:12345".into(),
            base_model: "stable-diffusion-v1-5".into(),
            file_path: "/tmp/lora.safetensors".into(),
        };
        engine.models.push(test_model.clone());

        let res = engine.find_by_hash("sha256:12345");
        assert!(res.is_ok());
        assert_eq!(res.unwrap().id, "m1");
    }
}
