use rig::{client::CompletionClient, completion::Prompt, providers::openai};
use shared::config::FactoryConfig;
use shared::guardrails::{self, ValidationResult};
use shared::security::SecurityPolicy;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    // 設定を読み込む
    let config = FactoryConfig::default();
    let policy = SecurityPolicy::default();

    tracing::info!("⚙️  Config loaded:");
    tracing::info!("   Ollama:   {}", config.ollama_url);
    tracing::info!("   ComfyUI:  {}", config.comfyui_url);
    tracing::info!("   Model:    {}", config.model_name);

    // セキュリティポリシーの検証
    tracing::info!("🔒 Security Policy:");
    tracing::info!("   Allowed tools: {:?}", policy.allowed_tools);
    tracing::info!("   Allowed hosts: {:?}", policy.allowed_hosts);
    tracing::info!("   External skills blocked: {}", policy.block_external_skills);
    tracing::info!("🛡️  Guardrails: ACTIVE");

    // 1. Ollama へ接続 (OpenAI互換 Chat Completions API)
    let client: openai::CompletionsClient = openai::Client::builder()
        .api_key("ollama")
        .base_url(&config.ollama_url)
        .build()?
        .completions_api();

    // 2. Factory Agent (工場長) を作成
    let factory_agent = client
        .agent(&config.model_name)
        .preamble(
            "あなたは ShortsFactory の工場長です。\
             YouTube Shorts向けの動画を効率的に量産する戦略を立案してください。\
             回答は必ず日本語で行ってください。",
        )
        .build();

    // 3. プロンプトを Guardrails で検証してから送信
    let user_prompt = "Mac mini M4 Proを使って、効率よく動画を量産する戦略を一言で教えて。";

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
