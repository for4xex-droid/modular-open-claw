use async_trait::async_trait;
use bastion::fs_guard::Jail;
use factory_core::contracts::{MediaRequest, MediaResponse};
use factory_core::error::FactoryError;
use factory_core::traits::{AgentAct, MediaEditor};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
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
        video: &std::path::PathBuf,
        audio: &std::path::PathBuf,
        subtitle: Option<&std::path::PathBuf>,
    ) -> Result<std::path::PathBuf, FactoryError> {
        let output = self.jail.root().join("final_output.mp4");
        
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i").arg(video)
           .arg("-i").arg(audio);
        
        // 字幕の焼き込み (Hard-burn) - Grade S Design
        if let Some(sub) = subtitle {
            // FFmpeg's subtitles filter has extremely sensitive escaping.
            // On Unix-like systems (macOS), absolute paths starting with / are usually fine,
            // but for absolute robustness, we escape colons and wrap in single quotes.
            // The single-quote escape pattern in FFmpeg is: ' -> '\'' (close, escaped quote, re-open)
            let sub_path = sub.to_string_lossy()
                .replace("'", "'\\''")
                .replace(":", "\\:");
            
            // Design: High visibility yellow for SNS, thick outline, and UI-safe MarginV=140
            // We use the explicit 'filename=' key to avoid ambiguity.
            let filter = format!(
                "subtitles=filename='{}':force_style='FontName=Hiragino Sans,FontSize=18,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=2.0,Shadow=1.0,Alignment=2,MarginV=120'",
                sub_path
            );
            cmd.arg("-vf").arg(filter);
        }

        // M4 Pro 最適化: Hardware Encoder (h264_videotoolbox) 強制
        // 再エンコードが必要なため、CPU負荷を下げ速度を数倍に引き上げる
        cmd.arg("-c:v").arg("h264_videotoolbox")
           .arg("-b:v").arg("6000k") // ショート動画向けの高ビットレート
           .arg("-pix_fmt").arg("yuv420p")
           .arg("-c:a").arg("aac")
           .arg("-shortest")
           .stdin(Stdio::null())
           .arg(&output);

        tracing::info!("MediaForge: Running hardware-accelerated FFmpeg (M4 Pro) with Grade S subtitles...");
        
        let output_res = cmd.output()
           .await
           .map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to spawn ffmpeg: {}", e),
        })?;

        if output_res.status.success() {
            Ok(output)
        } else {
            let err = String::from_utf8_lossy(&output_res.stderr);
            Err(FactoryError::Infrastructure {
                reason: format!("FFmpeg execution failed: {}", err),
            })
        }
    }

    async fn resize_for_shorts(&self, input: &std::path::PathBuf) -> Result<std::path::PathBuf, FactoryError> {
        let output = self.jail.root().join("resized_shorts.mp4");
        
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i").arg(input)
           .arg("-vf").arg("scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920")
           .arg("-c:v").arg("h264_videotoolbox") // M4 Pro 最適化
           .arg("-b:v").arg("8000k")
           .arg("-pix_fmt").arg("yuv420p")
           .arg("-c:a").arg("copy")
           .stdin(Stdio::null())
           .arg(&output);

        tracing::info!("MediaForge: Resizing video (Hardware Accelerated)...");
        let output_res = cmd.output()
           .await
           .map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to spawn ffmpeg: {}", e),
        })?;

        if output_res.status.success() {
            Ok(output)
        } else {
            let err = String::from_utf8_lossy(&output_res.stderr);
            Err(FactoryError::Infrastructure {
                reason: format!("FFmpeg resize failed: {}", err),
            })
        }
    }

    /// 複数の動画クリップを 1つの動画ファイルに結合する
    async fn concatenate_clips(&self, clips: Vec<String>, output_name: String) -> Result<String, FactoryError> {
        let output = self.jail.root().join(&output_name);
        info!("🎬 MediaForge: Concatenating {} clips -> {}", clips.len(), output.display());

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
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("FFmpeg concat failed: {}", e) })?;

        if status.success() {
            Ok(output.to_string_lossy().to_string())
        } else {
            Err(FactoryError::Infrastructure { reason: "FFmpeg concat execution failed".into() })
        }
    }

    async fn get_duration(&self, path: &std::path::Path) -> Result<f32, FactoryError> {
        let output = Command::new("ffprobe")
            .arg("-v").arg("error")
            .arg("-show_entries").arg("format=duration")
            .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
            .arg(path)
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("ffprobe duration failed: {}", e) })?;

        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        s.parse::<f32>().map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to parse duration '{}': {}", s, e) })
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
