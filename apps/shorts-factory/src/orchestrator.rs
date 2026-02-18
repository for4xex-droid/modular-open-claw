use factory_core::contracts::{
    ConceptRequest, TrendRequest, TrendResponse,
    VideoRequest, MediaRequest, MediaResponse,
    VoiceRequest,
    WorkflowRequest, WorkflowResponse
};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use infrastructure::trend_sonar::TrendSonarClient;
use infrastructure::concept_manager::ConceptManager;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::media_forge::MediaForgeClient;
use infrastructure::voice_actor::VoiceActor;
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
        media_forge: MediaForgeClient,
    ) -> Self {
        Self {
            supervisor,
            arbiter,
            trend_sonar,
            concept_manager,
            comfy_bridge,
            voice_actor,
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
        _jail: &Jail,
    ) -> Result<Self::Output, FactoryError> {
        info!("🏭 Production Pipeline Start: Category = {}", input.category);

        // 1. トレンド取得 (TrendSonar) - 非重負荷
        let trend_req = TrendRequest { category: input.category };
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

        // 3. 音声合成 (VoiceActor) - 重負荷 (TTS) [NEW in Phase 6]
        let voice_res = {
            let _guard = self.arbiter.acquire(ResourceUser::Voicing).await;
            let voice_req = VoiceRequest {
                text: concept_res.script.clone(),
                speaker_id: 0, // jvnv-F1-jp
                style: Some("Neutral".to_string()),
            };
            self.supervisor.enforce_act(&self.voice_actor, voice_req).await?
        };

        // 4. 動画生成 (ComfyBridge) - 重負荷 (GPU)
        let video_res = {
            let _guard = self.arbiter.acquire(ResourceUser::Generating).await;
            let video_req = VideoRequest {
                prompt: concept_res.visual_prompts.first().cloned().unwrap_or_default(),
                workflow_id: "shorts_standard_v1".to_string(),
            };
            self.supervisor.enforce_act(&self.comfy_bridge, video_req).await?
        };

        // 5. 最終合成 (MediaForge) - CPU
        let media_req = MediaRequest {
            video_path: video_res.output_path,
            audio_path: voice_res.audio_path,
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
