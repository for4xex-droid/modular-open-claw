use rig::{client::CompletionClient, completion::Prompt, providers::openai};
use shared::config::FactoryConfig;
use shared::guardrails::{self, ValidationResult};
use shared::security::SecurityPolicy;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::trend_sonar::TrendSonarClient;
use infrastructure::media_forge::MediaForgeClient;
use bastion::fs_guard::Jail;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

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
    
    // 3. インフラクライアントの準備
    let trend_sonar = TrendSonarClient::new(shield.clone());
    let comfy_bridge = ComfyBridgeClient::new(shield.clone(), &config.comfyui_url, config.comfyui_timeout_secs);
    let media_forge = MediaForgeClient::new(jail.clone());

    // 4. Ollama へ接続 (OpenAI互換 Chat Completions API)
    let client: openai::CompletionsClient = openai::Client::builder()
        .api_key("ollama")
        .base_url(&config.ollama_url)
        .build()?
        .completions_api();

    // 5. Factory Agent (工場長) を作成し、ツールを装着
    tracing::info!("🤝 Factory Manager (Agent) wrapping tools...");
    let factory_agent = client
        .agent(&config.model_name)
        .preamble(
            "あなたは ShortsFactory の工場長です。\
             YouTube Shorts向けの動画を効率的に量産する戦略を立案し、ツールを駆使して実行してください。\
             回答は必ず日本語で行ってください。",
        )
        .tool(trend_sonar)
        .tool(comfy_bridge)
        .tool(media_forge)
        .build();

    // 6. プロンプトを Guardrails で検証してから送信
    let user_prompt = "現在のトレンドを調べて、それに基づいた動画生成ワークフローを提案して。";

    // Guardrails: サニタイズ → バリデーション
    let sanitized = guardrails::sanitize_input(user_prompt);
    match guardrails::validate_input(&sanitized) {
        ValidationResult::Valid => {
            tracing::info!("🧠 Factory Manager に質問中...");
            let response = factory_agent.prompt(&sanitized).await?;
            println!("\n🏭 Factory Manager: {}", response);
        }
        ValidationResult::Blocked(reason) => {
            tracing::warn!("🚫 Guardrails がプロンプトをブロック: {}", reason);
            println!("\n⛔ プロンプトは安全上の理由でブロックされました: {}", reason);
        }
    }

    Ok(())
}
