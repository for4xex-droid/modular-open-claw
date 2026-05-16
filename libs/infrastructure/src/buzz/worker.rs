use super::generator::BuzzContentGenerator;
use super::scheduler::BuzzScheduler;
use aiome_core::error::AiomeError;
use aiome_core::traits::{JobQueue, JobStatus};
use chrono::{DateTime, Utc};
use std::time::SystemTime;

/// Background worker that polls the job queue and generates new Buzz drafts
/// when the scheduler permits (interval + daily limit checks).
#[tracing::instrument(skip_all, name = "buzz_worker")]
pub async fn process_pending_buzz(
    jq: &dyn JobQueue,
    gen: &BuzzContentGenerator,
    sched: &BuzzScheduler,
) -> Result<(), AiomeError> {
    // 1. Fetch recent jobs to find buzz-category entries
    let recent_jobs = jq.fetch_recent_jobs(100).await?;
    let buzz_jobs: Vec<_> = recent_jobs
        .into_iter()
        .filter(|j| j.category == "buzz")
        .collect();

    // 2. Count daily posts (jobs created today UTC) and find most recent timestamp
    let now = Utc::now();
    let start_of_day = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|ndt| ndt.and_utc())
        .unwrap_or(now); // fallback: treat entire day as counted
    let mut daily_post_count: u8 = 0;
    let mut last_published = SystemTime::UNIX_EPOCH;

    for job in &buzz_jobs {
        if let Ok(created_dt) = DateTime::parse_from_rfc3339(&job.created_at) {
            let created_utc = created_dt.with_timezone(&Utc);
            if created_utc >= start_of_day {
                daily_post_count = daily_post_count.saturating_add(1);
            }
            let sys_time = SystemTime::from(created_utc);
            if sys_time > last_published {
                last_published = sys_time;
            }
        }
    }

    // 3. If there's already a pending draft, skip generation
    if buzz_jobs.iter().any(|j| j.status == JobStatus::Pending) {
        return Ok(());
    }

    // 4. Generate a new draft if the scheduler permits
    if sched.can_publish(last_published, daily_post_count) {
        let template = sched.next_template();
        let topic = "System Automation";
        let draft = gen
            .generate(topic, template.clone(), "Aiome Project")
            .await?;

        let output_json =
            serde_json::to_string(&draft).map_err(|e| AiomeError::Infrastructure {
                reason: format!("BuzzDraft serialization failed: {e}"),
            })?;

        let job_id = jq
            .enqueue(
                "buzz",
                topic,
                &format!("{:?}", template),
                None,
                None,
                None,
                1,
            )
            .await?;

        jq.complete_job(&job_id, Some(&output_json)).await?;
        jq.update_job_status(&job_id, JobStatus::Pending).await?;

        sched.update_last_template(template);
        tracing::info!(job_id = %job_id, "🐝 [BuzzWorker] Draft generated and enqueued");
    }

    Ok(())
}
