/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use serde::{Deserialize, Serialize};

/// Phase 10.1b: LoRAモデルのメタデータを管理するエンジン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraModel {
    pub id: String,
    pub name: String,
    pub lora_hash: String,
    pub base_model: String,
    pub file_path: String,
}

pub struct LoraEngine {
    // RED: Empty models for now
    pub models: Vec<LoraModel>,
}

impl LoraEngine {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

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
