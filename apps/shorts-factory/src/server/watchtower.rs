use bytes::Bytes;
use std::sync::Arc;
use infrastructure::job_queue::SqliteJobQueue;
use factory_core::traits::JobQueue;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};
use tracing::{info, warn, error};
use shared::watchtower::{ControlCommand, CoreEvent, LogEntry};

/// Backpressure-safe Tracing Layer
pub struct LogDrain {
    sender: mpsc::Sender<CoreEvent>,
}

impl LogDrain {
    pub fn new(sender: mpsc::Sender<CoreEvent>) -> Self {
        Self { sender }
    }
}

impl<S> tracing_subscriber::Layer<S> for LogDrain
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level().to_string();
        let target = metadata.target().to_string();
        
        // Format message
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message;

        let entry = LogEntry {
            level,
            target,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Wrap in CoreEvent
        let event = CoreEvent::Log(entry);

        // The Backpressure Trap Fix: Use try_send and drop if full
        if let Err(_e) = self.sender.try_send(event) {
            // Silently drop
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

const SOCKET_PATH: &str = "/tmp/aiome.sock";

use factory_core::contracts::WorkflowRequest;

pub struct WatchtowerServer {
    log_rx: mpsc::Receiver<CoreEvent>,
    job_tx: mpsc::Sender<WorkflowRequest>,
    job_queue: Arc<SqliteJobQueue>,
}

impl WatchtowerServer {
    pub fn new(
        log_rx: mpsc::Receiver<CoreEvent>,
        job_tx: mpsc::Sender<WorkflowRequest>,
        job_queue: Arc<SqliteJobQueue>,
    ) -> Self {
        Self { log_rx, job_tx, job_queue }
    }

    pub async fn start(mut self) -> Result<(), anyhow::Error> {
        // The Orphan Socket Fix: Remove before bind
        if Path::new(SOCKET_PATH).exists() {
            let _ = std::fs::remove_file(SOCKET_PATH);
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;
        info!("🗼 Watchtower UDS Bound: {}", SOCKET_PATH);

        // Permission Hardening: 0o600
        std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o600))?;

        // The Reconnection Chasm Fix: Loop accept
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    info!("🔗 Watchtower Connected");
                    self.handle_connection(stream).await;
                    info!("Disconnection detected. Waiting for next Watchtower...");
                    // log_rx remains open, channel buffers up to 1000 logs then drops.
                }
                Err(e) => {
                    error!("❌ UDS Accept Error: {}", e);
                    // Prevent tight loop on error
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
    
    async fn handle_connection(&mut self, stream: UnixStream) {
        // The Stream Framing Fix: Use LengthDelimitedCodec
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        loop {
            tokio::select! {
                // 1. Send Events (Log or Heartbeat)
                Some(event) = self.log_rx.recv() => {
                    let json = serde_json::to_vec(&event).unwrap_or_default();
                    if let Err(e) = framed.send(Bytes::from(json)).await {
                        warn!("⚠️ Failed to send event to Watchtower: {}", e);
                        break; // Connection broken
                    }
                }
                
                // 2. Receive Commands (Watchtower -> Core)
                result = framed.next() => {
                    match result {
                        Some(Ok(bytes)) => {
                            if let Ok(cmd) = serde_json::from_slice::<ControlCommand>(&bytes) {
                                self.handle_command(cmd).await;
                            } else {
                                warn!("⚠️ Invalid command received from Watchtower");
                            }
                        }
                        Some(Err(e)) => {
                            warn!("⚠️ UDS Stream Error: {}", e);
                            break;
                        }
                        None => {
                            info!("🔌 Watchtower Disconnected (EOF)");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_command(&self, cmd: ControlCommand) {
        match cmd {
             ControlCommand::Generate { category, topic, style } => {
                 info!("📥 Received Generate Command: {} ({}) with style {}", category, topic, style.as_deref().unwrap_or("auto"));
                 let req = WorkflowRequest {
                     category,
                     topic,
                     remix_id: None,
                     skip_to_step: None,
                     style_name: style.unwrap_or_default(),
                     custom_style: None,
                 };
                 if let Err(e) = self.job_tx.send(req).await {
                     error!("❌ Failed to send WorkflowRequest to Core dispatcher: {}", e);
                 }
             }
             ControlCommand::SetCreativeRating { job_id, rating } => {
                 info!("🧘 Samsara Rating Received: job={} rating={}", job_id, rating);
                 match self.job_queue.set_creative_rating(&job_id, rating).await {
                     Ok(_) => info!("✅ Creative rating saved: job={} rating={}", job_id, rating),
                     Err(e) => error!("❌ Failed to save creative rating: {}", e),
                 }
             }
             ControlCommand::LinkSns { job_id, platform, video_id } => {
                 info!("🔗 Linking Job {} to {} video ID: {}", job_id, platform, video_id);
                 match self.job_queue.link_sns_data(&job_id, &platform, &video_id).await {
                     Ok(_) => info!("✅ SNS data linked: job={} video_id={}", job_id, video_id),
                     Err(e) => error!("❌ Failed to link SNS data: {}", e),
                 }
             }
             ControlCommand::StopGracefully => {
                 info!("🛑 Graceful shutdown requested via Watchtower");
                 std::process::exit(0);
             }
             ControlCommand::EmergencyShutdown => {
                 error!("💀 Emergency shutdown requested via Watchtower");
                 std::process::exit(1);
             }
             ControlCommand::GetStatus => {
                 info!("📊 Status request received (handled via Heartbeat)");
             }
             _ => {
                 warn!("⚠️ Unhandled command: {:?}", cmd);
             }
        }
    }
}
