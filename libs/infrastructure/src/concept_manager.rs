use factory_core::contracts::{ConceptRequest, ConceptResponse};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use async_trait::async_trait;
use rig::providers::openai;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use tracing::{info, warn, error};

/// 動画コンセプト生成機 (Director)
/// 
/// トレンドデータを入力として受け取り、LLMを使用して
/// 具体的な動画タイトル、脚本、画像生成用プロンプトを生成する。
pub struct ConceptManager {
    url: String,
    model: String,
}

impl ConceptManager {
    pub fn new(api_base: &str, model: &str) -> Self {
        Self {
            url: api_base.to_string(),
            model: model.to_string(),
        }
    }

    fn get_client(&self) -> Result<openai::Client, FactoryError> {
        openai::Client::builder()
            .api_key("ollama")
            .base_url(&self.url)
            .build()
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to build LLM client: {}", e) })
    }
}

#[async_trait]
impl AgentAct for ConceptManager {
    type Input = ConceptRequest;
    type Output = ConceptResponse;

    async fn execute(
        &self,
        input: Self::Input,
        _jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, FactoryError> {
        info!("🎬 ConceptManager: Generating video concept from {} trends...", input.trend_items.len());

        let client = self.get_client()?;
        let agent = client.agent(&self.model)
            .preamble("あなたは YouTube Shorts のプロフェッショナルな動画プロデューサーです。
            与えられたトレンドキーワードに基づき、視聴者の目を引く動画コンセプトを1つ提案してください。
            
            以下の条件（3幕構成・構造化プロンプト）を厳守してください：
            1. 出力は純粋な JSON フォーマットのみとし、他のテキストを含めない。
            2. JSON は以下のキーを持つこと：
               - 'title': 動画のタイトル (日本語)
               - 'script_intro': 導入部（3〜5秒）の脚本 (日本語)
               - 'script_body': 本編（15〜45秒）の脚本 (日本語)
               - 'script_outro': 結末・オチ（5〜10秒）の脚本 (日本語)
               - 'common_style': 全シーン共通の画風、ライティング、特定のキャラクター指定 (英語)
               - 'visual_prompts': 導入、本編、結末の各シーンに対応するアクションや背景描写（英語、必ず3件）
               - 'metadata': その他の設定 (HashMap<String, String>)
            3. 視聴維持率を高めるため、各パートは起承転結を意識し、視覚的な変化が伝わるように描写してください。")
            .build();

        let trend_list = input.trend_items.iter()
            .map(|i| format!("- {} (Score: {})", i.keyword, i.score))
            .collect::<Vec<_>>()
            .join("\n");

        let user_prompt = format!("トレンドリスト：\n{}\n\n動画コンセプトを生成してください。", trend_list);

        let result = match agent.prompt(user_prompt).await {
            Ok(response) => {
                // JSON のみを抽出
                let json_text = extract_json(&response)?;
                
                let concept: ConceptResponse = serde_json::from_str(&json_text)
                    .map_err(|e| {
                        error!("Failed to parse LLM response as JSON: {}. Response: {}", e, json_text);
                        FactoryError::Infrastructure { reason: format!("LLM JSON Parse Error: {}", e) }
                    })?;

                info!("✅ ConceptManager: Concept generated: '{}'", concept.title);
                Ok(concept)
            }
            Err(e) => {
                error!("LLM Error: {}", e);
                Err(FactoryError::Infrastructure { reason: format!("LLM Prompt Error: {}", e) })
            }
        };

        // VRAM 解放プロトコル (keep_alive: 0)
        // rig-core の背後にある Ollama に直接アンロードを指示
        if let Err(e) = self.unload_model().await {
            warn!("⚠️ ConceptManager: Failed to unload model: {}", e);
        }

        result
    }
}

impl ConceptManager {
    /// Ollama からモデルを即時アンロードし、VRAM を解放する
    async fn unload_model(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🧹 ConceptManager: Releasing VRAM (keep_alive: 0)...");
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "keep_alive": 0
        });

        // /v1/chat/completions ではなく、Ollama 自体の /api/generate を叩く必要がある場合が多い
        // api_base が http://.../v1 の場合は、/v1 を除いたベースURLを取得
        let base_url = self.url.trim_end_matches("/v1");
        let unload_url = format!("{}/api/generate", base_url);

        client.post(unload_url)
            .json(&body)
            .send()
            .await?;

        Ok(())
    }
}

/// 文字列からJSONブロックを探して抽出する
fn extract_json(text: &str) -> Result<String, FactoryError> {
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        Ok(text[start..=end].to_string())
    } else {
        Err(FactoryError::Infrastructure { reason: "LLM response did not contain JSON".into() })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block() {
        let text = "Here is the result: {\"title\": \"test\"} Hope you like it.";
        let result = extract_json(text).unwrap();
        assert_eq!(result, "{\"title\": \"test\"}");
    }

    #[test]
    fn test_extract_json_no_block() {
        let text = "There is no json here";
        let result = extract_json(text);
        assert!(result.is_err());
    }
}
