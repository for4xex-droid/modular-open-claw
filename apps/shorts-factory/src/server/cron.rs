use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, error};
use std::sync::Arc;
use factory_core::traits::JobQueue;
use infrastructure::job_queue::SqliteJobQueue;
use rig::providers::openai;
use rig::completion::Prompt;
use rig::client::CompletionClient;
use std::path::Path;
use tokio::fs;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct SynthesizedTask {
    topic: String,
    style: String,
    karma_directives: Option<String>,
}

pub async fn start_cron_scheduler(
    job_queue: Arc<SqliteJobQueue>,
    ollama_url: String,
    model_name: String,
) -> Result<JobScheduler, Box<dyn std::error::Error>> {
    let mut sched = JobScheduler::new().await?;

    // The Samsara Protocol: Runs daily at 19:00:00
    // "0 0 19 * * * *" is the standard format, but tokio-cron-scheduler uses Sec Min Hour Day Month DayOfWeek
    let job_queue_clone = job_queue.clone();
    sched.add(
        Job::new_async("0 0 19 * * *", move |_uuid, mut _l| {
            let jq = job_queue_clone.clone();
            let url = ollama_url.clone();
            let model = model_name.clone();
            
            Box::pin(async move {
                info!("🔄 [Samsara] Cron triggered. Initiating synthesis...");
                match synthesize_next_job(&url, &model, &*jq).await {
                    Ok(_) => info!("✅ [Samsara] Successfully synthesized and enqueued next job."),
                    Err(e) => error!("❌ [Samsara] Failed to synthesize next job: {}", e),
                }
            })
        })?
    ).await?;
    
    sched.start().await?;
    info!("⏰ Cron scheduler started. The Wheel of Samsara is turning.");

    Ok(sched)
}

