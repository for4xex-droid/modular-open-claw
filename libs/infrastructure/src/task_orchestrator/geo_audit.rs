/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::Job;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::info;

pub struct GeoAuditConductor {
    geo_url: String,
    geo_threshold: u32,
}

impl GeoAuditConductor {
    pub fn new(geo_url: String, geo_threshold: u32) -> Self {
        Self {
            geo_url,
            geo_threshold,
        }
    }
}

#[async_trait]
impl TaskConductor for GeoAuditConductor {
    fn capable_categories(&self) -> Vec<String> {
        vec!["geo_audit".to_string()]
    }

    fn conductor_name(&self) -> &str {
        "GeoAuditConductor"
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        info!("🔍 [GEO] Executing audit job: {}", job.id);

        if job.topic.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "GEO audit content cannot be empty".to_string(),
            });
        }

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Running Generative Engine Optimization (GEO) audit...".to_string(),
                percent: Some(10),
            })
            .await
        {
            tracing::warn!(
                "Failed to send progress event for GEO job {}: {}",
                job.id,
                e
            );
        }

        // Job topic either has raw content or json payload. Let's send the whole job topic.
        // It's up to the geo-optimizer external service to parse what it needs,
        // but we'll format it consistently.
        let payload = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&job.topic) {
            parsed
        } else {
            serde_json::json!({
                "content": job.topic,
                "topic": "Autonomous Geo Optimization"
            })
        };

        // SEC: Use global SSRF-protected HTTP client with per-request timeout
        let client = aiome_core::http::get_http_client();
        match client
            .post(&self.geo_url)
            .timeout(std::time::Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(geo_result) = resp.json::<serde_json::Value>().await {
                    let score = (geo_result["score"].as_u64().unwrap_or(0)).min(100) as u32;
                    let geo_passed = score >= self.geo_threshold;

                    let mut optimized_content = job.topic.clone();
                    if let Some(optimized) = geo_result["optimized_content"].as_str() {
                        optimized_content = optimized.to_string();
                    }

                    if progress_tx
                        .send(TaskEvent::QualityGate {
                            job_id: job.id.clone(),
                            score,
                            passed: geo_passed,
                            conductor: self.conductor_name().to_string(),
                        })
                        .await
                        .is_err()
                    {
                        tracing::debug!("[GEO] Progress receiver dropped during QualityGate");
                    }

                    if !geo_passed {
                        return Err(AiomeError::Infrastructure {
                            reason: format!(
                                "GEO Optimization failed. Score: {} Threshold: {}",
                                score, self.geo_threshold
                            ),
                        });
                    }

                    return Ok((optimized_content, None));
                }

                Err(AiomeError::Infrastructure {
                    reason: "Failed to parse GEO optimizer response".to_string(),
                })
            }
            Ok(resp) => Err(AiomeError::Infrastructure {
                reason: format!("GEO optimizer returned error status: {}", resp.status()),
            }),
            Err(e) => Err(AiomeError::Infrastructure {
                reason: format!("GEO optimizer audit request failed: {}", e),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_geo_audit_conductor_categories() {
        let conductor = GeoAuditConductor::new("http://localhost:8080".to_string(), 60);
        assert_eq!(
            conductor.capable_categories(),
            vec!["geo_audit".to_string()]
        );
        assert_eq!(conductor.conductor_name(), "GeoAuditConductor");
    }

    #[tokio::test]
    async fn test_geo_audit_conductor_conduct_failure_stub() {
        let conductor = GeoAuditConductor::new("http://localhost:0".to_string(), 60);
        let (tx, _rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-geo-1".to_string();
        job.topic = "Check this content".to_string();
        job.category = "geo_audit".to_string();

        let result = conductor.conduct(job, tx).await;
        // Test fails because http://localhost:0 won't resolve.
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_geo_audit_conductor_empty_topic() {
        let conductor = GeoAuditConductor::new("http://localhost:8080".to_string(), 60);
        let (tx, _rx) = mpsc::channel(10);

        // Empty string
        let mut job = Job::default();
        job.id = "test-geo-empty-1".to_string();
        job.topic = "".to_string();
        let result = conductor.conduct(job, tx.clone()).await;
        assert!(result.is_err(), "Empty topic must be rejected");

        // Whitespace-only string
        let mut job2 = Job::default();
        job2.id = "test-geo-empty-2".to_string();
        job2.topic = "   \t\n  ".to_string();
        let result2 = conductor.conduct(job2, tx).await;
        assert!(result2.is_err(), "Whitespace-only topic must be rejected");
    }
}
