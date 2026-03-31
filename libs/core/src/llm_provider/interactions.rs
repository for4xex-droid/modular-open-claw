use crate::error::AiomeError;
use aiome_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};
use async_trait::async_trait;
use std::fmt::Debug;

/// Gemini Interactions API 経由の LLM プロバイダー
#[derive(Debug, Clone)]
pub struct InteractionsGeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: Option<String>,
    fallback: Option<std::sync::Arc<dyn LlmProvider>>,
}

impl InteractionsGeminiProvider {
    /// 新しい InteractionsGeminiProvider を作成します
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
        Self {
            client,
            api_key,
            model,
            base_url: None,
            fallback: None,
        }
    }

    /// ベースURLを指定して新しい InteractionsGeminiProvider を作成します（テスト用）
    pub fn with_base_url(
        client: reqwest::Client,
        api_key: String,
        model: String,
        base_url: String,
    ) -> Self {
        Self {
            client,
            api_key,
            model,
            base_url: Some(base_url),
            fallback: None,
        }
    }

    /// フェイルオーバー用のプロバイダーを設定します
    pub fn with_fallback(mut self, fallback: std::sync::Arc<dyn LlmProvider>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    async fn execute_request(&self, request: &LlmRequest) -> Result<LlmResponse, AiomeError> {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com");

        // Interactions API エンドポイント
        let url = format!("{}/v1beta/interactions", base.trim_end_matches('/'));

        // 直近のユーザーメッセージを input として抽出
        let input = request
            .messages
            .iter()
            .rfind(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        // メタデータから previous_interaction_id を取得
        let previous_id = request
            .metadata
            .as_ref()
            .and_then(|m| m.get("previous_interaction_id"));

        let mut payload = serde_json::json!({
            "model": format!("models/{}", self.model),
            "input": input,
        });

        if let Some(pid) = previous_id {
            payload["previous_interaction_id"] = serde_json::json!(pid);
        }

        // システム命令があれば追加 (Interactions API 仕様に合わせる必要あり)
        if let Some(sys) = request.messages.iter().find(|m| m.role == "system") {
            payload["system_instruction"] = serde_json::json!({
                "parts": [{ "text": sys.content }]
            });
        }

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Interactions API network error: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AiomeError::Infrastructure {
                reason: format!("Interactions API error ({}): {}", status, err_text),
            });
        }

        let res_body: serde_json::Value =
            response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to parse Interactions API response: {}", e),
                })?;

        // outputs 配列から text/thought タイプの出力を統合
        let mut content = String::new();
        let mut reasoning = String::new();
        if let Some(outputs) = res_body["outputs"].as_array() {
            for out in outputs {
                match out["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = out["text"].as_str() {
                            content.push_str(text);
                        }
                    }
                    Some("thought") | Some("reasoning") => {
                        if let Some(text) = out["text"].as_str() {
                            reasoning.push_str(text);
                        }
                    }
                    _ => {}
                }
            }
        }

        // レスポンスのメタデータに interaction_id を含める
        let mut response_metadata = std::collections::HashMap::new();
        if let Some(id) = res_body["id"].as_str() {
            response_metadata.insert("interaction_id".to_string(), id.to_string());
        }

        Ok(LlmResponse {
            content,
            stop_reason: StopReason::EndTurn, // 簡略化
            reasoning: if reasoning.is_empty() {
                None
            } else {
                Some(reasoning)
            },
            metadata: Some(response_metadata),
        })
    }
}

#[async_trait]
impl LlmProvider for InteractionsGeminiProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(aiome_contracts::llm::LlmMessage {
                role: "system".to_string(),
                content: sys.to_string(),
                cache: false,
            });
        }
        messages.push(aiome_contracts::llm::LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            cache: false,
        });

        self.complete_with_cache(LlmRequest {
            messages,
            ..Default::default()
        })
        .await
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let max_retries = 3;
        let mut retry_count = 0;
        let mut last_err = None;

        while retry_count <= max_retries {
            if retry_count > 0 {
                let delay = std::time::Duration::from_millis(500 * (1 << (retry_count - 1)));
                tokio::time::sleep(delay).await;
                tracing::warn!("Retrying Interactions API (count: {})", retry_count);
            }

            match self.execute_request(&request).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    tracing::error!("Interactions API error: {:?}", e);
                    last_err = Some(e);
                    retry_count += 1;
                }
            }
        }

        // フェイルオーバー
        if let Some(fallback) = &self.fallback {
            tracing::warn!(
                "Interactions API failed after {} retries, falling back to {}",
                max_retries,
                fallback.name()
            );
            return fallback.complete_with_cache(request).await;
        }

        Err(last_err.unwrap_or(AiomeError::Infrastructure {
            reason: "Unknown error in InteractionsGeminiProvider".into(),
        }))
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "gemini-interactions"
    }
}
