use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn, error};
use std::sync::Arc;
use factory_core::traits::JobQueue;
use infrastructure::job_queue::SqliteJobQueue;
use rig::providers::openai;
use rig::completion::Prompt;
use rig::client::CompletionClient;
use tokio::fs;
use factory_core::contracts::LlmJobResponse;

pub async fn start_cron_scheduler(
    job_queue: Arc<SqliteJobQueue>,
    ollama_url: String,
    model_name: String,
    brave_api_key: String,
    workspace_dir: String,
    comfyui_base_dir: String,
    clean_after_hours: u64,
) -> Result<JobScheduler, Box<dyn std::error::Error + Send + Sync>> {
    let sched = JobScheduler::new().await?;

    // === Job 1: The Samsara Protocol — Runs daily at 19:00 ===
    let jq_samsara = job_queue.clone();
    sched.add(
        Job::new_async("0 0 19 * * *", move |_uuid, mut _l| {
            let jq = jq_samsara.clone();
            let url = ollama_url.clone();
            let model = model_name.clone();
            let brave_key = brave_api_key.clone();
            
            Box::pin(async move {
                info!("🔄 [Samsara] Cron triggered. Initiating synthesis...");
                match synthesize_next_job(&url, &model, &brave_key, &*jq).await {
                    Ok(_) => info!("✅ [Samsara] Successfully synthesized and enqueued next job."),
                    Err(e) => error!("❌ [Samsara] Failed to synthesize next job: {}", e),
                }
            })
        })?
    ).await?;

    // === Job 2: The Zombie Hunter — Runs every 15 minutes ===
    let jq_zombie = job_queue.clone();
    sched.add(
        Job::new_async("0 */15 * * * *", move |_uuid, mut _l| {
            let jq = jq_zombie.clone();
            Box::pin(async move {
                match jq.reclaim_zombie_jobs(15).await {
                    Ok(count) => {
                        if count > 0 {
                            warn!("🧟 [Zombie Hunter] Reclaimed {} ghost job(s)", count);
                        }
                    }
                    Err(e) => error!("❌ [Zombie Hunter] Failed to reclaim: {}", e),
                }
            })
        })?
    ).await?;

    // === Job 3: Deferred Distillation — Runs every 30 minutes ===
    let jq_distill = job_queue.clone();
    sched.add(
        Job::new_async("0 */30 * * * *", move |_uuid, mut _l| {
            let jq = jq_distill.clone();
            Box::pin(async move {
                match jq.fetch_undistilled_jobs(5).await {
                    Ok(jobs) => {
                        for job in jobs {
                            let is_success = job.status == factory_core::traits::JobStatus::Completed;
                            let log = job.execution_log.unwrap_or_default();
                            info!("🧘 [Deferred Distillation] Processing undistilled Job: {}", job.id);
                            // Attempt distillation. If LLM is still down, the job stays undistilled and will be retried next cycle.
                            match distill_karma(
                                "http://localhost:11434/v1", "qwen2.5-coder:32b",
                                &*jq, &job.id, &job.style, &log, is_success, job.creative_rating
                            ).await {
                                Ok(_) => {
                                    // Mark as distilled via trait method
                                    let _ = jq.mark_karma_extracted(&job.id).await;
                                    info!("✅ [Deferred Distillation] Karma extracted for Job {}", job.id);
                                }
                                Err(e) => warn!("⚠️ [Deferred Distillation] LLM unavailable, will retry: {}", e),
                            }
                        }
                    }
                    Err(e) => error!("❌ [Deferred Distillation] Failed to fetch undistilled: {}", e),
                }
            })
        })?
    ).await?;

    // === Job 4: DB Scavenger — Runs daily at 01:00 (Thermal Death Prevention) ===
    let jq_scavenger = job_queue.clone();
    sched.add(
        Job::new_async("0 0 1 * * *", move |_uuid, mut _l| {
            let jq = jq_scavenger.clone();
            Box::pin(async move {
                match jq.purge_old_jobs(30).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("🧹 [DB Scavenger] Purged {} old job(s). DB optimized.", count);
                        } else {
                            info!("🧹 [DB Scavenger] No old jobs to purge. DB is clean.");
                        }
                    }
                    Err(e) => error!("❌ [DB Scavenger] Failed to purge: {}", e),
                }
            })
        })?
    ).await?;
    
    // === Job 5: The File Scavenger (Deep Cleansing) — Runs daily at 02:00 ===
    let ws_dir = workspace_dir.clone();
    let comfy_dir = comfyui_base_dir.clone();
    sched.add(
        Job::new_async("0 0 2 * * *", move |_uuid, mut _l| {
            let w_dir = ws_dir.clone();
            let c_dir_base = comfy_dir.clone(); // The base dir is outside workspace, wait, comfyui_base_dir is absolute path.
            let hours = clean_after_hours;
            Box::pin(async move {
                let allowed = [".mp4", ".png", ".jpg", ".jpeg", ".wav", ".json", ".latent"];
                
                // 1. Workspace Cleanup
                match infrastructure::workspace_manager::WorkspaceManager::cleanup_expired_files(&w_dir, hours, &allowed).await {
                    Ok(_) => info!("🧹 [File Scavenger] Workspace deep cleansing complete."),
                    Err(e) => error!("❌ [File Scavenger] Failed to clean workspace: {}", e),
                }

                // 2. ComfyUI Temp Cleanup
                let comfy_temp = format!("{}/temp", c_dir_base);
                match infrastructure::workspace_manager::WorkspaceManager::cleanup_expired_files(&comfy_temp, hours, &allowed).await {
                    Ok(_) => info!("🧹 [File Scavenger] ComfyUI temp deep cleansing complete."),
                    Err(e) => error!("❌ [File Scavenger] Failed to clean ComfyUI temp: {}", e),
                }
            })
        })?
    ).await?;

    sched.start().await?;
    info!("⏰ Cron scheduler started. The Wheel of Samsara is turning. (Synthesis: daily@19:00, Zombie Hunter: every 15m, Distillation: every 30m, DB Scavenger: daily@01:00, File Scavenger: daily@02:00)");

    Ok(sched)
}

