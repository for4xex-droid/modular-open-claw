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
mod asset_manager;
mod server;
use server::telemetry::TelemetryHub;
use server::router::{create_router, AppState};
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

use clap::Parser;
use tuning::StyleManager;
use asset_manager::AssetManager;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// 通常の動画生成モード
    Generate {
        /// 動画のカテゴリ
        #[arg(short, long, default_value = "tech")]
        category: String,

        /// 動画のトピック (テーマ)
        #[arg(short, long, default_value = "AIの未来")]
        topic: String,

        /// Remix 対象の動画ID (workspace/<ID> を再利用)
        #[arg(short, long)]
        remix: Option<String>,

        /// スキップ先のステップ (voice, visual)
        #[arg(short, long)]
        step: Option<String>,
    },
    /// 指令センター用サーバーモード (Port: 3000)
    Serve {
        #[arg(short, long, default_value = "3000")]
        port: u16,
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

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
    let supervisor = Supervisor::new(jail.clone(), SupervisorPolicy::Retry { max_retries: 3 });
    tracing::info!("⚖️  Governance Layer (Lex AI) Active");

    // 4. 新規マネージャの初期化 (Phase 8)
    let style_path = std::env::current_dir()?.join("styles.toml");
    let style_manager = Arc::new(StyleManager::load_from_file(style_path).unwrap_or_else(|_| {
        warn!("⚠️ styles.toml not found, using empty manager");
        StyleManager::new_empty()
    }));
    
    let asset_manager = Arc::new(AssetManager::new(std::env::current_dir()?.join("workspace")));

    // 5. インフラクライアントの準備
    let arbiter = Arc::new(ResourceArbiter::new());

    // Sidecar Manager ("The Reaper")
    let sidecar_manager = Arc::new(SidecarManager::new(vec![
        "python".to_string(), "python3".to_string(), "uv".to_string(), "main".to_string(),
    ]));

    // TTS Sidecar
    {
        let sm = sidecar_manager.clone();
        sm.clean_port(5001).await?;
        let mut cmd = Command::new("uv");
        cmd.arg("run").arg("server_fastapi.py").current_dir("services/Style-Bert-VITS2");
        sm.spawn(cmd).await?;
        info!("🎙️  TTS Sidecar server spawned on port 5001");
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

    // 6. 生産ライン・オーケストレーターの準備
    let orchestrator = Arc::new(ProductionOrchestrator::new(
        trend_sonar,
        concept_manager,
        voice_actor,
        comfy_bridge,
        media_forge,
        sound_mixer,
        supervisor,
        arbiter,
        style_manager.clone(),
        asset_manager.clone(),
    ));

    // コマンド分岐
    match args.command.unwrap_or(Commands::Generate { 
        category: "tech".to_string(), 
        topic: "AIの未来".to_string(), 
        remix: None, 
        step: None 
    }) {
        Commands::Serve { port } => {
            info!("📡 Starting Command Center Server on port {}", port);
            
            // Telemetry Hub
            let telemetry = Arc::new(TelemetryHub::new());
            telemetry.start_heartbeat_loop().await;

            // Axum Router
            let state = Arc::new(AppState {
                telemetry,
                orchestrator,
                style_manager,
                jail,
                is_busy: Arc::new(std::sync::Mutex::new(false)),
                asset_manager,
            });
            let app = create_router(state);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            
            axum::serve(listener, app).await?;
        }
        Commands::Generate { category, topic, remix, step } => {
            let workflow_req = WorkflowRequest { 
                category: category.clone(), 
                topic: topic.clone(),
                remix_id: remix.clone(),
                skip_to_step: step.clone(),
                style_name: String::new(), 
                custom_style: None,
            };
        
            info!("🚀 Launching Production Pipeline...");
            
            tokio::select! {
                res = orchestrator.execute(workflow_req, &jail) => {
                    match res {
                        Ok(res) => {
                            println!("\n🎬 動画生成完了！");
                            println!("   📝 タイトル: {}", res.concept.title);
                            println!("   🎨 スタイル: {}", res.concept.style_profile);
                            println!("   🎥 ファイル: {}", res.final_video_path);
                        }
                        Err(e) => {
                            error!("❌ 生成パイプラインが失敗: {}", e);
                        }
                    }
                }
                _ = signal::ctrl_c() => {
                    tracing::info!("🛑 SIGINT received. Shutting down gracefully...");
                }
            }
        }
    }

    Ok(())
}
