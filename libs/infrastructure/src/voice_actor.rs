use factory_core::contracts::{VoiceRequest, VoiceResponse};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use async_trait::async_trait;
use tracing::{info, error};
use std::path::Path;
use std::time::Duration;

/// 音声合成アクター (Qwen3-TTS Client)
///
/// Qwen3-TTS の OpenAI互換 /v1/audio/speech エンドポイントにリクエストを送信し、
/// 生成された音声（WAV）を Jail 内に保存する。
///
/// 【設計思想】
/// テキストを句点（。）単位で分割し、各文を個別にTTS合成する。
/// 合成された各音声ファイルを FFmpeg で結合し、文間に 0.15秒の無音を挿入する。
/// TTS サーバー側で末尾トリミングを行い、ハルシネーション（余分な音声）を防止する。
pub struct VoiceActor {
    server_url: String,
    default_voice: String,
    client: reqwest::Client,
}

impl VoiceActor {
    pub fn new(server_url: &str, default_voice: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            default_voice: default_voice.to_string(),
            client,
        }
    }

    /// テキスト浄化パイプライン
    fn sanitize_for_tts(text: &str) -> String {
        let mut t = String::with_capacity(text.len());

        // 1. 制御文字・絵文字の除去
        for c in text.chars() {
            if c.is_control() && c != '\n' {
                continue;
            }
            let cp = c as u32;
            if (0x1F600..=0x1F64F).contains(&cp)
                || (0x1F300..=0x1F5FF).contains(&cp)
                || (0x1F680..=0x1F6FF).contains(&cp)
                || (0x1F900..=0x1F9FF).contains(&cp)
                || (0x2600..=0x26FF).contains(&cp)
                || (0x2700..=0x27BF).contains(&cp)
                || (0xFE00..=0xFE0F).contains(&cp)
                || (0x200D..=0x200D).contains(&cp)
            {
                continue;
            }
            t.push(c);
        }

        // 2. 三点リーダーの除去
        t = t.replace("…", "、")
             .replace("...", "、")
             .replace("..", "、");

        // 3. 連続空白・句読点の正規化
        while t.contains("  ") { t = t.replace("  ", " "); }
        while t.contains("。。") { t = t.replace("。。", "。"); }
        while t.contains("、、") { t = t.replace("、、", "、"); }
        t = t.replace("、。", "。");

        t.trim().to_string()
    }

    /// テキストを文単位で分割する
    fn split_into_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();

        for c in text.chars() {
            current.push(c);
            if c == '。' || c == '？' || c == '！' {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    sentences.push(s);
                }
                current.clear();
            }
        }

        let remaining = current.trim().to_string();
        if !remaining.is_empty() {
            sentences.push(remaining);
        }

        if sentences.is_empty() && !text.trim().is_empty() {
            sentences.push(text.trim().to_string());
        }

        sentences
    }
}

#[async_trait]
impl AgentAct for VoiceActor {
    type Input = VoiceRequest;
    type Output = VoiceResponse;

    async fn execute(
        &self,
        input: Self::Input,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, FactoryError> {
        let sanitized_text = Self::sanitize_for_tts(&input.text);
        if sanitized_text.is_empty() {
            return Err(FactoryError::TtsFailure {
                reason: "Sanitized text is empty.".into(),
            });
        }

        let voice = if input.voice.is_empty() {
            self.default_voice.clone()
        } else {
            input.voice.clone()
        };

        info!(
            "🗣️ VoiceActor: Synthesizing full text with voice '{}' for: '{}'",
            voice,
            sanitized_text.chars().take(80).collect::<String>()
        );

        let url = format!("{}/v1/audio/speech", self.server_url);

        let mut body = serde_json::json!({
            "input": sanitized_text,
            "voice": voice,
            "response_format": "wav",
        });

        if let Some(s) = input.speed {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("speed".into(), serde_json::json!(s));
            }
        }

        let response = self.client.post(&url).json(&body).send().await
            .map_err(|e| FactoryError::TtsFailure {
                reason: format!("Failed to connect to TTS: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().await.unwrap_or_default();
            error!("TTS Server Error [{}]: {}", status, err_body);
            return Err(FactoryError::TtsFailure {
                reason: format!("TTS Server Error [{}]: {}", status, err_body),
            });
        }

        let audio_bytes = response.bytes().await
            .map_err(|e| FactoryError::TtsFailure {
                reason: format!("Failed to read data: {}", e),
            })?;

        let output_filename = format!("voice_{}.wav", uuid::Uuid::new_v4());
        let output_relative = Path::new("assets/audio").join(&output_filename);
        jail.create_dir_all("assets/audio").map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to create audio directory: {}", e),
        })?;
        let output_abs = jail.root().join(&output_relative);

        std::fs::write(&output_abs, &audio_bytes)
            .map_err(|e| FactoryError::Infrastructure {
                reason: format!("Failed to write audio: {}", e),
            })?;

        info!("✅ VoiceActor: Synthesis completed: {}", output_relative.display());
        Ok(VoiceResponse {
            audio_path: output_relative.to_str().unwrap_or_default().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_for_tts() {
        let t = VoiceActor::sanitize_for_tts("テスト🎉です😊");
        assert_eq!(t, "テストです");
    }

    #[test]
    fn test_sanitize_removes_ellipsis() {
        let t = VoiceActor::sanitize_for_tts("未来は…ここにある。");
        assert_eq!(t, "未来は、ここにある。");
    }

    #[test]
    fn test_sanitize_normalizes_punctuation() {
        let t = VoiceActor::sanitize_for_tts("テスト。。重複。");
        assert_eq!(t, "テスト。重複。");
    }

    #[test]
    fn test_split_into_sentences() {
        let sentences = VoiceActor::split_into_sentences("最初の文です。二番目の文です。最後です。");
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "最初の文です。");
        assert_eq!(sentences[1], "二番目の文です。");
        assert_eq!(sentences[2], "最後です。");
    }

    #[test]
    fn test_split_question_marks() {
        let sentences = VoiceActor::split_into_sentences("なぜですか？理由はこれです。");
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0], "なぜですか？");
    }
}
