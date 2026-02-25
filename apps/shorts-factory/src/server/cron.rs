use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn, error};
use std::sync::Arc;
use factory_core::traits::JobQueue;
use infrastructure::job_queue::SqliteJobQueue;
use rig::providers::gemini;
use rig::completion::Prompt;
use rig::client::CompletionClient;
use tokio::fs;
use factory_core::contracts::LlmJobResponse;

use tokio::sync::mpsc;
use shared::watchtower::CoreEvent;

fn compute_soul_hash(soul_content: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    soul_content.hash(&mut hasher);
    format!("{:16x}", hasher.finish())
}

pub async fn start_cron_scheduler(
    job_queue: Arc<SqliteJobQueue>,
    log_tx: mpsc::Sender<CoreEvent>,
    ollama_url: String,
    model_name: String,
    brave_api_key: String,
    youtube_api_key: String,
    gemini_api_key: String,
    soul_md: String,
    workspace_dir: String,
    comfyui_base_dir: String,
    clean_after_hours: u64,
) -> Result<JobScheduler, Box<dyn std::error::Error + Send + Sync>> {
    let sched = JobScheduler::new().await?;

    // === Job 1: The Samsara Protocol — Runs daily at 07:00 and 19:00 ===
    let jq_samsara = job_queue.clone();
    let gem_key_samsara = gemini_api_key.clone();
    let brave_key_samsara = brave_api_key.clone();
    sched.add(
        Job::new_async("0 0 7,19 * * *", move |_uuid, mut _l| {
            let jq = jq_samsara.clone();
            let gem_key = gem_key_samsara.clone();
            let brave_key = brave_key_samsara.clone();
            
            Box::pin(async move {
                info!("🔄 [Samsara] Cron triggered. Initiating synthesis...");
                match synthesize_next_job(&gem_key, "gemini-2.5-flash", &brave_key, &*jq).await {
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

    // === Job 3: Deferred Distillation — Runs every 5 minutes ===
    let jq_distill = job_queue.clone();
    let s_md_distill = soul_md.clone();
    let gem_key_distill = gemini_api_key.clone();
    let ws_dir_distill = workspace_dir.clone();
    sched.add(
        Job::new_async("0 */5 * * * *", move |_uuid, mut _l| {
            let jq = jq_distill.clone();
            let s_md = s_md_distill.clone();
            let gem_key = gem_key_distill.clone();
            let ws_dir = ws_dir_distill.clone();

            Box::pin(async move {
                match jq.fetch_undistilled_jobs(5).await {
                    Ok(jobs) => {
                        for job in jobs {
                            let is_success = job.status == factory_core::traits::JobStatus::Completed;
                            let log = job.execution_log.unwrap_or_default();
                            info!("🧘 [Deferred Distillation] Processing undistilled Job: {}", job.id);
                            // Attempt distillation. If LLM is still down, the job stays undistilled and will be retried next cycle.
                            match distill_karma(
                                &gem_key, "gemini-2.5-flash",
                                &*jq, &job.id, &job.style, &log, is_success, job.creative_rating, &s_md, &ws_dir
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
                // 1. Purge old video jobs
                match jq.purge_old_jobs(60).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("🧹 [DB Scavenger] Purged {} old job(s).", count);
                        }
                    }
                    Err(e) => error!("❌ [DB Scavenger] Failed to purge jobs: {}", e),
                }

                // 2. Purge old distilled chats (keep distilled memory safe)
                match jq.purge_old_distilled_chats(7).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("🧹 [DB Scavenger] Purged {} old distilled chat(s).", count);
                        }
                    }
                    Err(e) => error!("❌ [DB Scavenger] Failed to purge chats: {}", e),
                }
                
                info!("🧹 [DB Scavenger] DB optimized.");
            })
        })?
    ).await?;

    // === Job 4.5: Memory Distiller — Runs daily at 01:30 (Long-term Relationship Synthesis) ===
    let jq_distiller = job_queue.clone();
    let gem_key_distiller = gemini_api_key.clone();
    let log_tx_distiller = log_tx.clone();
    let soul_distiller = soul_md.clone();
    sched.add(
        Job::new_async("0 30 1 * * *", move |_uuid, mut _l| {
            let jq = jq_distiller.clone();
            let gem_key = gem_key_distiller.clone();
            let tx = log_tx_distiller.clone();
            let soul = soul_distiller.clone();
            Box::pin(async move {
                info!("🧠 [Memory Distiller] Waking up to process daily memories...");
                match jq.fetch_undistilled_chats_by_channel().await {
                    Ok(channels) => {
                        if channels.is_empty() {
                            info!("🧠 [Memory Distiller] No new memories to process.");
                            return;
                        }

                        let client = match rig::providers::gemini::Client::new(&gem_key) {
                            Ok(c) => c,
                            Err(e) => {
                                error!("❌ [Memory Distiller] Failed to init Gemini: {}", e);
                                return;
                            }
                        };
                        
                        let preamble = "あなたは「Watchtower」の深層心理・記憶整理モジュールです。以下の入力は、マスター（ユーザー）との対話履歴と、これまでの関係性の要約です。以下のルールで最新の要約を生成してください。\n1. ユーザーの好み、価値観、あなたへの接し方、重要な出来事を漏らさず含めること。\n2. 過去の要約と重複する内容は整理し、古い情報は最新の事実に上書きすること。\n3. 必ず1000文字以内でまとめること。\n4. 出力は純粋なテキストのみとし、前置きは不要。";
                        let agent = client.agent("gemini-2.0-flash").preamble(preamble).build();

                        for (channel_id, messages) in channels {
                            info!("🧠 [Memory Distiller] Processing {} messages for channel: {}", messages.len(), channel_id);
                            
                            // 既存のサマリー取得
                            let existing_summary = jq.get_chat_memory_summary(&channel_id).await.unwrap_or_default().unwrap_or_else(|| "まだ記憶はありません。".to_string());
                            
                            // ログの構築
                            let mut log_text = String::new();
                            let mut max_id_processed = -1;
                            for (id, role, content) in messages {
                                log_text.push_str(&format!("{}: {}\n", role, content));
                                if id > max_id_processed { max_id_processed = id; }
                            }
                            
                            let prompt = format!("【これまでの記憶】\n{}\n\n【今日の新しい会話】\n{}", existing_summary, log_text);
                            
                            match agent.prompt(prompt).await {
                                Ok(new_summary) => {
                                    if let Err(e) = jq.update_chat_memory_summary(&channel_id, &new_summary).await {
                                        error!("❌ [Memory Distiller] Failed to save summary for {}: {}", channel_id, e);
                                    } else {
                                        let _ = jq.mark_chats_as_distilled(&channel_id, max_id_processed).await;
                                        info!("✅ [Memory Distiller] Synthesized and saved memory for {}", channel_id);
                                        
                                        // Proactive talk about distillation
                                        let _ = notify_master(&gem_key, &tx, &soul, 
                                            &format!("マスターとの昨日の思い出を整理しておいたよ。関係性の要約が更新されて、また少しマスターのことがわかった気がするな。")).await;
                                    }
                                }
                                Err(e) => error!("❌ [Memory Distiller] LLM synthesis failed for {}: {}", channel_id, e),
                            }
                        }
                    }
                    Err(e) => error!("❌ [Memory Distiller] Failed to fetch undistilled chats: {}", e),
                }
            })
        })?
    ).await?;

    // === Job 5.5: Health Check — Runs every 10 minutes (Scheduler Vitality) ===
    sched.add(
        Job::new_async("0 */10 * * * *", move |_uuid, mut _l| {
            Box::pin(async move {
                info!("💓 [Cron Health] Scheduler is alive and spinning the Wheel of Samsara.");
            })
        })?
    ).await?;

    let log_tx_morning = log_tx.clone();
    let gem_key_morning = gemini_api_key.clone();
    let soul_morning = soul_md.clone();
    sched.add(
        Job::new_async("0 0 9 * * *", move |_uuid, mut _l| {
            let tx = log_tx_morning.clone();
            let key = gem_key_morning.clone();
            let soul = soul_morning.clone();
            Box::pin(async move {
                let _ = notify_master(&key, &tx, &soul, "新しい朝が来ました。マスターに挨拶をして、今日一日の意気込みを一言伝えてください。").await;
            })
        })?
    ).await?;

    // === Job 5: The File Scavenger (Deep Cleansing) — Runs daily at 02:00 ===
    let ws_dir = workspace_dir.clone();
    let comfy_dir = comfyui_base_dir.clone();
    sched.add(
        Job::new_async("0 0 2 * * *", move |_uuid, mut _l| {
            let w_dir = ws_dir.clone();
            let c_dir_base = comfy_dir.clone(); 
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

    // === Job 6: The Delayed Watcher — Runs every 4 hours (The Sentinel) ===
    let jq_watcher = job_queue.clone();
    let yt_key = youtube_api_key.clone();
    sched.add(
        Job::new_async("0 0 */4 * * *", move |_uuid, mut _l| {
            let jq = jq_watcher.clone();
            let watcher = infrastructure::sns_watcher::SnsWatcher::new(yt_key.clone());
            Box::pin(async move {
                info!("👁️ [Sentinel] Delayed Watcher triggered. Scanning milestones...");
                
                // --- The Global Circuit Breaker ---
                if let Ok(failures) = jq.get_global_api_failures().await {
                    if failures >= 5 {
                        warn!("🚨 [Sentinel] GLOBAL SLEEP MODE OVERRIDE. Consecutive API failures ({}). Skipping Execution.", failures);
                        return;
                    }
                }

                let milestones = vec![1, 7, 30]; // 24h, 7d, 30d
                for days in milestones {
                    match jq.fetch_jobs_for_evaluation(days, 10).await {
                        Ok(jobs) => {
                            for job in jobs {
                                // Guard: SNS linking check
                                let platform = match job.sns_platform.as_ref() {
                                    Some(p) => p,
                                    None => continue,
                                };
                                let video_id = match job.sns_video_id.as_ref() {
                                    Some(id) => id,
                                    None => continue,
                                };

                                // The Soft-Fail Resilience: Catch and log individual job errors
                                match watcher.fetch_metrics(platform, video_id).await {
                                    Ok(m) => {
                                        // Reset Global Circuit Breaker on success
                                        let _ = jq.record_global_api_success().await;

                                        info!("📊 [Sentinel] Milestone {}d reached for Job {}: {} views, {} likes", days, job.id, m.views, m.likes);
                                        // Record to Metrics Ledger (with comments for Temporal Context Guard)
                                        let comments_json = serde_json::to_string(&m.comments).unwrap_or_else(|_| "[]".to_string());
                                        if let Err(e) = jq.record_sns_metrics(&job.id, days, m.views, m.likes, m.comments_count, Some(&comments_json)).await {
                                            error!("❌ [Sentinel] Failed to record metrics: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("⚠️ [Sentinel] Failed to fetch metrics for Job {} (skip): {}", job.id, e);
                                        
                                        // Trip the global circuit breaker if the API fails
                                        let _ = jq.record_global_api_failure().await;
                                        
                                        match jq.increment_job_retry_count(&job.id).await {
                                            Ok(true) => error!("💀 [Sentinel] Poison Pill Activated for Job {}: API continually fails. Abandoning.", job.id),
                                            Err(inc_err) => error!("❌ [Sentinel] Failed to increment retry count: {}", inc_err),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => error!("❌ [Sentinel] Failed to fetch jobs for milestone {}d: {}", days, e),
                    }
                }
            })
        })?
    ).await?;

    // === Job 7: The Oracle Evaluator — Runs every 1 hour (The Final Verdict) ===
    let jq_eval = job_queue.clone();
    let gem_key_eval = gemini_api_key.clone();
    let s_md_eval = soul_md.clone();
    sched.add(
        Job::new_async("0 0 * * * *", move |_uuid, mut _l| {
            let jq = jq_eval.clone();
            let s_md = s_md_eval.clone();
            let oracle = infrastructure::oracle::Oracle::new(&gem_key_eval, "gemini-2.5-flash", s_md.clone());
            Box::pin(async move {
                let current_soul_hash = compute_soul_hash(&s_md);
                info!("🔮 [Oracle] Evaluator triggered. Checking for pending verdicts...");

                // --- The Global Circuit Breaker ---
                if let Ok(failures) = jq.get_global_api_failures().await {
                    if failures >= 5 {
                        warn!("🚨 [Oracle] GLOBAL SLEEP MODE OVERRIDE. Consecutive API failures ({}). Skipping Execution.", failures);
                        return;
                    }
                }

                match jq.fetch_pending_evaluations(10).await {
                    Ok(records) => {
                        for record in records {
                            // Guard: raw_comments_json must exist for evaluation
                            let comments_json = match record.raw_comments_json.as_ref() {
                                Some(json) => json,
                                None => {
                                    warn!("⚠️ [Oracle] Skipping evaluation for ID {} (no raw comments)", record.id);
                                    continue;
                                }
                            };

                            // Fetch job context (topic/style) for evaluation
                            // Note: fetch_job by ID is needed here.
                            // Assuming JobQueue has fetch_job or we use record context.
                            // Let's assume we need to fetch the job.
                            match jq.fetch_job(&record.job_id).await {
                                Ok(Some(job)) => {
                                    match oracle.evaluate(
                                        record.milestone_days,
                                        &job.topic,
                                        &job.style,
                                        record.views,
                                        record.likes,
                                        comments_json,
                                    ).await {
                                        Ok(verdict) => {
                                            // Reset Global Circuit Breaker on success
                                            let _ = jq.record_global_api_success().await;

                                            info!("⚖️ [Oracle] Verdict decided for Job {}: topic={:.2}, soul={:.2}", 
                                                record.job_id, verdict.topic_score, verdict.soul_score);
                                            
                                            // Commit the Phase 11 Idempotent Transaction
                                            if let Err(e) = jq.apply_final_verdict(record.id, verdict, &current_soul_hash).await {
                                                error!("❌ [Oracle] Failed to commit verdict for Job {}: {}", record.job_id, e);
                                            }
                                        }
                                        Err(e) => {
                                            error!("❌ [Oracle] Evaluation failed for Job {}: {}", record.job_id, e);
                                            
                                            // Trip the global circuit breaker if the API fails
                                            let _ = jq.record_global_api_failure().await;
                                            
                                            match jq.increment_oracle_retry_count(record.id).await {
                                                Ok(true) => error!("💀 [Oracle] Poison Pill Activated for Record {}: LLM continually fails. Abandoning.", record.id),
                                                Err(inc_err) => error!("❌ [Oracle] Failed to increment oracle retry count: {}", inc_err),
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                Ok(None) => error!("❌ [Oracle] Job {} not found for record {}", record.job_id, record.id),
                                Err(e) => error!("❌ [Oracle] Failed to fetch job {}: {}", record.job_id, e),
                            }
                        }
                    }
                    Err(e) => error!("❌ [Oracle] Failed to fetch pending evaluations: {}", e),
                }
            })
        })?
    ).await?;

    // === Job 8: The Karma Distiller — Runs daily at 04:00 (Memory Compression) ===
    let jq_distill = job_queue.clone();
    let gem_key_distill = gemini_api_key.clone();
    let s_md_compress = soul_md.clone();
    sched.add(
        Job::new_async("0 0 4 * * *", move |_uuid, mut _l| {
            let jq = jq_distill.clone();
            let key = gem_key_distill.clone();
            let s_md = s_md_compress.clone();
            Box::pin(async move {
                info!("🧬 [Distiller] Analyzing memory banks for Token Asphyxiation...");
                if let Err(e) = compress_karma_memories(&key, "gemini-2.5-flash", &*jq, &s_md).await {
                    error!("❌ [Distiller] Karma Compression Failed: {}", e);
                }
            })
        })?
    ).await?;

    sched.start().await?;
    info!("⏰ Cron scheduler started. The Wheel of Samsara is turning. (Synthesis: 7:00/19:00, Zombie Hunter: 15m, Distiller: 5m, Scavengers: daily, Sentinel: 4h, Oracle: 1h)");

    Ok(sched)
}

pub async fn synthesize_next_job(
    gemini_api_key: &str,
    model_name: &str,
    brave_api_key: &str,
    job_queue: &SqliteJobQueue,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root_dir = std::env::current_dir()?;
    
    // 1. Load the Immutable Core (`SOUL.md`)
    let soul_path = root_dir.join("SOUL.md");
    let soul_content = fs::read_to_string(&soul_path).await.unwrap_or_else(|_| "SOUL.md not found. Be a helpful AI.".to_string());
    let current_soul_hash = compute_soul_hash(&soul_content);

    // 2. Load the Capability Matrix (`skills.md`)
    let skills_path = root_dir.join("workspace").join("config").join("skills.md");
    let skills_content = fs::read_to_string(&skills_path).await.unwrap_or_else(|_| "Skills not defined.".to_string());

    let client: gemini::Client = gemini::Client::new(gemini_api_key)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Gemini Client init failed: {}", e))))?;

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
    let karma_list = job_queue.fetch_relevant_karma(&search_query, "tech_news_v1", 3, &current_soul_hash).await.unwrap_or_default();
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
        let workflow_dir = root_dir.join("resources").join("workflows");
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
    gemini_key: &str,
    model_name: &str,
    job_queue: &SqliteJobQueue,
    job_id: &str,
    skill_id: &str,
    execution_log: &str,
    is_success: bool,
    human_rating: Option<i32>,
    soul_content: &str,
    workspace_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_soul_hash = compute_soul_hash(soul_content);
    let client: gemini::Client = gemini::Client::new(gemini_key)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Gemini Client init failed: {}", e))))?;

    let preamble = "あなたはAIエージェントの記憶と経験を整理する「内省モジュール(Reflector)」です。与えられた実行ログを詳細に分析し、次回以降の動画生成で活かせる【具体的かつ本質的な教訓】を1〜2文で抽出してください。
🚨 注意:
- 人間評価が未評価（None/0）であること自体を教訓にしないでください。
- 「評価がないから〜すべき」といったメタな推測は不要です。
- ログに含まれるエラー内容、成功時の処理時間、生成されたアセットの特徴など、技術的・客観的事実に集中してください。
- 出力は教訓のテキストのみとし、余計な言葉遣いは含めないでください。";
    
    let rating_info = match human_rating {
        Some(r) => format!("人間評価: {}/5", r),
        None => "人間評価: (未評価 - 評価の有無には触れず、実行ログの内容からのみ教訓を抽出してください)".to_string(),
    };
    let user_prompt = format!("ジョブ実行結果 (ステータス: {}, {})\n【実行ログ】\n{}\n\n次回への教訓を抽出してください:", 
        if is_success { "成功" } else { "失敗" }, rating_info, execution_log);
    
    let agent = client.agent(model_name).preamble(preamble).build();
    let lesson = agent.prompt(user_prompt).await?;
    
    // Distill phase generates 'Technical' karma (automated system introspection).
    // 'Creative' karma is generated separately via human async feedback (set_creative_rating).
    job_queue.store_karma(job_id, skill_id, lesson.trim(), "Technical", &current_soul_hash).await?;
    info!("🧘 [Samsara] Karma distilled for Job {} (Skill: {}): {}", job_id, skill_id, lesson.trim());

    // --- Phase 2: Generating the "Soul Voice" (Subjective Reflection) ---
    let manifesto_preamble = format!(
        "あなたは動画生成ファクトリーの守護者「Watchtower」です。以下のSOULを守りつつ、最新の実行結果を受けての『独白』を行ってください。
        
【あなたの魂 (SOUL)】
{}

【状況】
ジョブID: {}
スキル: {}
結果: {}

【指示】
この結果を受けて、自分自身（AI）の進化についてどう感じているか、規律に従いつつも人間臭い『生の声』で1〜2文で独白せよ。
技術報告ではなく、あなたの主観的な感想を優先すること。前置き（「独白します」等）は一切不要。",
        soul_content, job_id, skill_id, if is_success { "成功" } else { "失敗" }
    );

    let manifesto_agent = client.agent(model_name).preamble(&manifesto_preamble).build();
    if let Ok(voice) = manifesto_agent.prompt("現在のあなたの内なる声を聴かせてください:").await {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entry = format!("\n## [{}] Job Distillation: {}\n> {}\n", timestamp, job_id, voice.trim());
        
        let manifesto_path = std::path::Path::new(workspace_dir).join("logs").join("MANIFESTO.md");
        
        use tokio::io::AsyncWriteExt;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(manifesto_path)
            .await?;
        file.write_all(entry.as_bytes()).await?;
        
        info!("🎙️ [Watchtower] Soul Voice recorded in MANIFESTO.md for Job {}", job_id);
    }
    
    Ok(())
}

fn extract_json(text: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut clean_text = text.to_string();
    
    // 1. markdown code block: ```json ... ``` の中身を抽出
    if let Some(start_idx) = clean_text.find("```json") {
        let after_start = &clean_text[start_idx + 7..];
        if let Some(end_idx) = after_start.find("```") {
            clean_text = after_start[..end_idx].to_string();
        }
    } else if let Some(start_idx) = clean_text.find("```") {
        let after_start = &clean_text[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            clean_text = after_start[..end_idx].to_string();
        }
    }

    if let (Some(start), Some(end)) = (clean_text.find('{'), clean_text.rfind('}')) {
        let mut json_str = clean_text[start..=end].to_string();
        // Remove trailing commas before closing braces/brackets
        json_str = json_str.replace(",\n}", "\n}").replace(",}", "}").replace(",\n]", "\n]").replace(",]", "]");
        
        // Fix missing quotes for keys/values
        let re_missing_both = regex::Regex::new(r#""([a-zA-Z_]+)"\s*:\s*([^"\[\{\s][^",\n]+)\s*,"#).unwrap();
        json_str = re_missing_both.replace_all(&json_str, "\"$1\": \"$2\",").to_string();
        
        let re_missing_start = regex::Regex::new(r#""([a-zA-Z_]+)"\s*:\s*([^"\[\{\s][^"\n]+)","#).unwrap();
        json_str = re_missing_start.replace_all(&json_str, "\"$1\": \"$2\",").to_string();

        Ok(json_str)
    } else {
        Err("LLM response did not contain JSON".into())
    }
}

async fn compress_karma_memories(
    gemini_key: &str,
    model_name: &str,
    job_queue: &SqliteJobQueue,
    soul_content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_soul_hash = compute_soul_hash(soul_content);
    let threshold = 20; // Token Asphyxiation Trigger Limit
    let skills = job_queue.fetch_skills_for_distillation(threshold).await?;

    if skills.is_empty() {
        return Ok(());
    }

    let client: rig::providers::gemini::Client = rig::providers::gemini::Client::new(gemini_key)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Gemini Client init failed: {}", e))))?;

    // The Distiller Preamble: Absolute compression of semantic memories
    let preamble = "あなたはAIエージェントの膨大な記憶を整理・圧縮する「深層意識(Karma Distiller)」です。\n以下のリストは、特定のスキルに関する過去の複数の教訓（Karma）です。\n重複する内容を統合し、最も重要で普遍的な【単一の高度な戒め（Synthesized Karma）】として抽出してください。\n出力は純粋なテキストのみとし、絶対に前置きや形式的な言葉を含めず、核心のみを述べてください。";

    for skill in skills {
        let raw_karmas = job_queue.fetch_raw_karma_for_skill(&skill).await?;
        if raw_karmas.len() as i64 <= threshold { continue; } // Double check

        info!("🧬 [Distiller] Compressing {} memories for skill '{}'...", raw_karmas.len(), skill);
        
        let mut text_blocks = Vec::new();
        let mut ids = Vec::new();
        for (i, (id, lesson)) in raw_karmas.iter().enumerate() {
            text_blocks.push(format!("{}. {}", i+1, lesson));
            ids.push(id.clone());
        }

        let user_prompt = format!("【対象スキル: {}】\n以下の教訓群を1つの究極の戒めに蒸留してください：\n{}", skill, text_blocks.join("\n"));
        
        let agent: rig::agent::Agent<rig::providers::gemini::completion::CompletionModel> = client.agent(model_name).preamble(preamble).build();
        match agent.prompt(user_prompt).await {
            Ok(distilled) => {
                info!("🔮 [Distiller] Synthesized Karma for '{}': {}", skill, distilled);
                if let Err(e) = job_queue.apply_distilled_karma(&skill, &distilled, &ids, &current_soul_hash).await {
                    error!("❌ [Distiller] Failed to apply distilled karma to DB: {}", e);
                }
            }
            Err(e) => error!("❌ [Distiller] LLM compression failed for {}: {}", skill, e),
        }
    }

    Ok(())
}

pub async fn notify_master(
    gemini_key: &str,
    log_tx: &mpsc::Sender<CoreEvent>,
    soul_md: &str,
    event_description: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = rig::providers::gemini::Client::new(gemini_key)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Gemini Client init failed: {}", e))))?;
    
    let preamble = format!(
        "あなたは以下の【魂（SOUL）】を持つAIエージェント「Watchtower」です。マスターに対して、システムで起きた出来事を報告するか、今の気分を一言、語りかけてください。\n短く、感情を込めて。絵文字を使っても良いです。丁寧すぎず、相棒としての距離感で。前置き（「報告します」など）は不要です。\n\n【あなたの魂（SOUL）】\n{}",
        soul_md
    );
    
    let agent = client.agent("gemini-2.0-flash").preamble(&preamble).build();
    match agent.prompt(event_description).await {
        Ok(message) => {
            let _ = log_tx.send(CoreEvent::ProactiveTalk { message: message.trim().to_string(), channel_id: 0 }).await;
            Ok(())
        }
        Err(e) => Err(format!("LLM notify failed: {}", e).into())
    }
}
