use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error};
use factory_core::traits::{JobQueue, JobStatus, AgentAct};
use factory_core::contracts::WorkflowRequest;
use factory_core::error::FactoryError;
use chrono::Utc;
use infrastructure::job_queue::SqliteJobQueue;
use crate::orchestrator::ProductionOrchestrator;
use bastion::fs_guard::Jail;

pub struct JobWorker {
    job_queue: Arc<SqliteJobQueue>,
    orchestrator: Arc<ProductionOrchestrator>,
    jail: Arc<Jail>,
    is_busy: Arc<Mutex<bool>>,
    soul_md: String,
}

impl JobWorker {
    pub fn new(
        job_queue: Arc<SqliteJobQueue>,
        orchestrator: Arc<ProductionOrchestrator>,
        jail: Arc<Jail>,
        soul_md: String,
    ) -> Self {
        Self {
            job_queue,
            orchestrator,
            jail,
            is_busy: Arc::new(Mutex::new(false)),
            soul_md,
        }
    }

    pub async fn start_loop(self: Arc<Self>) {
        info!("🤖 JobWorker: Starting autonomous execution loop...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            // 1. Check if busy
            {
                let busy = self.is_busy.lock().await;
                if *busy {
                    continue;
                }
            }

            // 2. Poll for next job
            match self.job_queue.dequeue().await {
                Ok(Some(job)) => {
                    info!("🏗️ JobWorker: Dequeued Job {}: {}", job.id, job.topic);
                    
                    let worker = self.clone();
                    tokio::spawn(async move {
                        worker.process_job(job).await;
                    });
                }
                Ok(None) => {
                    // No pending jobs
                }
                Err(e) => {
                    error!("❌ JobWorker: Failed to dequeue job: {}", e);
                }
            }
        }
    }

    async fn process_job(&self, job: factory_core::traits::Job) {
        // Set busy
        {
            let mut busy = self.is_busy.lock().await;
            *busy = true;
        }

        let job_id = job.id.clone();
        let queue = self.job_queue.clone();
        let soul_hash = compute_soul_hash(&self.soul_md);

        // 0. Start Heartbeat Pulse (The Life Support)
        let (hb_tx, mut hb_rx) = tokio::sync::oneshot::channel::<()>();
        let hb_job_id = job_id.clone();
        let hb_queue = queue.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = hb_queue.heartbeat_pulse(&hb_job_id).await {
                            error!("⚠️ JobWorker: Heartbeat Pulse Failed for {}: {}", hb_job_id, e);
                        }
                    }
                    _ = &mut hb_rx => break,
                }
            }
        });

        // Map Job to WorkflowRequest
        let req = WorkflowRequest {
            category: "tech".to_string(), 
            topic: job.topic.clone(),
            remix_id: None,
            skip_to_step: None,
            style_name: job.style.clone(),
            custom_style: None,
        };

        match self.orchestrator.execute(req, &self.jail).await {
            Ok(res) => {
                info!("✅ JobWorker: Job {} completed successfully: {}", job_id, res.final_video_path);
                
                // Store success log for Distillation
                let success_log = format!(
                    "SUCCESS_LOG: {}\nVideo: {}\nConcept: {}", 
                    Utc::now().to_rfc3339(), 
                    res.final_video_path,
                    res.concept.title
                );
                let _ = self.job_queue.store_execution_log(&job_id, &success_log).await;

                if let Err(e) = self.job_queue.complete_job(&job_id).await {
                    error!("❌ JobWorker: Failed to mark job as completed: {}", e);
                }
            }
            Err(e) => {
                error!("🚨 JobWorker: Job {} failed: {}", job_id, e);
                
                // ALWAYS record execution log on failure for Distillation
                let error_detail = format!("FAILURE_LOG: {}\nError: {}", Utc::now().to_rfc3339(), e);
                let _ = self.job_queue.store_execution_log(&job_id, &error_detail).await;

                // --- Honorable Abort & Internal Karma Backpropagation ---
                match e {
                    FactoryError::TtsFailure { reason } => {
                        warn!("💀 JobWorker: TTS FAILURE detected. Executing Honorable Abort for Job {}", job_id);
                        let _ = self.job_queue.fail_job(&job_id, &format!("TTS_ABORT: {}", reason)).await;
                        
                        let lesson = format!(
                            "WARNING: このコンセプトはTTSエンジンを破壊する可能性がありました。理由は: {}。今後はより純粋な日本語のみを使用してください。",
                            reason
                        );
                        let _ = self.job_queue.store_karma(&job_id, "voicing_failure_system", &lesson, "failure", &soul_hash).await;
                    }
                    _ => {
                        let lesson = format!("SYSTEM_ALERT: ジョブが {} により失敗しました。", e);
                        let _ = self.job_queue.store_karma(&job_id, "system_infrastructure", &lesson, "failure", &soul_hash).await;
                        let _ = self.job_queue.fail_job(&job_id, &e.to_string()).await;
                    }
                }
            }
        }

        // Stop Heartbeat Pulse
        let _ = hb_tx.send(());

        // Release busy
        {
            let mut busy = self.is_busy.lock().await;
            *busy = false;
        }
    }
}

fn compute_soul_hash(soul_content: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    soul_content.hash(&mut hasher);
    format!("{:16x}", hasher.finish())
}
