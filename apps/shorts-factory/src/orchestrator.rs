use factory_core::contracts::{
    ConceptRequest, TrendRequest, TrendResponse,
    VideoRequest, MediaRequest, MediaResponse,
    VoiceRequest, WorkflowRequest, WorkflowResponse
};
use factory_core::traits::{AgentAct, MediaEditor};
use factory_core::error::FactoryError;
use infrastructure::trend_sonar::BraveTrendSonar;
use infrastructure::concept_manager::ConceptManager;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::media_forge::MediaForgeClient;
use infrastructure::voice_actor::VoiceActor;
use infrastructure::sound_mixer::SoundMixer;
use crate::supervisor::Supervisor;
use crate::arbiter::{ResourceArbiter, ResourceUser};
use crate::asset_manager::AssetManager;
use tuning::StyleManager;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

/// 映像量産統括者 (ProductionOrchestrator)
/// 
/// 複数のアクターを協調させ、トレンド分析から動画完成までのパイプラインを管理する。
pub struct ProductionOrchestrator {
    pub trend_sonar: BraveTrendSonar,
    pub concept_manager: ConceptManager,
    pub voice_actor: VoiceActor,
    pub comfy_bridge: ComfyBridgeClient,
    pub media_forge: MediaForgeClient,
    pub sound_mixer: SoundMixer,
    pub supervisor: Supervisor,
    pub arbiter: Arc<ResourceArbiter>,
    pub style_manager: Arc<StyleManager>,
    pub asset_manager: Arc<AssetManager>,
    pub export_dir: String,
}

impl ProductionOrchestrator {
    pub fn new(
        trend_sonar: BraveTrendSonar,
        concept_manager: ConceptManager,
        voice_actor: VoiceActor,
        comfy_bridge: ComfyBridgeClient,
        media_forge: MediaForgeClient,
        sound_mixer: SoundMixer,
        supervisor: Supervisor,
        arbiter: Arc<ResourceArbiter>,
        style_manager: Arc<StyleManager>,
        asset_manager: Arc<AssetManager>,
        export_dir: String,
    ) -> Self {
        Self {
            trend_sonar,
            concept_manager,
            voice_actor,
            comfy_bridge,
            media_forge,
            sound_mixer,
            supervisor,
            arbiter,
            style_manager,
            asset_manager,
            export_dir,
        }
    }
}

#[async_trait]
impl AgentAct for ProductionOrchestrator {
    type Input = WorkflowRequest;
    type Output = WorkflowResponse;

