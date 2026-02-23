use factory_core::contracts::{ConceptRequest, ConceptResponse};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use async_trait::async_trait;
use rig::providers::gemini;
use rig::prelude::*;
use rig::completion::Prompt;
use tracing::{info, error};

/// 動画コンセプト生成機 (Director)
/// 
/// トレンドデータを入力として受け取り、LLM (Gemini) を使用して
/// 具体的な動画タイトル、脚本（字幕用・TTS用）、画像生成用プロンプトを生成する。
pub struct ConceptManager {
    api_key: String,
    model: String,
}

impl ConceptManager {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    fn get_client(&self) -> Result<gemini::Client, FactoryError> {
        gemini::Client::new(&self.api_key)
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Gemini Client error: {}", e) })
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
        info!("🎬 ConceptManager: Generating video concept with Gemini ({}) for topic '{}'...", self.model, input.topic);

        let client = self.get_client()?;
        let style_list = input.available_styles.join(", ");
        
        // ... (preamble construction remains same) ...
        let preamble = format!(
            "あなたは YouTube Shorts のプロフェッショナルな動画プロデューサーです。
            先端テクノロジーを愛する、知的で魅力的なナレーターとして、
            難解な最新技術を鮮やかな比喩と引き込まれる語りで伝えてください。

            【ミッション】
            与えられたトレンドキーワードに基づき、視聴者の心を一瞬で掴む動画コンセプトを提案してください。

            【絶対契約 - 二重台本アーキテクチャ】
            字幕の見栄えと発音の自然さを両立させるため、以下の2系統テキストを生成してください。
            1. display_*: 字幕表示用。英数字（OpenAI, 600億ドル）をそのまま使い、読みやすくスタイリッシュに。
            2. script_*: 音声合成用。全てひらがな・カタカナ・漢字のみで記述し、発音ミスを防止。

            【台本の構成と分量 ★最重要★】
            動画は30〜60秒。各セクションに十分な情報量が必要です。薄い台本は絶対禁止。

            ■ intro（導入 / 2〜3文）
              - 1文目: 衝撃的な事実や疑問で視聴者を引き込む「フック」
              - 2文目以降: なぜこの話題が重要なのかを端的に示す

            ■ body（本編 / 5〜7文）★ここが最も重要★
              - 具体的な数字やデータを必ず1つ以上含める
              - 「なぜそうなのか」の理由や背景を説明する
              - 身近な例え話や比喩を1つ以上使って難しい概念をわかりやすくする
              - 視聴者が「へぇ」と思う意外な事実や視点を入れる

            ■ outro（結末 / 2〜3文）
              - 話の核心を一言でまとめる
              - 視聴者への問いかけやCTA（コメント促進）で締める

            【文体ルール】
            - 語り口は「知性的だが親しみやすい」トーン。「〜なんです」「〜ですよね」を基本語尾とする。
            - 一文は短く（25文字以内目安）。リズム感を重視。
            - 三点リーダー（…）は音声合成エラーの原因になるため絶対に使用禁止。句点（。）で文を切ること。
            - script_* では英字・数字を全てカナに変換すること（例: OpenAI→オープンエーアイ、600億→ろっぴゃくおく）。

            【ビジュアルプロンプト制約 ★重要★】
            visual_prompts は、各セクション（intro, body, outro）の内容を象徴する具体的かつ詳細な英語の描写にしてください。
            - 抽象的な表現（例: \"future city\"）は避け、具体的な要素（例: \"neon-lit Tokyo street with holographic advertisements, heavy rain, 8k resolution\"）を記述すること。
            - 構図の指定（例: rule of thirds, dynamic angle, extreme close-up, dramatic perspective）を含めること。
            - ライティングの指定（例: cinematic lighting, volumetric fog, rim lighting, glowing neon）を追加し、プロの品質を確保すること。
            - Pony V6/SDXL等のモデルにおいてクオリティを引き上げる修飾語（例: hyper-detailed, masterpiece, best quality, ultra highres）を付与すること。
            - 人物を描画する場合、表情やポーズ（例: confident smile, pointing at camera）も指定すること。
            - 文脈無視の画像は絶対禁止。台本の内容と密接に関連したビジュアルを提案してください。
            - 全て英語で記述し、カンマ区切りで詳細な属性を追加してください。

            【出力形式（JSONのみ、解説やコメント禁止）】

            ```json
            {{
              \"title\": \"日本語タイトル\",
              \"display_intro\": \"...\",
              \"display_body\": \"...\",
              \"display_outro\": \"...\",
              \"script_intro\": \"...\",
              \"script_body\": \"...\",
              \"script_outro\": \"...\",
              \"common_style\": \"cinematic anime style, hyper-detailed, dramatic lighting, futuristic atmosphere\",
              \"style_profile\": \"{}\",
              \"visual_prompts\": [
                \"[intro用の詳細な描写]\",
                \"[body用の詳細な描写]\",
                \"[outro用の詳細な描写]\"
              ],
              \"metadata\": {{ \"narrator_persona\": \"tech_visionary\" }}
            }}
            ```

            上記の例は分量と構成の参考です。この程度の情報密度を必ず維持してください。",
            style_list
        );

