//! # ComfyBridge — ComfyUI API クライアント (スタブ)
//!
//! ComfyUI REST API (`http://127.0.0.1:8188`) と通信し、
//! 画像/動画生成ワークフローを実行する。
//!
//! Phase 2 で完全実装する予定。現在はスタブのみ。

use async_trait::async_trait;
use factory_core::error::FactoryError;
use factory_core::traits::VideoGenerator;
use std::path::PathBuf;

/// ComfyUI API クライアント
#[derive(Debug, Clone)]
pub struct ComfyBridgeClient {
    /// ComfyUI の API エンドポイント
    pub base_url: String,
    /// タイムアウト（秒）
    pub timeout_secs: u64,
}

impl ComfyBridgeClient {
    pub fn new(base_url: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            base_url: base_url.into(),
            timeout_secs,
        }
    }
}

#[async_trait]
impl VideoGenerator for ComfyBridgeClient {
    async fn generate_video(
        &self,
        _prompt: &str,
        _workflow_id: &str,
    ) -> Result<PathBuf, FactoryError> {
        // TODO: Phase 2 で実装
        // 1. ワークフロー JSON を読み込み
        // 2. プロンプトを埋め込み
        // 3. POST /prompt で実行
        // 4. WebSocket で進捗監視
        // 5. GET /history で結果取得
        tracing::warn!("ComfyBridge: generate_video はまだスタブです");
        Err(FactoryError::ComfyWorkflowFailed {
            reason: "Not implemented yet (Phase 2)".to_string(),
        })
    }

    async fn health_check(&self) -> Result<bool, FactoryError> {
        // ComfyUI の /system_stats エンドポイントに GET
        let url = format!("{}/system_stats", self.base_url);
        match reqwest::get(&url).await {
            Ok(res) => Ok(res.status().is_success()),
            Err(e) => Err(FactoryError::ComfyConnection {
                url: self.base_url.clone(),
                source: e.into(),
            }),
        }
    }
}