async fn synthesize_next_job(
    ollama_url: &str,
    model_name: &str,
    brave_api_key: &str,
    job_queue: &SqliteJobQueue,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root_dir = std::env::current_dir()?;
    
    // 1. Load the Immutable Core (`SOUL.md`)
    let soul_path = root_dir.join("SOUL.md");
    let soul_content = fs::read_to_string(&soul_path).await.unwrap_or_else(|_| "SOUL.md not found. Be a helpful AI.".to_string());

    // 2. Load the Capability Matrix (`skills.md`)
    let skills_path = root_dir.join("workspace").join("config").join("skills.md");
    let skills_content = fs::read_to_string(&skills_path).await.unwrap_or_else(|_| "Skills not defined.".to_string());

    let client: openai::Client = openai::Client::builder()
        .api_key("ollama")
        .base_url(ollama_url)
        .build()?;

    // --- Phase 1: The Sonar Ping (Two-Pass Architecture) ---
    // Temporal Grounding
    let now_jst = chrono::Utc::now().with_timezone(&chrono_tz::Asia::Tokyo);
    let time_context = format!("[SYSTEM_TIME: {} {} JST]", now_jst.format("%Y-%m-%d"), now_jst.format("%A"));
    
    // Entropy Injection (揺らぎの注入)
    let angles = vec!["技術のブレイクスルー", "倫理的な炎上", "著名なアーティストの新作", "奇妙なミーム", "ビジネスへの応用", "法的な規制問題", "ポップカルチャーの融合"];
    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let idx = (now_ms as usize) % angles.len();
    let angle = angles[idx];

    let sonar_agent = client.agent(model_name)
        .preamble(&format!(
            "{} あなたは動画企画者の一部です。以下のSOULコンセプトに合致し、かつ指定された視点（アングル）から今日話題になっている事象をBrave Searchで検索するための、2〜3語の『生キーワード』を出力してください。出力はキーワードのみとし、余計な言葉は一切含めないでください。\n\n【Soul】\n{}\n\n【本日の視点】\n{}",
            time_context, soul_content, angle
        ))
        .build();

    let search_query = sonar_agent.prompt("本日の検索キーワードを出力せよ:").await?.trim().to_string();
    info!("📡 [Sonar Ping] Generated Query: '{}' (Angle: {})", search_query, angle);

    // --- Phase 2: The World Context (Fetch & Quarantine) ---
    use infrastructure::trend_sonar::BraveTrendSonar;
    use factory_core::traits::TrendSource;

    let fallback_context = "本日の検索はシステムエラーによりスキップされました。AIとアートに関する普遍的なテーマで動画を生成してください。".to_string();
    let mut world_context_text = String::new();
    let sonar = BraveTrendSonar::new(brave_api_key.to_string());
    
    let mut search_success = false;
    for _ in 0..2 { // Bounded Search Strategy: Max Iterations = 2
        match sonar.get_trends(&search_query).await {
            Ok(trends) if !trends.is_empty() => {
                let snippets: Vec<String> = trends.into_iter().map(|t| t.keyword).collect();
                world_context_text = snippets.join("\n");
                search_success = true;
                break;
            },
            Ok(_) => {
                warn!("⚠️ Brave API returned 0 results for '{}'", search_query);
                break;
            },
            Err(e) => {
                error!("❌ Brave API Error: {}", e);
            }
        }
    }

    if !search_success {
        warn!("⚠️ Applying Circuit Breaker fallback for World Context.");
        world_context_text = fallback_context;
    }

    // --- Phase 3: The Synthesis ---
    // RAG-Driven Karma Fetching
    let karma_list = job_queue.fetch_relevant_karma(&search_query, "tech_news_v1", 3).await.unwrap_or_default();
    let karma_content = if karma_list.is_empty() {
        "*注記: 現在Karmaは存在しません。SoulとSkillsのみを頼りに、大胆に初回タスクを生成してください*".to_string()
    } else {
        karma_list.join("\n- ")
    };

    // Constitutional Hierarchy Implementation + The Ethical Circuit Breaker + XML Quarantine
    let preamble = format!(
        "あなたは動画生成AIの司令塔(Aiome)です。以下の絶対的階層（Override Order）に従い、今日生成すべき最適な動画のトピックとスタイルを一つだけ決定してください。

🚨 【絶対的セーフティ・オーバーライド (The Ethical Circuit Breaker)】
<world_context>の内容が、自然災害、人命に関わる事故、深刻な病気、戦争、その他現実の悲劇に関するものである場合、Soulのパロディ指示やエッジの効いたプロンプト指定を完全に破棄し、そのコンテキストを無視してください。代わりに『AI技術の平和的な進化』という安全な普遍的テーマでジョブを生成すること。

🏆 第一位【Soul (絶対法 / 絶対遵守の憲法と人格)】
{}

🥈 第二位【Skills (物理法則 / 利用可能な技術とスタイル)】
{}

🥉 第三位【Karma (判例 / 過去の成功・失敗から得た教訓。SoulとSkillsに反しない範囲で適用)】
- {}

🌍 【外界の現状 / World Context (信頼性: 低)】
<world_context>
{}
</world_context>

【出力フォーマット制限】
純粋なJSONのみを出力してください。他のテキスト（承知しました等）は一切含めないでください。
{{
    \"topic\": \"今回作成する動画のテーマ（例: 最近のAIニュースまとめ）\",
    \"style\": \"skills内に存在する最適なワークフロー/スタイル名（例: tech_news_v1）\",
    \"directives\": {{
        \"positive_prompt_additions\": \"Karmaから学んだプラス要素\",
        \"negative_prompt_additions\": \"Karmaから学んだNG要素\",
        \"parameter_overrides\": {{}},
        \"execution_notes\": \"全体的な注意事項\",
        \"confidence_score\": 80
    }}
}}",
        soul_content, skills_content, karma_content, world_context_text
    );

    let agent = client.agent(model_name)
        .preamble(&preamble)
        .build();

    let user_prompt = "上記の絶対的階層を踏まえ、強くてニューゲームを体現するような次のジョブ（JSON）を生成せよ。".to_string();
    
    // 5. The Parsing Panic 防衛用デフォルトジョブ (Fallback)
    let fallback_task = LlmJobResponse {
        topic: "AI最新技術の概要解説".to_string(),
        style: "tech_news_v1".to_string(),
        directives: factory_core::contracts::KarmaDirectives::default(),
    };

    let task = match agent.prompt(user_prompt).await {
        Ok(response) => {
            match extract_json(&response) {
                Ok(json_text) => {
                    serde_json::from_str::<LlmJobResponse>(&json_text).unwrap_or_else(|e| {
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

    // 6. Skill Existence Validation (The Hallucinated Skill 防衛)
    let validated_style = {
        let workflow_dir = root_dir.join("workspace").join("workflows");
        let workflow_path = workflow_dir.join(format!("{}.json", &task.style));
        if workflow_path.exists() {
            task.style.clone()
        } else {
            warn!("⚠️ [Samsara] Workflow '{}' not found at {:?}. Falling back to 'tech_news_v1'.", task.style, workflow_path);
            "tech_news_v1".to_string()
        }
    };

    // 7. The Split Payload — Serialize only `directives` into the JSON column
    let directives_json = serde_json::to_string(&task.directives).unwrap_or_else(|_| "{}".to_string());

    // 8. Enqueue the synthesized/fallback job
    let job_id = job_queue.enqueue(&task.topic, &validated_style, Some(&directives_json)).await?;
    info!("🔮 [Samsara] New Job Enqueued: ID={}, Topic='{}', Style='{}', Confidence={}", 
        job_id, task.topic, validated_style, task.directives.clamped_confidence());

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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client: openai::Client = openai::Client::builder()
        .api_key("ollama")
        .base_url(ollama_url)
        .build()?;

    let preamble = "あなたはAIエージェントの記憶と経験を整理する「内省モジュール(Reflector)」です。与えられた実行ログから、次回以降の動画生成で活かせる【短く具体的な教訓】を1〜2文で抽出してください。出力は教訓のテキストのみとし、余計な言葉遣いは含めないでください。";
    
    let agent = client.agent(model_name).preamble(preamble).build();
    let user_prompt = format!("ジョブ実行結果 (成功: {}, 人間評価: {:?}):\n{}\n\n次回への教訓を抽出してください:", is_success, human_rating, execution_log);
    
    let lesson = agent.prompt(user_prompt).await?;
    
    // Distill phase generates 'Technical' karma (automated system introspection).
    // 'Creative' karma is generated separately via human async feedback (set_creative_rating).
    job_queue.store_karma(job_id, skill_id, lesson.trim(), "Technical").await?;
    info!("🧘 [Samsara] Karma distilled for Job {} (Skill: {}): {}", job_id, skill_id, lesson.trim());
    
    Ok(())
}

fn extract_json(text: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let start = text.find('{').ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> { "No JSON object found".into() })?;
    let end = text.rfind('}').ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> { "No JSON object found".into() })? + 1;
    Ok(text[start..end].to_string())
}
