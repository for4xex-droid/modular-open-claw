use factory_core::error::FactoryError;
use std::path::{Path, PathBuf};
use tracing::info;
use tokio::process::Command;
use std::process::Stdio;

/// プロフェッショナル・オーディオ合成機 ("The Sound Mixer")
pub struct SoundMixer {
    bgm_library_path: PathBuf,
}

impl SoundMixer {
    pub fn new(bgm_library_path: PathBuf) -> Self {
        Self { bgm_library_path }
    }

    /// ナレーション、BGM、効果音をミキシングし、完パケ音声を生成する
    pub async fn mix_and_finalize(
        &self,
        narration_path: &Path,
        category: &str,
        output_path: &Path,
        style: &tuning::StyleProfile,
    ) -> Result<PathBuf, FactoryError> {
        info!("🎶 SoundMixer: Mixing narration with BGM (Style: {})...", style.name);
        let output = output_path.to_path_buf();

        // 1. BGM 選択
        let bgm_path = self.select_bgm(category).await?;
        
        // ナレーションの長さを取得 (秒)
        let duration = self.get_audio_duration(narration_path).await?;
        
        // 2. FFmpeg Complex Filter の構築
        let filter = format!(
            "[1:a]aloop=loop=-1:size=2e+09[bgm]; \
             [bgm][0:a]sidechaincompress=threshold={}:ratio=20:attack=10:release=200[bgm_ducked]; \
             [0:a][bgm_ducked]amix=inputs=2:weights=1.0 {}:duration=first:normalize=0[out]; \
             [out]loudnorm=I=-14:LRA=11:TP=-1.5[final]",
            style.ducking_threshold,
            style.ducking_ratio,
        );

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(narration_path)
            .arg("-i").arg(bgm_path)
            .arg("-filter_complex").arg(filter)
            .arg("-map").arg("[final]")
            .arg("-t").arg(duration.to_string())
            .arg(output_path)
            .stdin(Stdio::null())
            .stderr(Stdio::null()) // 防止: デッドロック (Pipe Buffer Full)
            .status()
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("FFmpeg mixer failed to spawn: {}", e) })?;

        if status.success() {
            info!("✅ SoundMixer: Finalized audio written to {}", output_path.display());
            Ok(output)
        } else {
            Err(FactoryError::Infrastructure { reason: "FFmpeg mixer execution failed".into() })
        }
    }

    async fn select_bgm(&self, category: &str) -> Result<PathBuf, FactoryError> {
        let category_bgm = self.bgm_library_path.join(format!("{}.mp3", category));
        if category_bgm.exists() {
            Ok(category_bgm)
        } else {
            let default_bgm = self.bgm_library_path.join("default.mp3");
            if default_bgm.exists() {
                Ok(default_bgm)
            } else {
                Err(FactoryError::MediaNotFound { path: "default.mp3".into() })
            }
        }
    }

    async fn get_audio_duration(&self, path: &Path) -> Result<f32, FactoryError> {
        let output = Command::new("ffprobe")
            .arg("-v").arg("error")
            .arg("-show_entries").arg("format=duration")
            .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
            .arg(path)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("ffprobe failed: {}", e) })?;

        let dur_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        dur_str.parse::<f32>().map_err(|_| FactoryError::Infrastructure { reason: "Failed to parse duration".into() })
    }
}
