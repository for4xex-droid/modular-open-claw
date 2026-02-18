//! # ComfyBridge — ComfyUI API クライアント
//!
//! ComfyUI REST API と通信し、画像/動画生成ワークフローを実行する。
//! Bastion ShieldClient を使用して、SSRF や DNS Rebinding を防止する。

use async_trait::async_trait;
use bastion::net_guard::ShieldClient;
use factory_core::contracts::{VideoRequest, VideoResponse};
use factory_core::error::FactoryError;
use factory_core::traits::{AgentAct, VideoGenerator};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::info;
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

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ComfyArgs {
    /// 動画のプロンプト
    pub prompt: String,
    /// 使用するワークフローID
    pub workflow_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ComfyOutput {
    /// 生成されたファイルの保存パス
    pub output_path: String,
}

#[async_trait]
impl AgentAct for ComfyBridgeClient {
    type Input = VideoRequest;
    type Output = VideoResponse;

    async fn execute(
        &self,
        input: Self::Input,
        _jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, FactoryError> {
        let path = self.generate_video(&input.prompt, &input.workflow_id).await?;
        Ok(VideoResponse {
            output_path: path.to_string_lossy().to_string(),
        })
    }
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

impl ComfyBridgeClient {
    /// 静止画に対して Ken Burns エフェクト (Pan & Zoom) を適用し、滑らかな動画クリップを生成する
    /// VE-01: 数学的なイージング関数による脱カクつき実装
    /// 静止画に対して Ken Burns エフェクト (Pan & Zoom) を適用し、滑らかな動画クリップを生成する
    /// VE-01: 数学的なイージング関数による脱カクつき実装
    pub async fn apply_ken_burns_effect(
        &self,
        image_path: &std::path::Path,
        duration_secs: f32,
        _jail: &bastion::fs_guard::Jail,
        style: &tuning::StyleProfile,
    ) -> Result<PathBuf, FactoryError> {
        let output_path = image_path.with_extension("mp4");
        info!("🎥 ComfyBridge: Applying Ken Burns effect (Style: {}) -> {}", style.name, output_path.display());

        // Polish: 30fps で 5秒間のズーム。
        // zoom='1 + zoom_speed * sin(...)': スタイルに応じた速度でサインカーブを描く
        // 30fps * duration_secs = total_frames
        let total_frames = (30.0 * duration_secs) as usize;
        let zoom_expr = format!("1+{}*sin(on/{}*3.14159/2)", style.zoom_speed * 100.0, total_frames); 
        
        let filter = format!(
            "zoompan=z='{}':d={}:s=1920x1080:fps=30,format=yuv420p",
            zoom_expr, total_frames
        );

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-loop").arg("1")
            .arg("-i").arg(image_path)
            .arg("-vf").arg(filter)
            .arg("-c:v").arg("libx264")
            .arg("-t").arg(duration_secs.to_string())
            .arg("-pix_fmt").arg("yuv420p")
            .arg(&output_path)
            .status()
            .map_err(|e| FactoryError::Infrastructure { reason: format!("FFmpeg execution failed: {}", e) })?;

        if !status.success() {
            return Err(FactoryError::Infrastructure { reason: "FFmpeg failed to apply Ken Burns effect".into() });
        }

        Ok(output_path)
    }
}

use std::process::Command;
