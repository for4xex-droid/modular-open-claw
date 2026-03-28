use aiome_contracts::a2a::internal::{
    ExecuteTaskRequest, TaskProgress,
    docker_conductor_server::{DockerConductor, DockerConductorServer},
};
use std::env;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{error, info};
use tracing_subscriber;

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

        tokio::spawn(async move {
            // RED: Dummy execution
            info!("Executing task for job: {}", job_id);

            tx.send(Ok(TaskProgress {
                message: "Initializing worker".into(),
                percent: 0,
                is_completed: false,
                is_failed: false,
                result: "".into(),
                error: "".into(),
            }))
            .await
            .unwrap();

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            tx.send(Ok(TaskProgress {
                message: "Task completed".into(),
                percent: 100,
                is_completed: true,
                is_failed: false,
                result: "SUCCESS_DUMMY_RESULT".into(),
                error: "".into(),
            }))
            .await
            .unwrap();
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
