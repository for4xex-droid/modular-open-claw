/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::TaskRegistry;
use axum::{
    extract::{Path, State},
    Json,
};
use infrastructure::job_queue::evaluation::EvaluationOps;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBuzzRequest {
    pub trend_source: String,
    pub project_context: String,
    pub template: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateDraftRequest {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuzzResponse {
    pub success: bool,
    pub message: String,
}

pub async fn generate(
    State(state): State<AppState>,
    _auth: crate::auth::ProAuthenticated,
    req: Option<Json<GenerateBuzzRequest>>,
) -> Result<Json<BuzzResponse>, AiomeError> {
    let req = req
        .ok_or_else(|| AiomeError::Validation {
            reason: "Missing request body".into(),
        })?
        .0;

    let template = match req.template.as_str() {
        "TechnicalInsight" => infrastructure::buzz::templates::BuzzTemplate::TechnicalInsight,
        "MilestoneAnnouncement" => {
            infrastructure::buzz::templates::BuzzTemplate::MilestoneAnnouncement
        }
        "CommunityQuestion" => infrastructure::buzz::templates::BuzzTemplate::CommunityQuestion,
        "ControversialTake" => infrastructure::buzz::templates::BuzzTemplate::ControversialTake,
        _ => {
            return Err(AiomeError::Validation {
                reason: format!("Unknown template: {}", req.template),
            })
        }
    };

    let draft = state
        .buzz_generator
        .generate(&req.trend_source, template, &req.project_context)
        .await?;

    let output_json = serde_json::to_string(&draft).map_err(|e| AiomeError::Validation {
        reason: format!("Failed to serialize draft: {}", e),
    })?;

    let jq = state.job_queue.get_inner();
    let job_id = jq
        .enqueue(
            "buzz",            // category
            &req.trend_source, // topic
            &req.template,     // style
            None,              // karma_directives
            None,              // permission_manifest
            None,              // agent_id
            1,                 // priority
        )
        .await?;

    jq.complete_job(&job_id, Some(&output_json)).await?;
    jq.update_job_status(&job_id, aiome_core_contracts::traits::JobStatus::Pending)
        .await?;

    Ok(Json(BuzzResponse {
        success: true,
        message: job_id,
    }))
}

/// Buzz の保留中ジョブ一覧を取得する。
///
/// **アクセスポリシー: Free Tier (Authenticated)**
/// このルートは意図的に `Authenticated`（Pro 不要）で保護されています。
/// 理由: Buzz 生成（`generate`）は Pro 限定ですが、既に生成依頼済みのジョブの
/// 進捗確認は無料ユーザーにも許可する設計です（ダウングレード後も確認可能）。
pub async fn list_pending(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl axum::response::IntoResponse, AiomeError> {
    let jq = state.job_queue.get_inner();
    let jobs = jq.fetch_recent_jobs(100).await?;

    let pending_jobs: Vec<_> = jobs
        .into_iter()
        .filter(|j| {
            j.category == "buzz" && j.status == aiome_core_contracts::traits::JobStatus::Pending
        })
        .collect();

    Ok(Json(pending_jobs))
}

pub async fn approve(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(id): Path<String>,
) -> Result<Json<BuzzResponse>, AiomeError> {
    let jq = state.job_queue.get_inner();
    let job = jq
        .fetch_job(&id)
        .await?
        .ok_or_else(|| AiomeError::NotFound {
            reason: "Job not found".into(),
        })?;

    if job.category != "buzz" || job.status != aiome_core_contracts::traits::JobStatus::Pending {
        return Err(AiomeError::Validation {
            reason: "Job is not a pending buzz draft".into(),
        });
    }

    jq.update_job_status(&id, aiome_core_contracts::traits::JobStatus::InProgress)
        .await?;

    Ok(Json(BuzzResponse {
        success: true,
        message: "Approved".into(),
    }))
}

pub async fn reject(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(id): Path<String>,
) -> Result<Json<BuzzResponse>, AiomeError> {
    let jq = state.job_queue.get_inner();
    let job = jq
        .fetch_job(&id)
        .await?
        .ok_or_else(|| AiomeError::NotFound {
            reason: "Job not found".into(),
        })?;

    if job.category != "buzz" || job.status != aiome_core_contracts::traits::JobStatus::Pending {
        return Err(AiomeError::Validation {
            reason: "Job is not a pending buzz draft".into(),
        });
    }

    jq.update_job_status(&id, aiome_core_contracts::traits::JobStatus::Failed)
        .await?;

    Ok(Json(BuzzResponse {
        success: true,
        message: "Rejected".into(),
    }))
}

pub async fn update_draft(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(id): Path<String>,
    req: Option<Json<UpdateDraftRequest>>,
) -> Result<Json<BuzzResponse>, AiomeError> {
    let req = req
        .ok_or_else(|| AiomeError::Validation {
            reason: "Missing request body".into(),
        })?
        .0;

    let jq = state.job_queue.get_inner();
    let job = jq
        .fetch_job(&id)
        .await?
        .ok_or_else(|| AiomeError::NotFound {
            reason: "Job not found".into(),
        })?;

    if job.category != "buzz" || job.status != aiome_core_contracts::traits::JobStatus::Pending {
        return Err(AiomeError::Validation {
            reason: "Job is not a pending buzz draft".into(),
        });
    }

    if let Some(artifacts) = job.output_artifacts {
        let mut draft: infrastructure::buzz::generator::BuzzDraft =
            serde_json::from_str(&artifacts).map_err(|e| AiomeError::Validation {
                reason: format!("Failed to parse artifacts: {}", e),
            })?;
        draft.text = req.text;

        let new_artifacts = serde_json::to_string(&draft).map_err(|e| AiomeError::Validation {
            reason: format!("Failed to serialize artifacts: {}", e),
        })?;

        jq.complete_job(&id, Some(&new_artifacts)).await?;
        jq.update_job_status(&id, aiome_core_contracts::traits::JobStatus::Pending)
            .await?;
    } else {
        return Err(AiomeError::Validation {
            reason: "Job has no draft artifacts".into(),
        });
    }

    Ok(Json(BuzzResponse {
        success: true,
        message: "Draft updated".into(),
    }))
}

pub async fn publish(
    State(state): State<AppState>,
    _auth: crate::auth::ProAuthenticated,
    Path(id): Path<String>,
) -> Result<Json<BuzzResponse>, AiomeError> {
    tracing::info!(buzz_id = %id, "Buzz publish requested");

    let jq = state.job_queue.get_inner();
    let job = jq
        .fetch_job(&id)
        .await?
        .ok_or_else(|| AiomeError::NotFound {
            reason: "Job not found".into(),
        })?;

    if job.status != aiome_core_contracts::traits::JobStatus::InProgress {
        tracing::warn!(buzz_id = %id, status = ?job.status, "Publish rejected: wrong status");
        return Err(AiomeError::Validation {
            reason: "Job is not in Approved (InProgress) state".into(),
        });
    }

    let raw_artifacts = job.output_artifacts.unwrap_or_default();
    if raw_artifacts.is_empty() {
        return Err(AiomeError::Validation {
            reason: "Buzz content is empty — cannot publish".into(),
        });
    }

    // Deserialize BuzzDraft to extract the actual tweet text
    let draft: infrastructure::buzz::generator::BuzzDraft = serde_json::from_str(&raw_artifacts)
        .map_err(|e| AiomeError::Validation {
            reason: format!("Failed to parse buzz draft: {e}"),
        })?;
    let content = draft.text;
    if content.is_empty() {
        return Err(AiomeError::Validation {
            reason: "Buzz draft text is empty — cannot publish".into(),
        });
    }

    let char_count = content.chars().count();
    if char_count > 280 {
        tracing::warn!(buzz_id = %id, char_count, "Content exceeds 280 chars");
        // X API counts Unicode codepoints, not bytes. Warn but do not block — the MCP tool
        // will return an error from the X API if the content is truly too long.
    }

    // Discover the MCP client that provides `post_tweet`
    let active_clients = state.mcp_manager.active_client_ids().await;
    let mut tweet_id: Option<String> = None;

    for cid in &active_clients {
        let client = match state.mcp_manager.get_client(cid).await {
            Some(c) => c,
            None => continue,
        };

        let tools =
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), client.list_tools())
                .await
            {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    tracing::warn!(mcp_client = %cid, error = %e, "list_tools failed");
                    continue;
                }
                Err(_) => {
                    tracing::warn!(mcp_client = %cid, "list_tools timed out");
                    continue;
                }
            };

        if !tools.iter().any(|t| t.name == "post_tweet") {
            continue;
        }

        tracing::info!(mcp_client = %cid, "Found post_tweet tool, invoking");
        let args = serde_json::json!({ "text": content });

        let result = match tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            client.call_tool("post_tweet", args),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                tracing::error!(mcp_client = %cid, error = %e, "post_tweet call failed");
                return Err(AiomeError::Infrastructure {
                    reason: format!("MCP post_tweet failed: {}", e),
                });
            }
            Err(_) => {
                return Err(AiomeError::Infrastructure {
                    reason: "MCP post_tweet timed out after 30s".into(),
                });
            }
        };

        if result.is_error {
            let error_text: String = result
                .content
                .into_iter()
                .filter_map(|c| {
                    if let crate::mcp::types::McpContent::Text { text } = c {
                        Some(text)
                    } else {
                        None
                    }
                })
                .collect();
            tracing::error!(buzz_id = %id, error = %error_text, "post_tweet returned error");
            return Err(AiomeError::Infrastructure {
                reason: format!("X post failed: {}", error_text),
            });
        }

        let mut out = String::new();
        for c in result.content {
            if let crate::mcp::types::McpContent::Text { text } = c {
                out.push_str(&text);
            }
        }
        tweet_id = Some(out);
        break;
    }

    let tid = tweet_id.ok_or_else(|| {
        tracing::error!(buzz_id = %id, "No MCP client provides post_tweet tool");
        AiomeError::Infrastructure {
            reason:
                "No MCP client with post_tweet tool available. Ensure @iflow-mcp/ is configured."
                    .into(),
        }
    })?;

    jq.do_link_sns_data(&id, "X", &tid).await?;
    jq.update_job_status(&id, aiome_core_contracts::traits::JobStatus::Completed)
        .await?;

    tracing::info!(buzz_id = %id, tweet_id = %tid, "Buzz published successfully");

    Ok(Json(BuzzResponse {
        success: true,
        message: format!("Published successfully with ID: {}", tid),
    }))
}

