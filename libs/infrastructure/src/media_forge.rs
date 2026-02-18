//! # MediaForge — FFmpeg 動画合成エンジン
//!
//! 動画、音声、字幕ファイルを合成して最終的な作品を書き出す。
//! Bastion Jail を使用して、一時ファイルが指定された領域外に漏れるのを防ぐ。

use async_trait::async_trait;
use bastion::fs_guard::Jail;
use factory_core::contracts::{MediaRequest, MediaResponse};
use factory_core::error::FactoryError;
use factory_core::traits::{AgentAct, MediaEditor};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tracing::info;

/// FFmpeg を使用した動画編集クライアント
#[derive(Clone)]
pub struct MediaForgeClient {
    /// 作業用の Jail
    pub jail: Arc<Jail>,
}

impl MediaForgeClient {
    pub fn new(jail: Arc<Jail>) -> Self {
        Self { jail }
    }
}

#[async_trait]
impl MediaEditor for MediaForgeClient {
    async fn combine_assets(
        &self,
        video: &PathBuf,
        audio: &PathBuf,
        subtitle: Option<&PathBuf>,
    ) -> Result<PathBuf, FactoryError> {
        let output = self.jail.root().join("final_output.mp4");
        
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i").arg(video)
           .arg("-i").arg(audio);
        
        if let Some(sub) = subtitle {
            cmd.arg("-i").arg(sub);
        }

        cmd.arg("-c:v").arg("copy")
           .arg("-c:a").arg("aac")
           .arg(&output);

        tracing::info!("MediaForge: Running FFmpeg to combine assets...");
        
        let status = cmd.status().await.map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to spawn ffmpeg: {}", e),
        })?;

        if status.success() {
            Ok(output)
        } else {
            Err(FactoryError::Infrastructure {
                reason: "FFmpeg execution failed".to_string(),
            })
        }
    }

    async fn resize_for_shorts(&self, input: &PathBuf) -> Result<PathBuf, FactoryError> {
        let output = self.jail.root().join("resized_shorts.mp4");
        
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i").arg(input)
           .arg("-vf").arg("scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920")
           .arg("-c:a").arg("copy")
           .arg(&output);

        tracing::info!("MediaForge: Resizing video for YouTube Shorts (9:16)...");

        let status = cmd.status().await.map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to spawn ffmpeg: {}", e),
        })?;

        if status.success() {
            Ok(output)
        } else {
            Err(FactoryError::Infrastructure {
                reason: "FFmpeg resize failed".to_string(),
            })
        }
    }

    /// 複数の動画クリップを 1つの動画ファイルに結合する
    async fn concatenate_clips(&self, clips: Vec<String>, output_name: String) -> Result<String, FactoryError> {
        let output = self.jail.root().join(&output_name);
        info!("🎬 MediaForge: Concatenating {} clips -> {}", clips.len(), output.display());

        // concat フィルタ用の一時ファイルを作成
        // file 'path/to/clip1.mp4'
        // file 'path/to/clip2.mp4' ...
        let mut concat_list = String::new();
        for clip in clips {
            concat_list.push_str(&format!("file '{}'\n", clip));
        }

        let list_path = self.jail.root().join("concat_list.txt");
        std::fs::write(&list_path, concat_list).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to write concat list: {}", e),
        })?;

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-f").arg("concat")
            .arg("-safe").arg("0")
            .arg("-i").arg(&list_path)
            .arg("-c").arg("copy")
            .arg(&output)
            .status().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("FFmpeg concat failed: {}", e) })?;

        if status.success() {
            Ok(output.to_string_lossy().to_string())
        } else {
            Err(FactoryError::Infrastructure { reason: "FFmpeg concat execution failed".into() })
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MediaForgeArgs {
    /// 動画、音声、字幕を合成
    Combine {
        video_path: String,
        audio_path: String,
        subtitle_path: Option<String>,
    },
    /// Shorts 用にリサイズ (9:16)
    Resize {
        input_path: String,
    },
}

#[derive(Serialize)]
pub struct MediaForgeOutput {
    pub output_path: String,
}

#[async_trait]
impl AgentAct for MediaForgeClient {
    type Input = MediaRequest;
    type Output = MediaResponse;

    async fn execute(
        &self,
        input: Self::Input,
        _jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, FactoryError> {
        let path = self.combine_assets(
            &PathBuf::from(input.video_path),
            &PathBuf::from(input.audio_path),
            input.subtitle_path.as_ref().map(PathBuf::from).as_ref(),
        ).await?;
        Ok(MediaResponse {
            final_path: path.to_string_lossy().to_string(),
        })
    }
}

impl Tool for MediaForgeClient {
    const NAME: &'static str = "media_forge";
    type Args = MediaForgeArgs;
    type Output = MediaForgeOutput;
    type Error = FactoryError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "FFmpeg を使用して、動画の合成や YouTube Shorts 向けのリサイズを行います。".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(MediaForgeArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = match args {
            MediaForgeArgs::Combine { video_path, audio_path, subtitle_path } => {
                self.combine_assets(
                    &PathBuf::from(video_path),
                    &PathBuf::from(audio_path),
                    subtitle_path.as_ref().map(PathBuf::from).as_ref(),
                ).await?
            }
            MediaForgeArgs::Resize { input_path } => {
                self.resize_for_shorts(&PathBuf::from(input_path)).await?
            }
        };

        Ok(MediaForgeOutput {
            output_path: path.to_string_lossy().to_string(),
        })
    }
}
