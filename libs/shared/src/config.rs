use serde::{Deserialize, Serialize};

/// ShortsFactory 全体の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    /// Ollama API エンドポイント
    pub ollama_url: String,
    /// ComfyUI API エンドポイント
    pub comfyui_url: String,
    /// バッチサイズ（一括企画する動画の本数）
    pub batch_size: usize,
    /// ComfyUI タイムアウト（秒）
    pub comfyui_timeout_secs: u64,
    /// 本番用モデル名
    pub model_name: String,
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434/v1".to_string(),
            comfyui_url: "http://127.0.0.1:8188".to_string(),
            batch_size: 10,
            comfyui_timeout_secs: 180,
            model_name: "qwen2.5-coder:32b".to_string(),
        }
    }
}