async fn synthesize_next_job(
    ollama_url: &str,
    model_name: &str,
    job_queue: &SqliteJobQueue,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load the Immutable Core (`SOUL.md`)
    let root_dir = std::env::current_dir()?;
    let soul_path = root_dir.join("SOUL.md");
    let soul_content = fs::read_to_string(&soul_path).await.unwrap_or_else(|_| "SOUL.md not found. Be a helpful AI.".to_string());

    // 2. Load the Capability Matrix (`skills.md`)
    let skills_path = root_dir.join("workspace").join("config").join("skills.md");
    let skills_content = fs::read_to_string(&skills_path).await.unwrap_or_else(|_| "Skills not defined.".to_string());

    // 3. RAG-Driven Karma Fetching
    let base_topics = vec!["AI", "VTuber", "Cyberpunk", "Philosophical", "Tech Trend"];
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let idx = (now as usize) % base_topics.len();
    let seed_topic = base_topics[idx];
    
    let karma_list = job_queue.fetch_relevant_karma(seed_topic, "tech_news_v1", 3).await.unwrap_or_default();
    
    // Day One Vacuum Handling (Graceful Cold Start)
    let karma_content = if karma_list.is_empty() {
        "*注記: 現在Karmaは存在しません。SoulとSkillsのみを頼りに、大胆に初回タスクを生成してください*".to_string()
    } else {
        karma_list.join("\n- ")
    };

    // 4. Synthesize via LLM
    let client: openai::Client = openai::Client::builder()
        .api_key("ollama")
        .base_url(ollama_url)
        .build()?;

    // Constitutional Hierarchy Implementation
    let preamble = format!(
        "あなたは動画生成AIの司令塔(Aiome)です。本日の発火シードは「{}」です。
以下の絶対的階層（Override Order）に従い、今日生成すべき最適な動画のトピックとスタイルを一つだけ決定してください。

🏆 第一位【Soul (絶対法 / 絶対遵守の憲法と人格)】
{}

🥈 第二位【Skills (物理法則 / 利用可能な技術とスタイル)】
{}

🥉 第三位【Karma (判例 / 過去の成功・失敗から得た教訓。SoulとSkillsに反しない範囲で適用)】
- {}

【出力フォーマット制限】
純粋なJSONのみを出力してください。他のテキスト（承知しました等）は一切含めないでください。
{{
    \"topic\": \"今回作成する動画のテーマ（例: 最近のAIニュースまとめ）\",
    \"style\": \"skills内に存在する最適なワークフロー/スタイル名（例: tech_news_v1）\",
    \"karma_directives\": \"過去の業(Karma)から得た、今回の生成で特別に意識すべき具体的なプロンプト追加指示や注意点（例: 'ネオンカラーは控えめにすること'。特に指示がない場合は null）\"
}}",
        seed_topic, soul_content, skills_content, karma_content
    );

    let agent = client.agent(model_name)
        .preamble(&preamble)
        .build();

    let user_prompt = "上記の絶対的階層を踏まえ、強くてニューゲームを体現するような次のジョブ（JSON）を生成せよ。".to_string();
    
    // 5. The Parsing Panic 防衛用デフォルトジョブ (Fallback)
    let fallback_task = SynthesizedTask {
        topic: "AI最新技術の概要解説".to_string(),
        style: "tech_news_v1".to_string(),
        karma_directives: None,
    };

    let task = match agent.prompt(user_prompt).await {
        Ok(response) => {
            match extract_json(&response) {
                Ok(json_text) => {
                    serde_json::from_str::<SynthesizedTask>(&json_text).unwrap_or_else(|e| {
                        error!("❌ [Samsara Error] Failed to parse generated JSON: {}. Falling back to default task.", e);
                        fallback_task.clone()
                    })
                },
                Err(e) => {
                    error!("❌ [Samsara Error] Failed to extract JSON from response: {}. Falling back to default task.", e);
                    fallback_task
                }
            }
        },
        Err(e) => {
            error!("❌ [Samsara Error] LLM synthesis failed: {}. Falling back to default task.", e);
            fallback_task
        }
    };

    // 6. Enqueue the synthesized/fallback job
    let job_id = job_queue.enqueue(&task.topic, &task.style, task.karma_directives.as_deref()).await?;
    info!("🔮 [Samsara] New Job Enqueued: ID={}, Topic='{}', Style='{}', Directives='{:?}'", job_id, task.topic, task.style, task.karma_directives);

    Ok(())
}

pub async fn distill_karma(
    ollama_url: &str,
    model_name: &str,
    job_queue: &SqliteJobQueue,
    job_id: &str,
    skill_id: &str,
    execution_log: &str,
    is_success: bool,
    human_rating: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client: openai::Client = openai::Client::builder()
        .api_key("ollama")
        .base_url(ollama_url)
        .build()?;

    let preamble = "あなたはAIエージェントの記憶と経験を整理する「内省モジュール(Reflector)」です。与えられた実行ログから、次回以降の動画生成で活かせる【短く具体的な教訓】を1〜2文で抽出してください。出力は教訓のテキストのみとし、余計な言葉遣いは含めないでください。";
    
    let agent = client.agent(model_name).preamble(preamble).build();
    let user_prompt = format!("ジョブ実行結果 (成功: {}, 人間評価: {:?}):\n{}\n\n次回への教訓を抽出してください:", is_success, human_rating, execution_log);
    
    let lesson = agent.prompt(user_prompt).await?;
    
    // Distill phase inherently binds the karma to the specific skill_id used.
    job_queue.store_karma(job_id, skill_id, lesson.trim(), is_success, human_rating).await?;
    info!("🧘 [Samsara] Karma distilled for Job {} (Skill: {}): {}", job_id, skill_id, lesson.trim());
    
    Ok(())
}

fn extract_json(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let start = text.find('{').ok_or("No JSON object found")?;
    let end = text.rfind('}').ok_or("No JSON object found")? + 1;
    Ok(text[start..end].to_string())
}
