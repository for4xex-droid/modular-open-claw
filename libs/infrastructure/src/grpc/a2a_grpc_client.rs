use aiome_contracts::a2a::{
    internal::docker_conductor_client::DockerConductorClient, A2aClient, A2aTaskProgress,
    A2aTaskRequest,
};
use aiome_contracts::error::AiomeError;
use async_stream::stream;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};
use tracing::{error, info};

#[derive(Clone)]
pub struct GrpcClientConfig {
    pub endpoint_url: String,
    pub connect_timeout: Duration,
    pub auth_token: String,
}

pub struct A2aGrpcClient {
    config: GrpcClientConfig,
}

impl A2aGrpcClient {
    pub fn new(config: GrpcClientConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl A2aClient for A2aGrpcClient {
    async fn execute_task(
        &self,
        request: A2aTaskRequest,
    ) -> Result<BoxStream<'static, Result<A2aTaskProgress, AiomeError>>, AiomeError> {
        let config_clone = self.config.clone();
        let request_clone = request.clone();

        // async-stream パッケージを利用して、ストリームを BoxStream として返す
        let stream = async_stream::try_stream! {
            let endpoint = Endpoint::from_shared(config_clone.endpoint_url.clone())
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Invalid endpoint URL: {}", e),
                })?;

            let channel = endpoint
                .timeout(config_clone.connect_timeout)
                .connect()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to connect to Shadow Clone: {}", e),
                })?;

            let mut client = DockerConductorClient::new(channel);

            let grpc_req = tonic::Request::new(aiome_contracts::a2a::internal::ExecuteTaskRequest {
                job_id: request_clone.job_id,
                prompt_b64: request_clone.prompt_b64,
                artifact_path: request_clone.artifact_path.unwrap_or_default(),
            });

            match client.execute_task(grpc_req).await {
                Ok(response) => {
                    let mut inner_stream = response.into_inner();
                    while let Ok(Some(progress)) = inner_stream.message().await {
                        yield A2aTaskProgress {
                            message: progress.message,
                            percent: progress.percent,
                            is_completed: progress.is_completed,
                            is_failed: progress.is_failed,
                            result: if progress.result.is_empty() { None } else { Some(progress.result) },
                            error: if progress.error.is_empty() { None } else { Some(progress.error) },
                        };
                    }
                }
                Err(status) => {
                    Err(AiomeError::Infrastructure {
                        reason: format!("gRPC execute_task failed: {}", status),
                    })?;
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn cancel_task(&self, _job_id: &str) -> Result<(), AiomeError> {
        // Shadow Clone のキャンセルは現状 docker stop に委譲するためダミー成功を返す（Phase 50 設計通り）
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::a2a::internal::{
        docker_conductor_server::{DockerConductor, DockerConductorServer},
        ExecuteTaskRequest as ProtoExecuteTaskRequest, TaskProgress as ProtoTaskProgress,
    };
    use futures::StreamExt;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::ReceiverStream;
    use tonic::{Request, Response, Status};

    struct MockDockerConductor;

    #[tonic::async_trait]
    impl DockerConductor for MockDockerConductor {
        type ExecuteTaskStream = ReceiverStream<Result<ProtoTaskProgress, Status>>;

        async fn execute_task(
            &self,
            request: Request<ProtoExecuteTaskRequest>,
        ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
            let req = request.into_inner();
            assert_eq!(req.job_id, "job-123");

            let (tx, rx) = tokio::sync::mpsc::channel(4);

            tokio::spawn(async move {
                tx.send(Ok(ProtoTaskProgress {
                    message: "Starting".into(),
                    percent: 10,
                    is_completed: false,
                    is_failed: false,
                    result: "".into(),
                    error: "".into(),
                }))
                .await
                .unwrap();

                tx.send(Ok(ProtoTaskProgress {
                    message: "Done".into(),
                    percent: 100,
                    is_completed: true,
                    is_failed: false,
                    result: "Success".into(),
                    error: "".into(),
                }))
                .await
                .unwrap();
            });

            Ok(Response::new(ReceiverStream::new(rx)))
        }
    }

    #[tokio::test]
    async fn test_grpc_client_execute_task_success() {
        // Start Mock Server on arbitrary port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let endpoint_url = format!("http://{}", addr);

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(DockerConductorServer::new(MockDockerConductor))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        // Test Client
        let config = GrpcClientConfig {
            endpoint_url,
            connect_timeout: Duration::from_secs(1),
            auth_token: "test-token".to_string(),
        };
        let client = A2aGrpcClient::new(config);

        let req = A2aTaskRequest {
            job_id: "job-123".into(),
            prompt_b64: "base64prompt".into(),
            artifact_path: None,
        };

        let result = client.execute_task(req).await;
        assert!(result.is_ok(), "execute_task failed to connect");

        let mut stream = result.unwrap();

        // Receive first message
        let msg1_result = stream.next().await.expect("Expected stream message 1");
        let msg1 = msg1_result.expect("Expected successful progress");
        assert_eq!(msg1.percent, 10);
        assert_eq!(msg1.message, "Starting");
        assert!(!msg1.is_completed);

        // Receive second message
        let msg2_result = stream.next().await.expect("Expected stream message 2");
        let msg2 = msg2_result.expect("Expected successful progress");
        assert_eq!(msg2.percent, 100);
        assert_eq!(msg2.message, "Done");
        assert!(msg2.is_completed);
        assert_eq!(msg2.result.unwrap(), "Success");

        // End of stream
        assert!(stream.next().await.is_none());
    }
}