    async fn execute(
        &self,
        input: WorkflowRequest,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<WorkflowResponse, FactoryError> {
        info!("🏭 Production Pipeline Start: Category = {}, Topic = {}", input.category, input.topic);

        // 0. プロジェクト ID の決定と初期化
        let project_id = input.remix_id.unwrap_or_else(|| {
            format!("{}_{}", input.category, chrono::Utc::now().format("%Y%m%d_%H%M%S"))
        });
        let project_root = self.asset_manager.init_project(&project_id)?;
        info!("📁 Project Workspace: {}", project_root.display());

        // 1. コンセプトの取得 (New or Remix)
        let concept_res = if let Some(_) = input.skip_to_step {
             // Remix モード: キャッシュから読み込み
             info!("🔄 Remix Mode: Loading existing concept...");
             self.asset_manager.load_concept(&project_id)?
        } else {
            // 新規生成モード
            info!("🌟 Generation Mode: Creating new concept...");
            
            // トレンド取得
            let trend_req = TrendRequest { category: input.category.clone() };
            let trend_res: TrendResponse = self.supervisor.enforce_act(&self.trend_sonar, trend_req).await?;
            
            // コンセプト立案 (Styles 注入 + トレンド共有)
            let concept_req = ConceptRequest { 
                topic: input.topic.clone(),
                category: input.category.clone(),
                trend_items: trend_res.items,
                available_styles: self.style_manager.list_available_styles(),
            };
            let res = self.supervisor.enforce_act(&self.concept_manager, concept_req).await?;
            
            // 保存
            self.asset_manager.save_concept(&project_id, &res)?;
            res
        };

        // 採択されたスタイルの取得
        // Phase 8.5: Remix Override logic
        let base_style_name = if !input.style_name.is_empty() {
            &input.style_name
        } else {
            &concept_res.style_profile
        };
        
        let mut style = self.style_manager.get_style(base_style_name);
        
        // Custom Overrides application
        if let Some(custom) = &input.custom_style {
            info!("🛠️  Applying Custom Style Overrides...");
            if let Some(v) = custom.zoom_speed { style.zoom_speed = v; }
            if let Some(v) = custom.pan_intensity { style.pan_intensity = v; }
            if let Some(v) = custom.bgm_volume { style.bgm_volume = v; }
            if let Some(v) = custom.ducking_threshold { style.ducking_threshold = v; }
            if let Some(v) = custom.ducking_ratio { style.ducking_ratio = v; }
            if let Some(v) = custom.fade_duration { style.fade_duration = v; }
        }
        
        info!("🎨 Applied Style: {} ({}) [Zoom: {:.4}, Vol: {:.2}]", 
            style.name, style.description, style.zoom_speed, style.bgm_volume);

        // --- 3幕構成 (Intro, Body, Outro) の各コンポーネント生成 ---
        let mut video_clips = Vec::new();
        let mut audio_clips = Vec::new();
        
        let acts = vec![
            (concept_res.script_intro.clone(), concept_res.visual_prompts.get(0).cloned().unwrap_or_default(), "intro"),
            (concept_res.script_body.clone(), concept_res.visual_prompts.get(1).cloned().unwrap_or_default(), "body"),
            (concept_res.script_outro.clone(), concept_res.visual_prompts.get(2).cloned().unwrap_or_default(), "outro"),
        ];

        for (i, (script, visual_prompt, act_name)) in acts.into_iter().enumerate() {
            let audio_path = project_root.join(format!("audio/scene_{}.wav", i));
            let video_clip_path = project_root.join(format!("visuals/scene_{}.mp4", i));

            // 3.1. 音声合成 (VoiceActor) / Bypass check
            if !audio_path.exists() || input.skip_to_step.as_deref() == Some("voice") {
                info!("🗣️  Processing Voice for Act: {}", act_name);
                let voice_res = {
                    let _guard = self.arbiter.acquire(ResourceUser::Voicing).await;
                    let voice_req = VoiceRequest {
                        text: script,
                        speaker_id: 0,
                        style: Some("Neutral".to_string()),
                    };
                    self.supervisor.enforce_act(&self.voice_actor, voice_req).await?
                };
                
                // Jail からプロジェクトディレクトリへコピー
                let temp_voice_path = std::path::PathBuf::from(voice_res.audio_path);
                std::fs::copy(&temp_voice_path, &audio_path).map_err(|e| FactoryError::Infrastructure {
                    reason: format!("Failed to persist audio: {}", e),
                })?;
            }
            audio_clips.push(audio_path);

            // 3.2. 画像生成 & 映像演出 (ComfyBridge) / Bypass check
            if !video_clip_path.exists() || input.skip_to_step.as_deref() == Some("visual") {
                info!("🖼️  Processing Visuals for Act: {}", act_name);
                let full_prompt = format!("{}, {}", concept_res.common_style, visual_prompt);
                
                let image_path = {
                    let _guard = self.arbiter.acquire(ResourceUser::Generating).await;
                    let video_req = VideoRequest {
                        prompt: full_prompt,
                        workflow_id: "shorts_standard_v1".to_string(),
                        input_image: None,
                    };
                    let res = self.supervisor.enforce_act(&self.comfy_bridge, video_req).await?;
                    std::path::PathBuf::from(res.output_path)
                };
                
                // Ken Burns エフェクト適用 (Style 注入)
                let clip = self.comfy_bridge.apply_ken_burns_effect(&image_path, 5.0, jail, &style).await?;
                std::fs::copy(&clip, &video_clip_path).map_err(|e| FactoryError::Infrastructure {
                    reason: format!("Failed to persist video clip: {}", e),
                })?;
            }
            video_clips.push(video_clip_path);
        }

        // 4. 最終合成 (MediaForge & SoundMixer)
        info!("🎞️  Orchestrator: Final Assembly (Style: {})...", style.name);
        
        // 4.1. ナレーションを結合
        let audio_strings: Vec<String> = audio_clips.iter().map(|p| p.to_string_lossy().to_string()).collect();
        let combined_narration_str = self.media_forge.concatenate_clips(audio_strings, "combined_narration.wav".to_string()).await?;
        let combined_narration = project_root.join("combined_narration.wav");
        std::fs::rename(combined_narration_str, &combined_narration).ok();
        
        // 4.2. 動画クリップを結合
        let video_strings: Vec<String> = video_clips.iter().map(|p| p.to_string_lossy().to_string()).collect();
        let combined_video_str = self.media_forge.concatenate_clips(video_strings, "combined_visuals.mp4".to_string()).await?;
        let combined_video = project_root.join("combined_visuals.mp4");
        std::fs::rename(combined_video_str, &combined_video).ok();
        
        // 4.3. BGM 混合とダッキング、正規化 (SoundMixer + Style 注入)
        let finalized_audio = project_root.join("finalized_audio.wav");
        self.sound_mixer.mix_and_finalize(&combined_narration, &input.category, &finalized_audio, &style).await?;
        
        // 4.4. 最終映像と最終音声を結合 (MediaForge)
        let media_req = MediaRequest {
            video_path: combined_video.to_string_lossy().to_string(),
            audio_path: finalized_audio.to_string_lossy().to_string(),
            subtitle_path: None,
        };
        let media_res: MediaResponse = self.supervisor.enforce_act(&self.media_forge, media_req).await?;

        // 5. 最終メタデータ保存
        self.asset_manager.save_metadata(&project_id, &style)?;

        // 6. Safe Move Protocol v2: 納品先への安全な移動
        info!("🚚  Orchestrator: Delivering final output via Safe Move Protocol...");
        let final_video_path = std::path::PathBuf::from(&media_res.final_path);
        let delivered_path = infrastructure::workspace_manager::WorkspaceManager::deliver_output(
            &project_id,
            &final_video_path,
            &self.export_dir,
        ).await?;

        info!("🏆 Production Pipeline Completed: {}", delivered_path.display());

        Ok(WorkflowResponse {
            final_video_path: delivered_path.to_string_lossy().to_string(),
            concept: concept_res,
        })
    }
}
