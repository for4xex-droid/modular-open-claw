use shared::config::FactoryConfig;
use shared::security::SecurityPolicy;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::trend_sonar::TrendSonarClient;
use infrastructure::media_forge::MediaForgeClient;
use bastion::fs_guard::Jail;
use std::sync::Arc;

mod supervisor;
mod orchestrator;
mod arbiter;
use supervisor::{Supervisor, SupervisorPolicy};
use orchestrator::ProductionOrchestrator;
use arbiter::ResourceArbiter;
use factory_core::contracts::WorkflowRequest;
use factory_core::traits::AgentAct;
use infrastructure::concept_manager::ConceptManager;
use infrastructure::voice_actor::VoiceActor;
use infrastructure::sound_mixer::SoundMixer;
use shared::health::HealthMonitor;
use tokio::signal;
use tracing::{info, error, warn};
use tokio::sync::Mutex;
use sidecar::SidecarManager;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    // 0. 運用監視 (Phase 3)
    let health = Arc::new(Mutex::new(HealthMonitor::new()));
    let status = health.lock().await.check();
    tracing::info!("📊 Initial Health Status: Memory {}MB, CPU {:.1}%", 
        status.memory_usage_mb, status.cpu_usage_percent);

    // 1. 設定を読み込む
    let config = FactoryConfig::default();
    let policy = SecurityPolicy::default_production();

    tracing::info!("⚙️  Config loaded:");
    tracing::info!("   Ollama:   {}", config.ollama_url);
    tracing::info!("   ComfyUI:  {}", config.comfyui_url);
    tracing::info!("   Model:    {}", config.model_name);

    // 2. セキュリティレイヤー (Bastion) の初期化
    tracing::info!("🔒 Industrial Security Layer (BASTION) Initializing...");
    let shield = Arc::new(policy.shield().clone());
    
    // 物理的リスク対策: 檻 (Jail) の位置をプロジェクト内の workspace に強制同期
    let jail_path = std::env::current_dir()?.join("workspace/shorts_factory");
    let jail = Arc::new(Jail::init(&jail_path)?);
    
    // ComfyUI 出力先の物理的同期用ディレクトリ作成
    let comfy_out = jail_path.join(&config.comfyui_output_dir);
    if !comfy_out.exists() {
        std::fs::create_dir_all(&comfy_out)?;
    }

    // DX向上対策: Guardrail Enforcement 状態の表示
    let enforce = std::env::var("ENFORCE_GUARDRAIL")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);
    tracing::info!("🛡️  Guardrails Enforcement: {}", if enforce { "Strict (DENY)" } else { "Relaxed (WARN)" });
    tracing::info!("📂 Jail Root: {}", jail_path.display());
    tracing::info!("📁 ComfyUI Sync: {}", comfy_out.display());
    
    // 3. 統治機構 (Supervisor) の初期化
    let supervisor = Arc::new(Supervisor::new(jail.clone(), SupervisorPolicy::Retry { max_retries: 3 }));
    tracing::info!("⚖️  Governance Layer (Lex AI) Active");

    // 4. インフラクライアントの準備
    let arbiter = ResourceArbiter::new();

    // Sidecar Manager ("The Reaper") の初期化
    let sidecar_manager = Arc::new(SidecarManager::new(vec![
        "python".to_string(),
        "python3".to_string(),
        "uv".to_string(),
        "main".to_string(),
    ]));

    // TTS サーバーの起動 (Port: 5001)
    {
        let sm = sidecar_manager.clone();
        sm.clean_port(5001).await?;
        
        // uv run server_fastapi.py を実行するコマンドを構築
        // Cwd はプロジェクトルートからの相対パス
        let mut cmd = Command::new("uv");
        cmd.arg("run")
           .arg("server_fastapi.py")
           .current_dir("services/Style-Bert-VITS2");
        
        sm.spawn(cmd).await?;
        info!("🎙️  TTS Sidecar server (Style-Bert-VITS2) spawned on port 5001");
    }

    // Infrastructure Clients
    let trend_sonar = TrendSonarClient::new(shield.clone());
    let concept_manager = ConceptManager::new(&config.ollama_url, &config.model_name);
    let comfy_bridge = ComfyBridgeClient::new(shield.clone(), &config.comfyui_url, config.comfyui_timeout_secs);
    let voice_actor = VoiceActor::new("http://localhost:5001", "jvnv-F1-jp");
    let bgm_path = std::env::current_dir()?.join("resources/bgm");
    if !bgm_path.exists() {
        std::fs::create_dir_all(&bgm_path)?;
    }
    let sound_mixer = SoundMixer::new(bgm_path);
    let media_forge = MediaForgeClient::new(jail.clone());

    // 5. 生産ライン・オーケストレーターの準備
    let orchestrator = ProductionOrchestrator::new(
        supervisor.clone(),
        arbiter.clone(),
        trend_sonar,
        concept_manager,
        comfy_bridge,
        voice_actor,
        sound_mixer,
        media_forge,
    );

    // 6. メインループ (Graceful Shutdown 対応)
    tokio::select! {
        _ = async {
            // 自動量産実行 (Phase 5 Batch Loop)
            let categories = vec!["jp_all", "tech", "entertainment"];
            
            for category in categories {
                let workflow_req = WorkflowRequest { category: category.to_string() };
                
                info!("🚀 Starting Production Pipeline for category: {}", workflow_req.category);
                
                // リソースチェック
                let status = health.lock().await.check();
                if status.memory_usage_mb > 1024 {
                    warn!("⚠️ High memory usage detected ({}MB). Skipping batch...", status.memory_usage_mb);
                    break;
                }

                match orchestrator.execute(workflow_req, &jail).await {
                    Ok(res) => {
                        println!("\n🎬 動画生成完了！");
                        println!("   🏷️ カテゴリ: {}", category);
                        println!("   📝 タイトル: {}", res.concept.title);
                        println!("   🎥 ファイル: {}", res.final_video_path);
                    }
                    Err(e) => {
                        error!("❌ カテゴリ {} の生成パイプラインが失敗: {}", category, e);
                    }
                }
                
                // 次のバッチまで少し待機
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
            
            info!("✅ All batches scheduled or completed.");
        } => {
            tracing::info!("🏁 Batch Production Task finished.");
        }
        _ = signal::ctrl_c() => {
            tracing::info!("🛑 SIGINT received. Shutting down gracefully...");
        }
    }

    Ok(())
}
