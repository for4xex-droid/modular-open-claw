use serde::{Deserialize, Serialize};

/// ShortsFactory 全体の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    /// Ollama API エンドポイント
    pub ollama_url: String,
    /// ComfyUI REST/WebSocket API エンドポイント
    pub comfyui_api_url: String,
    /// バッチサイズ（一括企画する動画の本数）
    pub batch_size: usize,
    /// ComfyUI タイムアウト（秒）
    pub comfyui_timeout_secs: u64,
    /// 本番用モデル名
    pub model_name: String,
    /// ComfyUI のベースディレクトリ (Zero-Copy)
    pub comfyui_base_dir: String,
}

impl FactoryConfig {
    /// 設定をファイルまたは環境変数から読み込む
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            // デフォルト値の設定
            .set_default("ollama_url", "http://localhost:11434/v1")?
            .set_default("comfyui_api_url", std::env::var("COMFYUI_API_URL").unwrap_or_else(|_| "ws://127.0.0.1:8188/ws".to_string()))?
            .set_default("batch_size", 10)?
            .set_default("comfyui_timeout_secs", 180)?
            .set_default("model_name", "qwen2.5-coder:32b")?
            .set_default("comfyui_base_dir", std::env::var("COMFYUI_BASE_DIR").unwrap_or_else(|_| "/Users/motista/Desktop/ComfyUI".to_string()))?
            // config.toml があれば読み込む
            .add_source(config::File::with_name("config").required(false))
            // 環境変数 (SHORTS_FACTORY_*) があれば上書き
            .add_source(config::Environment::with_prefix("SHORTS_FACTORY"))
            .build()?;

        settings.try_deserialize()
    }
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self::load().unwrap_or_else(|_| {
            Self {
                ollama_url: "http://localhost:11434/v1".to_string(),
                comfyui_api_url: std::env::var("COMFYUI_API_URL").unwrap_or_else(|_| "ws://127.0.0.1:8188/ws".to_string()),
                batch_size: 10,
                comfyui_timeout_secs: 180,
                model_name: "qwen2.5-coder:32b".to_string(),
                comfyui_base_dir: std::env::var("COMFYUI_BASE_DIR").unwrap_or_else(|_| "/Users/motista/Desktop/ComfyUI".to_string()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_load_defaults() {
        let config = FactoryConfig::default();
        assert_eq!(config.ollama_url, "http://localhost:11434/v1");
        assert_eq!(config.model_name, "qwen2.5-coder:32b");
    }

    #[test]
    fn test_config_load_from_file() {
        // 一時的な config.toml を作成 (toml 拡張子を付加してフォーマットを認識させる)
        let mut file = tempfile::Builder::new()
            .suffix(".toml")
            .tempfile()
            .unwrap();
        writeln!(file, "ollama_url = \"http://custom:11434/v1\"").unwrap();
        writeln!(file, "comfyui_api_url = \"ws://custom:8188/ws\"").unwrap();
        writeln!(file, "batch_size = 5").unwrap();
        writeln!(file, "comfyui_timeout_secs = 60").unwrap();
        writeln!(file, "model_name = \"custom-model\"").unwrap();
        writeln!(file, "comfyui_base_dir = \"custom_dir\"").unwrap();
        
        // config::File::from(path) を使って明示的なファイルを読み込む
        // 拡張子があるためフォーマットは自動判別される
        let settings = config::Config::builder()
            .add_source(config::File::from(file.path()))
            .build()
            .unwrap();
        
        let config: FactoryConfig = settings.try_deserialize().unwrap();
        assert_eq!(config.ollama_url, "http://custom:11434/v1");
        assert_eq!(config.model_name, "custom-model");
    }
}
