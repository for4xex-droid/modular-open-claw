use factory_core::contracts::{
    ConceptRequest, TrendRequest, TrendResponse,
    VideoRequest, MediaRequest, MediaResponse,
    VoiceRequest, VoiceResponse,
    WorkflowRequest, WorkflowResponse
};
use factory_core::traits::{AgentAct, MediaEditor};
use factory_core::error::FactoryError;
use infrastructure::trend_sonar::TrendSonarClient;
use infrastructure::concept_manager::ConceptManager;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::media_forge::MediaForgeClient;
use infrastructure::voice_actor::VoiceActor;
use infrastructure::sound_mixer::SoundMixer;
use crate::supervisor::Supervisor;
use crate::arbiter::{ResourceArbiter, ResourceUser};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;
use bastion::fs_guard::Jail;

/// 生産ライン・オーケストレーター
pub struct ProductionOrchestrator {
    supervisor: Arc<Supervisor>,
    arbiter: ResourceArbiter,
    trend_sonar: TrendSonarClient,
    concept_manager: ConceptManager,
    comfy_bridge: ComfyBridgeClient,
    voice_actor: VoiceActor,
    sound_mixer: SoundMixer,
    media_forge: MediaForgeClient,
}

impl ProductionOrchestrator {
    pub fn new(
        supervisor: Arc<Supervisor>,
        arbiter: ResourceArbiter,
        trend_sonar: TrendSonarClient,
        concept_manager: ConceptManager,
        comfy_bridge: ComfyBridgeClient,
        voice_actor: VoiceActor,
        sound_mixer: SoundMixer,
        media_forge: MediaForgeClient,
    ) -> Self {
        Self {
            supervisor,
            arbiter,
            trend_sonar,
            concept_manager,
            comfy_bridge,
            voice_actor,
            sound_mixer,
            media_forge,
        }
    }
}

#[async_trait]
impl AgentAct for ProductionOrchestrator {
    type Input = WorkflowRequest;
    type Output = WorkflowResponse;

    async fn execute(
        &self,
        input: Self::Input,
        jail: &Jail,
    ) -> Result<Self::Output, FactoryError> {
        info!("🏭 Production Pipeline Start: Category = {}", input.category);

        // 1. トレンド取得 (TrendSonar) - 非重負荷
        let trend_req = TrendRequest { category: input.category.clone() };
        let trend_res: TrendResponse = self.supervisor.enforce_act(&self.trend_sonar, trend_req).await?;
        
        if trend_res.items.is_empty() {
            return Err(FactoryError::Infrastructure { reason: "No trends found for the category".into() });
        }

        // 2. コンセプト生成 (ConceptManager / Director) - 重負荷 (LLM)
        let concept_res = {
            let _guard = self.arbiter.acquire(ResourceUser::Scripting).await;
            let concept_req = ConceptRequest { trend_items: trend_res.items };
            self.supervisor.enforce_act(&self.concept_manager, concept_req).await?
        };

        // --- 3幕構成 (Intro, Body, Outro) の各コンポーネント生成 ---
        let mut video_clips = Vec::new();
        let mut audio_clips = Vec::new();
        
        // 各パートの脚本とプロンプトの対応付け
        let acts = vec![
            (concept_res.script_intro.clone(), concept_res.visual_prompts.get(0).cloned().unwrap_or_default(), "intro"),
            (concept_res.script_body.clone(), concept_res.visual_prompts.get(1).cloned().unwrap_or_default(), "body"),
            (concept_res.script_outro.clone(), concept_res.visual_prompts.get(2).cloned().unwrap_or_default(), "outro"),
        ];

        for (script, visual_prompt, act_name) in acts {
            info!("🎬 Processing Act: {}", act_name);

            // 3.1. 音声合成 (VoiceActor) - 重負荷 (TTS)
            let voice_res = {
                let _guard = self.arbiter.acquire(ResourceUser::Voicing).await;
                let voice_req = VoiceRequest {
                    text: script,
                    speaker_id: 0,
                    style: Some("Neutral".to_string()),
                };
                self.supervisor.enforce_act(&self.voice_actor, voice_req).await?
            };
            audio_clips.push(std::path::PathBuf::from(voice_res.audio_path));

            // 3.2. 画像生成 (ComfyBridge) - 重負荷 (GPU)
            // 固定部 (common_style) と 可変部 (visual_prompt) を結合
            let full_prompt = format!("{}, {}", concept_res.common_style, visual_prompt);
            let video_res = {
                let _guard = self.arbiter.acquire(ResourceUser::Generating).await;
                let video_req = VideoRequest {
                    prompt: full_prompt,
                    workflow_id: "shorts_standard_v1".to_string(),
                };
                self.supervisor.enforce_act(&self.comfy_bridge, video_req).await?
            };
            
            // 3.3. Ken Burns エフェクト適用 (CPU Offloading)
            let image_path = std::path::PathBuf::from(video_res.output_path);
            let video_clip = self.comfy_bridge.apply_ken_burns_effect(&image_path, 5.0, jail).await?;
            video_clips.push(video_clip);
        }

        // 4. 最終合成 (MediaForge & SoundMixer)
        info!("🎞️  Orchestrator: Final Assembly (3-Act Concatenation)...");
        
        // 4.1. ナレーションを結合
        let audio_strings: Vec<String> = audio_clips.iter().map(|p| p.to_string_lossy().to_string()).collect();
        let combined_narration = self.media_forge.concatenate_clips(audio_strings, "combined_narration.wav".to_string()).await?;
        
        // 4.2. 動画クリップを結合
        let video_strings: Vec<String> = video_clips.iter().map(|p| p.to_string_lossy().to_string()).collect();
        let combined_video_str = self.media_forge.concatenate_clips(video_strings, "combined_visuals.mp4".to_string()).await?;
        let combined_video = std::path::PathBuf::from(combined_video_str);
        
        // 4.3. BGM 混合とダッキング、正規化 (SoundMixer)
        let finalized_audio = jail.root().join("finalized_audio.wav");
        self.sound_mixer.mix_and_finalize(std::path::Path::new(&combined_narration), &input.category, &finalized_audio).await?;
        
        // 4.4. 最終映像と最終音声を結合 (MediaForge)
        let media_req = MediaRequest {
            video_path: combined_video.to_string_lossy().to_string(),
            audio_path: finalized_audio.to_string_lossy().to_string(),
            subtitle_path: None,
        };
        let media_res: MediaResponse = self.supervisor.enforce_act(&self.media_forge, media_req).await?;

        info!("🏆 Production Pipeline Completed: {}", media_res.final_path);

        Ok(WorkflowResponse {
            final_video_path: media_res.final_path,
            concept: concept_res,
        })
    }
}
