/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_contracts::a2a::internal::{
    ExecuteTaskRequest, TaskProgress,
    docker_conductor_server::{DockerConductor, DockerConductorServer},
};
use aiome_core::llm_provider::{GeminiProvider, LlmProvider, OllamaProvider};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::env;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};
use tracing::info;

pub struct ShadowWorkerService {
    auth_token: String,
}

#[tonic::async_trait]
impl DockerConductor for ShadowWorkerService {
    type ExecuteTaskStream = ReceiverStream<Result<TaskProgress, Status>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        // Token verification (Threat #36 mitigation)
        let token = match request.metadata().get("authorization") {
            Some(t) => t.to_str().unwrap_or(""),
            None => return Err(Status::unauthenticated("Missing authorization metadata")),
        };

        if !token.ends_with(&self.auth_token) {
            return Err(Status::unauthenticated("Invalid auth token"));
        }

        let req = request.into_inner();
        info!("Received task request: job_id={}", req.job_id);

        let (tx, rx) = mpsc::channel(4);
        let job_id = req.job_id.clone();
        let prompt_b64 = req.prompt_b64.clone();

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
            let llm_res = match env::var("GEMINI_API_KEY") {
                Ok(key) if !key.is_empty() => {
                    let provider = GeminiProvider::new(
                        reqwest::Client::new(),
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
    tracing_subscriber::fmt::init();

    info!("Starting Shadow Worker...");

    // Port and Token configuration
    let port = env::var("SHADOW_CLONE_GRPC_PORT").unwrap_or_else(|_| "50051".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    let auth_token =
        env::var("A2A_AUTH_TOKEN").expect("A2A_AUTH_TOKEN environment variable is required");

    let worker = ShadowWorkerService { auth_token };

    // Setup health check server (Threat #38 mitigation)
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<DockerConductorServer<ShadowWorkerService>>()
        .await;

    info!("Shadow Worker listening on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(DockerConductorServer::new(worker))
        .serve(addr)
        .await?;

    Ok(())
}
