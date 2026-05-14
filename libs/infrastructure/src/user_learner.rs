/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::security::AgentHook;
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
    slm_bridge: Option<Arc<crate::slm_bridge::SlmBridge>>,
    pub profile: std::sync::RwLock<UserProfile>,
}

impl UserLearner {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        semaphore: Arc<Semaphore>,
        profile: UserProfile,
        slm_bridge: Option<Arc<crate::slm_bridge::SlmBridge>>,
    ) -> Self {
        Self {
            provider,
            semaphore,
            slm_bridge,
            profile: std::sync::RwLock::new(profile),
        }
    }

    /// `learn_from_session` を実行する
    pub async fn learn_from_session(
        &self,
        conversation_summary: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let resolver = shared::app_data::AppDataResolver::new()
            .map_err(|e| format!("AppDataResolver init failed: {}", e))?;
        let user_path = resolver.resolve("USER.md").to_string_lossy().to_string();
        let current_user = std::fs::read_to_string(&user_path).unwrap_or_default();

        if let Ok(_permit) = self.semaphore.try_acquire() {
            info!("🎓 [UserLearner] Analyzing session for user preference updates...");

            let system_prompt = r##"
ユーザーとの会話から、ユーザーのプロフィール（名前、好み、美的スタイル、対話スタイル、性格的特徴）を抽出し、JSON形式で返してください。
また、人間が読みやすい形式の USER.md 用のコンテンツも生成してください。
更新が必要ない場合は、単に "NO_UPDATE" と返してください。

JSONフォーマット:
{
  "name": "ユーザー名",
  "preferences": ["好み1", "好み2"],
  "aesthetic_style": "視覚的スタイルの好み",
  "interaction_style": "対話のトーンやスタイル",
  "traits": {"特徴1": "説明1"},
  "markdown_content": "# User Profile\n\n..."
}
"##;

            let prompt = format!(
                "現在のプロフィール:\n{}\n\n最近の会話内容:\n{}\n\n上記に基づき、更新された情報をJSONで出力せよ。未判明の項目は現在の値を維持せよ。",
                current_user, conversation_summary
            );

            match self.provider.complete(&prompt, Some(system_prompt)).await {
                Ok(resp) => {
                    let reply = resp.content.trim();
                    if reply == "NO_UPDATE" || reply.is_empty() {
                        info!("🎓 [UserLearner] No updates needed.");
                        return Ok(false);
                    }

                    // SEC: LLM output size limit (64KB) to prevent disk exhaustion
                    const MAX_LLM_OUTPUT_SIZE: usize = 64 * 1024;
                    if reply.len() > MAX_LLM_OUTPUT_SIZE {
                        warn!(
                            "⚠️ [UserLearner] LLM output too large ({} bytes), skipping",
                            reply.len()
                        );
                        return Ok(false);
                    }

                    // JSON パースの試行
                    // LLM が Markdown のコードブロックで囲んでくる場合があるため、トリミング
                    let trimmed = reply.trim();
                    let json_str = trimmed
                        .strip_prefix("```json")
                        .or_else(|| trimmed.strip_prefix("```"))
                        .and_then(|s| s.strip_suffix("```"))
                        .unwrap_or(trimmed)
                        .trim();

                    #[derive(Deserialize)]
                    struct LlmUpdate {
                        name: Option<String>,
                        preferences: Option<Vec<String>>,
                        aesthetic_style: Option<String>,
                        interaction_style: Option<String>,
                        traits: Option<std::collections::HashMap<String, String>>,
                        markdown_content: Option<String>,
                    }

                    if let Ok(update) = serde_json::from_str::<LlmUpdate>(json_str) {
                        let mut profile =
                            self.profile.write().map_err(|_| "Failed to lock profile")?;

                        if let Some(n) = update.name {
                            profile.name = n;
                        }
                        if let Some(p) = update.preferences {
                            profile.preferences = p;
                        }
                        if let Some(a) = update.aesthetic_style {
                            profile.aesthetic_style = Some(a);
                        }
                        if let Some(i) = update.interaction_style {
                            profile.interaction_style = Some(i);
                        }
                        if let Some(t) = update.traits {
                            profile.traits = t;
                        }

                        if let Some(md) = update.markdown_content {
                            // バックアップと保存
                            let backup_path = format!("{}.bak", user_path);
                            if let Err(e) = fs::copy(&user_path, &backup_path) {
                                warn!("⚠️ [UserLearner] Failed to backup USER.md: {:?}", e);
                            }
                            fs::write(&user_path, md)?;
                            info!("✅ [UserLearner] USER.md and structured profile updated.");
                        }
                        return Ok(true);
                    } else {
                        // フォールバック: 以前の Markdown 直接更新ロジック (念のため)
                        if reply.len() > 50
                            && (reply.contains('#') || reply.contains("- "))
                            && !reply.contains('{')
                        {
                            let backup_path = format!("{}.bak", user_path);
                            if let Err(e) = fs::copy(&user_path, &backup_path) {
                                warn!(
                                    "⚠️ [UserLearner] Failed to backup USER.md (legacy): {:?}",
                                    e
                                );
                            }
                            fs::write(&user_path, reply)?;
                            info!("✅ [UserLearner] USER.md updated (legacy fallback).");
                            return Ok(true);
                        }
                        warn!(
                            "⚠️ [UserLearner] Failed to parse LLM update: {}",
                            &reply[..reply.len().min(200)]
                        );
                    }
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

        if let Err(e) = self.learn_from_session(&summary).await {
            warn!("⚠️ [UserLearner] on_post_execute learning failed: {:?}", e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};
    use aiome_core_contracts::security::AgentHook;

    #[derive(Debug)]
    struct MockLlm {
        response_json: String,
    }

    impl MockLlm {
        fn new(json: &str) -> Self {
            Self {
                response_json: json.to_string(),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.response_json.clone(),
                stop_reason: StopReason::EndTurn,
                ..Default::default()
            })
        }

        async fn complete_with_cache(
            &self,
            _req: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }

        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }

        fn name(&self) -> &str {
            "mock-llm"
        }

        async fn stream_complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<
            std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
            AiomeError,
        > {
            Err(AiomeError::Infrastructure {
                reason: "streaming not supported in mock".into(),
            })
        }
    }

    #[tokio::test]
    async fn test_user_learner_implements_agent_hook() {
        let json = r##"{"name": "Alice", "markdown_content": "# User Profile"}"##;
        let provider = Arc::new(MockLlm::new(json));
        let semaphore = Arc::new(Semaphore::new(1));
        let learner = UserLearner::new(provider, semaphore, UserProfile::default(), None);

        // AgentHook を実装していることを確認
        let hook: Box<dyn AgentHook> = Box::new(learner);

        let request = LlmRequest::default();
        let response = LlmResponse {
            content: "[]".to_string(),
            stop_reason: StopReason::EndTurn,
            ..Default::default()
        };

        let result = hook.on_post_execute(&request, &response).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_user_learner_updates_structured_profile() {
        let json = r##"{
            "name": "Alice",
            "preferences": ["rust", "security"],
            "aesthetic_style": "Dark",
            "interaction_style": "Friendly",
            "traits": {"curiosity": "high"},
            "markdown_content": "# User Profile\n\n- Name: Alice"
        }"##;

        let provider = Arc::new(MockLlm::new(json));
        let semaphore = Arc::new(Semaphore::new(1));
        let learner = UserLearner::new(provider, semaphore, UserProfile::default(), None);

        // Act: 会話から学習を実行
        let _ = learner.learn_from_session("User likes Rust.").await;

        // Assert: 構造化プロファイルが更新されていることを確認
        let profile = learner.profile.read().unwrap();
        assert_eq!(profile.name, "Alice");
        assert!(profile.preferences.contains(&"rust".to_string()));
        assert_eq!(profile.aesthetic_style.as_deref(), Some("Dark"));
        assert_eq!(
            profile.traits.get("curiosity").map(|s| s.as_str()),
            Some("high")
        );
    }
}