pub async fn history(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl axum::response::IntoResponse, AiomeError> {
    let jq = state.job_queue.get_inner();
    let jobs = jq.fetch_recent_jobs(100).await?;

    let history_jobs: Vec<_> = jobs
        .into_iter()
        .filter(|j| {
            j.category == "buzz" && j.status == aiome_core_contracts::traits::JobStatus::Completed
        })
        .collect();

    Ok(Json(history_jobs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_integration_tests::{create_test_server, test_bearer};
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_buzz_endpoints_are_registered() {
        let (server, _state, _tmp) = create_test_server().await;
        let bearer = test_bearer();

        let res = server
            .get("/api/v1/buzz/pending")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {}", bearer)).unwrap(),
            )
            .await;

        // It is currently unimplemented, so it returns 500. We will change this to expect 200.
        assert_eq!(res.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_buzz_unauthenticated_access_blocked() {
        let (server, _state, _tmp) = create_test_server().await;

        // No Authorization header → must be rejected
        let res = server.get("/api/v1/buzz/pending").await;
        assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_buzz_publish_nonexistent_job() {
        let (server, _state, _tmp) = create_test_server().await;
        let bearer = test_bearer();

        let res = server
            .post("/api/v1/buzz/publish/nonexistent-job-id-000")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {}", bearer)).unwrap(),
            )
            .await;

        // Should fail with NotFound (404) — matching project convention
        let status = res.status_code();
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "Expected 404 NOT_FOUND for nonexistent job, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_buzz_all_endpoints_respond() {
        let (server, _state, _tmp) = create_test_server().await;
        let bearer = test_bearer();

        let endpoints = vec![
            ("GET", "/api/v1/buzz/pending"),
            ("GET", "/api/v1/buzz/history"),
        ];

        for (method, path) in endpoints {
            let res = match method {
                "GET" => {
                    server
                        .get(path)
                        .add_header(
                            axum::http::header::AUTHORIZATION,
                            axum::http::HeaderValue::from_str(&format!("Bearer {}", bearer))
                                .unwrap(),
                        )
                        .await
                }
                _ => unreachable!(),
            };
            // All implemented endpoints should return 200/400/404, not 500
            assert_ne!(
                res.status_code(),
                StatusCode::INTERNAL_SERVER_ERROR,
                "Endpoint {} {} returned 500 — still stubbed",
                method,
                path
            );
        }
    }
}