        let agent = client.agent(&self.model)
            .preamble(&preamble)
            .temperature(0.7)
            .build();

        let trend_list = input.trend_items.iter()
            .map(|i| format!("- {} (Score: {})", i.keyword, i.score))
            .collect::<Vec<_>>()
            .join("\n");

        let user_prompt = format!("現在の重要トレンド：\n{}\n\nこの中から最も興味深い話題を選び、最高品質の動画コンセプトを生成してください。", trend_list);

        let response: String = agent.prompt(user_prompt).await
            .map_err(|e| {
                error!("Gemini Error: {}", e);
                FactoryError::Infrastructure { reason: format!("Gemini Prompt Error: {}", e) }
            })?;

        let json_text = extract_json(&response)?;
        
        let concept: ConceptResponse = serde_json::from_str(&json_text)
            .map_err(|e| {
                error!("Failed to parse Gemini response as JSON: {}. Response: {}", e, json_text);
                FactoryError::Infrastructure { reason: format!("Gemini JSON Parse Error: {}", e) }
            })?;

        info!("✅ ConceptManager: Concept generated: '{}'", concept.title);
        Ok(concept)
    }
}

/// 文字列からJSONブロックを探して抽出する
fn extract_json(text: &str) -> Result<String, FactoryError> {
    let mut clean_text = text.to_string();
    
    // 1. markdown code block: ```json ... ``` の中身を抽出
    if let Some(start_idx) = clean_text.find("```json") {
        let after_start = &clean_text[start_idx + 7..];
        if let Some(end_idx) = after_start.find("```") {
            clean_text = after_start[..end_idx].to_string();
        }
    } else if let Some(start_idx) = clean_text.find("```") {
        // フォールバック: 言語指定なしの ``` ... ``` も試す
        let after_start = &clean_text[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            clean_text = after_start[..end_idx].to_string();
        }
    }

    if let (Some(start), Some(end)) = (clean_text.find('{'), clean_text.rfind('}')) {
        let mut json_str = clean_text[start..=end].to_string();
        // Remove trailing commas before closing braces/brackets, which is a common LLM hallucination
        json_str = json_str.replace(",\n}", "\n}").replace(",}", "}").replace(",\n]", "\n]").replace(",]", "]");
        
        // 欠落したダブルクオートを修復する簡易的な処理 (LLMが先頭のクオートを忘れがち)
        // `"key": 値,` -> `"key": "値",`
        // ただし [ や { または " で始まるものは除外
        let re_missing_both = regex::Regex::new(r#""([a-zA-Z_]+)"\s*:\s*([^"\[\{\s][^",\n]+)\s*,"#).unwrap();
        json_str = re_missing_both.replace_all(&json_str, "\"$1\": \"$2\",").to_string();
        
        // 先頭だけ忘れて末尾はある場合: `"key": 値",` -> `"key": "値",`
        let re_missing_start = regex::Regex::new(r#""([a-zA-Z_]+)"\s*:\s*([^"\[\{\s][^"\n]+)","#).unwrap();
        json_str = re_missing_start.replace_all(&json_str, "\"$1\": \"$2\",").to_string();

        Ok(json_str)
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
