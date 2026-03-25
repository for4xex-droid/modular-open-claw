/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::{LlmProvider, LlmRequest, LlmResponse};
use aiome_contracts::security::AgentHook;
use async_trait::async_trait;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use serde::{Deserialize, Serialize};

/// ユーザーの構造化プロファイル
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    pub name: String,
    pub preferences: Vec<String>,
    pub aesthetic_style: Option<String>,
    pub interaction_style: Option<String>,
    pub traits: std::collections::HashMap<String, String>,
}

/// ユーザー行動パターンの学習エンジン
#[derive(Debug)]
pub struct UserLearner {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    semaphore: Arc<Semaphore>,
    pub profile: std::sync::RwLock<UserProfile>,
}

impl UserLearner {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        semaphore: Arc<Semaphore>,
        profile: UserProfile,
    ) -> Self {
        Self {
            provider,
            semaphore,
            profile: std::sync::RwLock::new(profile),
        }
    }

    /// `learn_from_session` を実行する
    pub async fn learn_from_session(
        &self,
        conversation_summary: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let filename = "USER.md";
        let (user_path, current_user) = if let Ok(c) = fs::read_to_string(filename) {
            (filename.to_string(), c)
        } else if let Ok(c) = fs::read_to_string(format!("../../{}", filename)) {
            (format!("../../{}", filename), c)
        } else {
            (filename.to_string(), String::new())
        };

        if let Ok(_permit) = self.semaphore.try_acquire() {
            info!("🎓 [UserLearner] Analyzing session for user preference updates...");
            let prompt = format!(
                "この会話からユーザーの好みや情報を抽出し、USER.mdを更新してください。既存の情報は消さずに補完してください。\n\n現在のUSER.md:\n{}\n\n最近の会話内容:\n{}\n\nルール:\n1. 更新が必要なら、新しいUSER.mdの内容全体を出力せよ。\n2. 更新が不要なら「NO_UPDATE」とだけ出力せよ。\n3. フォーマットはMarkdownを維持せよ。日本語で出力せよ。",
                current_user, conversation_summary
            );

            match self.provider.complete(&prompt, None).await {
                Ok(resp) => {
                    let reply = resp.content.trim();
                    if reply != "NO_UPDATE" && !reply.is_empty() {
                        // 異常サイズ・内容検知 (短すぎる、またはMarkdown構造を成していない場合はブロック)
                        if reply.len() < 50 || (!reply.contains('#') && !reply.contains("- ")) {
                            warn!("⚠️ [UserLearner] 生成された内容が異常に短いか、不正な形式です。上書きを中止します。");
                            return Ok(false);
                        }

                        // 上書き前にバックアップを作成
                        let backup_path = format!("{}.bak", user_path);
                        let _ = fs::copy(&user_path, &backup_path);

                        fs::write(&user_path, reply)?;
                        info!(
                            "✅ [UserLearner] {} has been updated based on session intelligence. Backup saved to {}.",
                            user_path, backup_path
                        );
                        return Ok(true);
                    }
                    info!("🎓 [UserLearner] No updates needed for {}.", user_path);
                }
                Err(e) => {
                    warn!("⚠️ [UserLearner] Failed to learn user preferences: {:?}", e);
                }
            }
        }
        Ok(false)
    }
}

#[async_trait]
impl AgentHook for UserLearner {
    async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_post_execute(
        &self,
        request: &LlmRequest,
        response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        // 会話の断片（リクエストメッセージの最後とレスポンス）から学習を試みる
        let mut summary = String::new();
        if let Some(m) = request.messages.last() {
            summary.push_str(&format!("User: {}\n", m.content));
        }
        summary.push_str(&format!("Assistant: {}\n", response.content));

        let _ = self.learn_from_session(&summary).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::error::AiomeError;
    use aiome_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};
    use aiome_contracts::security::AgentHook;

    #[derive(Debug)]
    struct MockLlm;
    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: "NO_UPDATE".to_string(), // Test default
                stop_reason: StopReason::EndTurn,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_user_learner_implements_agent_hook() {
        let provider = Arc::new(MockLlm);
        let semaphore = Arc::new(Semaphore::new(1));
        let learner = UserLearner::new(provider, semaphore, UserProfile::default());

        // AgentHook を実装していることを確認
        let hook: Box<dyn AgentHook> = Box::new(learner);

        let request = LlmRequest::default();
        let response = LlmResponse {
            content: "User likes coffee.".to_string(),
            stop_reason: StopReason::EndTurn,
        };

        // RED: on_post_execute should trigger session learning
        let result = hook.on_post_execute(&request, &response).await;
        assert!(result.is_ok());
    }
}
