use factory_core::contracts::{
    ConceptRequest, ConceptResponse, TrendRequest, TrendResponse,
    VideoRequest, VideoResponse, MediaRequest, MediaResponse,
    WorkflowRequest, WorkflowResponse
};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use infrastructure::trend_sonar::TrendSonarClient;
use infrastructure::concept_manager::ConceptManager;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::media_forge::MediaForgeClient;
use crate::supervisor::Supervisor;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;
use bastion::fs_guard::Jail;

/// 生産ライン・オーケストレーター
/// 
/// トレンドの取得から最終的な動画合成までの全行程を
/// Supervisor の管理下で段階的に実行する。
pub struct ProductionOrchestrator {
    supervisor: Arc<Supervisor>,
    trend_sonar: TrendSonarClient,
    concept_manager: ConceptManager,
    comfy_bridge: ComfyBridgeClient,
    media_forge: MediaForgeClient,
}

impl ProductionOrchestrator {
    pub fn new(
        supervisor: Arc<Supervisor>,
        trend_sonar: TrendSonarClient,
        concept_manager: ConceptManager,
        comfy_bridge: ComfyBridgeClient,
        media_forge: MediaForgeClient,
    ) -> Self {
        Self {
            supervisor,
            trend_sonar,
            concept_manager,
            comfy_bridge,
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

        // 1. トレンド取得 (TrendSonar)
        let trend_req = TrendRequest { category: input.category };
        let trend_res: TrendResponse = self.supervisor.enforce_act(&self.trend_sonar, trend_req).await?;
        
        if trend_res.items.is_empty() {
            return Err(FactoryError::Infrastructure { reason: "No trends found for the category".into() });
        }

        // 2. コンセプト生成 (ConceptManager / Director)
        let concept_req = ConceptRequest { trend_items: trend_res.items };
        let concept_res: ConceptResponse = self.supervisor.enforce_act(&self.concept_manager, concept_req).await?;

        // 3. 動画生成 (ComfyBridge)
        // ※ 本来は全シーン生成するが、デモとして最初のプロンプトのみ使用
        let video_req = VideoRequest {
            prompt: concept_res.visual_prompts.first().cloned().unwrap_or_default(),
            workflow_id: "shorts_standard_v1".to_string(),
        };
        let video_res: VideoResponse = self.supervisor.enforce_act(&self.comfy_bridge, video_req).await?;

        // 4. 音声・合成 (MediaForge)
        // ※ 本来は音声合成(TTS)も入るが、現状はダミーパスを使用
        let media_req = MediaRequest {
            video_path: video_res.output_path,
            audio_path: "assets/dummy_bgm.mp3".to_string(),
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
