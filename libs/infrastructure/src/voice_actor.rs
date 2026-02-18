use factory_core::contracts::{VoiceRequest, VoiceResponse};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use bastion::fs_guard::Jail;
use async_trait::async_trait;
use tracing::{info, error};
use std::path::Path;

/// 音声合成アクター (Style-Bert-VITS2 Client)
pub struct VoiceActor {
    server_url: String,
    model_name: String,
}

impl VoiceActor {
    pub fn new(server_url: &str, model_name: &str) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            model_name: model_name.to_string(),
        }
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
        info!("🗣️ VoiceActor: Synthesizing voice for text: '{}'...", input.text);

        let client = reqwest::Client::new();
        let url = format!("{}/voice", self.server_url);

        let query = [
            ("text", input.text),
            ("model_name", self.model_name.clone()),
            ("speaker_id", input.speaker_id.to_string()),
            ("style", input.style.unwrap_or_else(|| "Neutral".to_string())),
            ("save_audio", "false".to_string()), // サーバー側には保存させずバイナリを取得
        ];

        let response = client.post(&url)
            .query(&query)
            .send()
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to connect to TTS server: {}", e) })?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            error!("TTS Server Error: {}", err_text);
            return Err(FactoryError::Infrastructure { reason: format!("TTS Server Error: {}", err_text) });
        }

        let audio_data = response.bytes().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to read audio data: {}", e) })?;

        // 音声ファイルの保存 (Jail 内)
        let filename = format!("voice_{}.wav", uuid::Uuid::new_v4());
        let relative_path = Path::new("assets/audio").join(&filename);
        
        // ディレクトリ作成
        jail.create_dir_all("assets/audio")
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create audio directory: {}", e) })?;

        jail.write(&relative_path, &audio_data)
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to save audio file: {}", e) })?;

        let audio_path = relative_path.to_str().unwrap_or_default().to_string();
        info!("✅ VoiceActor: Voice synthesis completed: {}", audio_path);

        Ok(VoiceResponse { audio_path })
    }
}
