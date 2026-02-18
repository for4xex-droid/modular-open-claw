//! # ComfyBridge — ComfyUI API クライアント
//!
//! ComfyUI REST API と通信し、画像/動画生成ワークフローを実行する。
//! Bastion ShieldClient を使用して、SSRF や DNS Rebinding を防止する。

use async_trait::async_trait;
use bastion::net_guard::ShieldClient;
use factory_core::error::FactoryError;
use factory_core::traits::VideoGenerator;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// ComfyUI API クライアント
#[derive(Clone)]
pub struct ComfyBridgeClient {
    /// Bastion ネットワークシールド
    pub shield: Arc<ShieldClient>,
    /// ComfyUI の API エンドポイント
    pub base_url: String,
    /// タイムアウト（秒）
    pub timeout_secs: u64,
}

impl ComfyBridgeClient {
    pub fn new(shield: Arc<ShieldClient>, base_url: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            shield,
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
        // TODO: Phase 2 続きで実装
        tracing::warn!("ComfyBridge: generate_video はまだスタブです");
        Err(FactoryError::ComfyWorkflowFailed {
            reason: "Not implemented yet (Phase 2)".to_string(),
        })
    }

    async fn health_check(&self) -> Result<bool, FactoryError> {
        let url = format!("{}/system_stats", self.base_url);
        match self.shield.get(&url).await {
            Ok(res) => Ok(res.status().is_success()),
            Err(e) => Err(FactoryError::ComfyConnection {
                url: self.base_url.clone(),
                source: e.into(),
            }),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct ComfyArgs {
    /// 動画のプロンプト
    pub prompt: String,
    /// 使用するワークフローID
    pub workflow_id: String,
}

#[derive(Serialize)]
pub struct ComfyOutput {
    /// 生成されたファイルの保存パス
    pub output_path: String,
}

impl Tool for ComfyBridgeClient {
    const NAME: &'static str = "comfy_bridge";
    type Args = ComfyArgs;
    type Output = ComfyOutput;
    type Error = FactoryError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "ComfyUI を使用して、プロンプトに基づいた画像や動画を生成します。".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ComfyArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = self.generate_video(&args.prompt, &args.workflow_id).await?;
        Ok(ComfyOutput {
            output_path: path.to_string_lossy().to_string(),
        })
    }
}
