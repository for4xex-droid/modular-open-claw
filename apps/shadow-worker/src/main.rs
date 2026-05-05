/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
use aiome_core::llm_provider::{GeminiProvider, LlmProvider, OllamaProvider};
use aiome_core_contracts::a2a::internal::{
    ExecuteTaskRequest, TaskProgress,
    docker_conductor_server::{DockerConductor, DockerConductorServer},
    proof_verifier_server::ProofVerifierServer,
};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};
use tracing::info;

mod proof_service;

pub struct ShadowWorkerService {
    auth_token: String,
    gemini_api_key: Option<secrecy::SecretString>,
}

#[tonic::async_trait]
impl DockerConductor for ShadowWorkerService {
    type ExecuteTaskStream = ReceiverStream<Result<TaskProgress, Status>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        // Token verification (Threat #36 mitigation & GAP-O Timing attack mitigation)
        let token = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("Missing authorization metadata"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization encoding"))?;

        if !shared::security::constant_time_ends_with(token, &self.auth_token) {
            return Err(Status::unauthenticated("Invalid auth token"));
        }

        let req = request.into_inner();
        info!("Received task request: job_id={}", req.job_id);

        let (tx, rx) = mpsc::channel(4);
        let job_id = req.job_id.clone();
        let prompt_b64 = req.prompt_b64.clone();
        let gemini_key = self.gemini_api_key.clone();

        tokio::spawn(async move {
            info!("Executing task for job: {}", job_id);

            let _ = tx
                .send(Ok(TaskProgress {
                    message: "Initializing worker and decoding prompt".into(),
                    percent: 10,
                    is_completed: false,
                    is_failed: false,
                    result: "".into(),
                    error: "".into(),
                    result_hash: "".into(),
                }))
                .await;

            let prompt = match general_purpose::STANDARD.decode(&prompt_b64) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        let _ = tx
                            .send(Ok(TaskProgress {
                                message: "Prompt decoding failed".into(),
                                percent: 100,
                                is_completed: true,
                                is_failed: true,
                                result: "".into(),
                                error: "Invalid UTF-8 in decoded prompt".into(),
                                result_hash: "".into(),
                            }))
                            .await;
                        return;
                    }
                },
                Err(e) => {
                    let _ = tx
                        .send(Ok(TaskProgress {
                            message: "Prompt decoding failed".into(),
                            percent: 100,
                            is_completed: true,
                            is_failed: true,
                            result: "".into(),
                            error: format!("Base64 decode error: {}", e),
                            result_hash: "".into(),
                        }))
                        .await;
                    return;
                }
            };

            let _ = tx
                .send(Ok(TaskProgress {
                    message: "Invoking LLM Provider".into(),
                    percent: 30,
                    is_completed: false,
                    is_failed: false,
                    result: "".into(),
                    error: "".into(),
                    result_hash: "".into(),
                }))
                .await;

            // Use GEMINI_API_KEY if exists, else fallback to Ollama
            let llm_res = match gemini_key {
                Some(key) if !secrecy::ExposeSecret::expose_secret(&key).is_empty() => {
                    let provider = GeminiProvider::new(
                        aiome_core::http::get_http_client().clone(),
                        key,
                        "gemini-2.5-flash".to_string(),
                    );
                    provider.complete(&prompt, Some("You are an autonomous Aiome Shadow Worker. Execute the requested objective securely.")).await
                }
                _ => {
                    // Internal docker network or host path
                    let host = env::var("OLLAMA_HOST")
                        .unwrap_or_else(|_| "http://host.docker.internal:11434".to_string());
                    let provider = OllamaProvider::new(host, "llama3".to_string());
                    provider.complete(&prompt, Some("You are an autonomous Aiome Shadow Worker. Execute the requested objective securely.")).await
                }
            };

            match llm_res {
                Ok(response) => {
                    let mut hasher = Sha256::new();
                    hasher.update(response.content.as_bytes());
                    let result_hash = hex::encode(hasher.finalize());

                    let _ = tx
                        .send(Ok(TaskProgress {
                            message: "Task completed successfully".into(),
                            percent: 100,
                            is_completed: true,
                            is_failed: false,
                            result: aiome_core::security_impl::purge_entities(&response.content),
                            error: "".into(),
                            result_hash,
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(TaskProgress {
                            message: "LLM execution failed".into(),
                            percent: 100,
                            is_completed: true,
                            is_failed: true,
                            result: "".into(),
                            error: format!("Engine Error: {:?}", e),
                            result_hash: "".into(),
                        }))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CELL_ID").unwrap_or_default().is_empty() {
        panic!(
            "🚨 FATAL: CELL_ID is not set! The Sovereign Verifier architecture requires strict cellular isolation. No identity = No survival."
        );
    }

    tracing_subscriber::fmt::init();

    info!("Starting Shadow Worker...");

    // Port and Token configuration
    let port = env::var("SHADOW_CLONE_GRPC_PORT").unwrap_or_else(|_| "50051".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    let auth_token = env::var("A2A_AUTH_TOKEN")
        .map_err(|_| "A2A_AUTH_TOKEN environment variable is required")?;
    shared::security::scrub_env("A2A_AUTH_TOKEN");

    let gemini_api_key = env::var("GEMINI_API_KEY")
        .ok()
        .map(secrecy::SecretString::from);
    shared::security::scrub_env("GEMINI_API_KEY");

    let proof_auth_token = auth_token.clone();

    let worker = ShadowWorkerService {
        auth_token,
        gemini_api_key,
    };

    let proof_timeout_secs: u64 = env::var("OXILEAN_PROOF_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let proof_semaphore_permits: usize = env::var("OXILEAN_PROOF_SEMAPHORE_PERMITS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let config = shared::config::AiomeConfig::default();
    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.db_path)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to connect to SQLite DB for OXP aggregation: {}", e);
            e
        })
        .ok();

    let proof_service = proof_service::OxiLeanProofService::new(
        proof_auth_token,
        Duration::from_secs(proof_timeout_secs),
        Arc::new(tokio::sync::Semaphore::new(proof_semaphore_permits)),
        db_pool,
    );

    // Setup health check server (Threat #38 mitigation)
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<DockerConductorServer<ShadowWorkerService>>()
        .await;
    health_reporter
        .set_serving::<ProofVerifierServer<proof_service::OxiLeanProofService>>()
        .await;

    info!("Shadow Worker listening on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(DockerConductorServer::new(worker))
        .add_service(ProofVerifierServer::new(proof_service))
        .serve(addr)
        .await?;

    Ok(())
}
