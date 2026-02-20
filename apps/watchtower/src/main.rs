use poise::serenity_prelude as serenity;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use shared::watchtower::{ControlCommand, CoreEvent, SystemStatus, LogEntry};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};
use bytes::Bytes;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use anyhow::Context as _; // Import trait for .context() method

use serenity::all::{ChannelId, CreateMessage, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage};

struct Data {
    cmd_tx: mpsc::Sender<ControlCommand>,
    latest_status: Arc<Mutex<Option<SystemStatus>>>,
    log_channel_id: ChannelId,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type PoiseContext<'a> = poise::Context<'a, Data, Error>;

/// Checking Core status
#[poise::command(slash_command)]
async fn status(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let status_guard = ctx.data().latest_status.lock().await;
    match &*status_guard {
        Some(s) => {
            let msg = format!(
                "🟢 **System Online**\nCPU: {:.1}%\nRAM: {}MB\nVRAM: {}MB\nJob: {:?}",
                s.cpu_usage, s.memory_used_mb, s.vram_used_mb, s.active_job_id
            );
            ctx.say(msg).await?;
        }
        None => {
            ctx.say("🔴 **Core Unreachable** (No Heartbeat)").await?;
        }
    }
    Ok(())
}

/// EMERGENCY KILL SWITCH (The Nuke Button)
#[poise::command(slash_command, owners_only)]
async fn nuke(ctx: PoiseContext<'_>) -> Result<(), Error> {
    ctx.say("⚠️ INITIATING EMERGENCY SHUTDOWN Protocol...").await?;
    let pid_str = std::fs::read_to_string("/tmp/aiome.id")
        .map_err(|e| Box::new(e) as Error)?; 
    let pid: i32 = pid_str.trim().parse()?;
    match signal::kill(Pid::from_raw(-pid), Signal::SIGKILL) {
        Ok(_) => {
            ctx.say(format!("✅ Target Destroyed (PGID: -{}). System halted.", pid)).await?;
            info!("💀 Executed NUKE command on PGID -{}", pid);
        }
        Err(e) => {
            ctx.say(format!("❌ NUKE FAILED: {}", e)).await?;
            error!("Failed to kill PGID -{}: {}", pid, e);
        }
    }
    Ok(())
}

/// Start a new video generation task
#[poise::command(slash_command)]
async fn generate(
    ctx: PoiseContext<'_>,
    #[description = "Category (e.g. tech, nature)"] category: String,
    #[description = "Topic/Theme"] topic: String,
    #[description = "Style Preset"] style: Option<String>,
) -> Result<(), Error> {
    ctx.say(format!("🚀 Dispatching Generate Request: **{}** ({})", topic, category)).await?;
    let cmd = ControlCommand::Generate { category, topic, style };
    if let Err(e) = ctx.data().cmd_tx.send(cmd).await {
        ctx.say(format!("❌ Failed to send command to Core loop: {}", e)).await?;
    } else {
        ctx.say("✅ Request queued for Core.").await?;
    }
    Ok(())
}

// ... event handler ...


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    
    let token = std::env::var("DISCORD_TOKEN").context("Missing DISCORD_TOKEN")?;
    let log_channel_id: u64 = std::env::var("DISCORD_LOG_CHANNEL_ID")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);

    let latest_status = Arc::new(Mutex::new(None));
    let (event_tx, mut event_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<ControlCommand>(100);

    // UDS Loop
    let status_clone = latest_status.clone();
    tokio::spawn(async move {
        loop {
            match UnixStream::connect("/tmp/aiome.sock").await {
                Ok(stream) => {
                    info!("🔗 Connected to Core.");
                    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
                    loop {
                        tokio::select! {
                            // 1. Core -> Bot
                            msg = framed.next() => {
                                match msg {
                                    Some(Ok(bytes)) => {
                                        if let Ok(event) = serde_json::from_slice::<CoreEvent>(&bytes) {
                                            match event {
                                                CoreEvent::Heartbeat(s) => { *status_clone.lock().await = Some(s); }
                                                _ => { let _ = event_tx.send(event).await; }
                                            }
                                        }
                                    }
                                    _ => break, // Reconnect
                                }
                            }
                            // 2. Bot -> Core
                            Some(cmd) = cmd_rx.recv() => {
                                let json = serde_json::to_vec(&cmd).unwrap_or_default();
                                if let Err(e) = framed.send(Bytes::from(json)).await {
                                    error!("❌ UDS Write Error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(_) => tokio::time::sleep(tokio::time::Duration::from_secs(5)).await,
            }
        }
    });

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![status(), nuke(), generate()],
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move {
                    if let serenity::FullEvent::InteractionCreate { interaction } = event {
                        if let Some(it) = interaction.as_message_component() {
                            if it.data.custom_id.starts_with("approve_") || it.data.custom_id.starts_with("reject_") {
                                let approved = it.data.custom_id.starts_with("approve_");
                                let uuid_str = it.data.custom_id.split('_').nth(1).unwrap_or("");
                                if let Ok(tid) = uuid::Uuid::parse_str(uuid_str) {
                                    let cmd = ControlCommand::ApprovalResponse { transition_id: tid, approved };
                                    let _ = data.cmd_tx.send(cmd).await;
                                    let _ = it.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .content(format!("{} **{}**", if approved { "✅ Approved" } else { "❌ Rejected" }, tid))
                                            .components(vec![])
                                    )).await;
                                }
                            }
                        }
                    }
                    Ok(())
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let data = Data { 
                    cmd_tx, 
                    latest_status, 
                    log_channel_id: ChannelId::new(log_channel_id) 
                };
                
                // Event Forwarder with Throttling
                let http = ctx.http.clone();
                let log_chan = data.log_channel_id;
                tokio::spawn(async move {
                    let mut buffer: Vec<LogEntry> = Vec::new();
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
                    loop {
                        tokio::select! {
                            Some(event) = event_rx.recv() => {
                                match event {
                                    CoreEvent::Log(l) => {
                                        buffer.push(l);
                                        if buffer.len() > 10 { // Flush if buffer large
                                            flush_logs(&mut buffer, log_chan, &http).await;
                                        }
                                    }
                                    CoreEvent::ApprovalRequest { transition_id, description } => {
                                        let msg = CreateMessage::new()
                                            .content(format!("🚨 **Approval Required**\n{}", description))
                                            .button(CreateButton::new(format!("approve_{}", transition_id)).label("✅ Approve").style(serenity::ButtonStyle::Success))
                                            .button(CreateButton::new(format!("reject_{}", transition_id)).label("❌ Reject").style(serenity::ButtonStyle::Danger));
                                        let _ = log_chan.send_message(&http, msg).await;
                                    }
                                    _ => {}
                                }
                            }
                            _ = interval.tick() => {
                                flush_logs(&mut buffer, log_chan, &http).await;
                            }
                        }
                    }
                });

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .await;

    client.unwrap().start().await.context("Serenity error")
}

async fn flush_logs(buffer: &mut Vec<LogEntry>, channel: ChannelId, http: &Arc<serenity::Http>) {
    if buffer.is_empty() { return; }
    let mut content = String::from("🗒️ **Core Logs**\n```\n");
    for log in buffer.drain(..) {
        let line = format!("[{}] {}\n", log.level, log.message);
        if content.len() + line.len() > 1900 { // Discord limit
            content.push_str("```");
            let _ = channel.say(http, &content).await;
            content = String::from("```\n");
        }
        content.push_str(&line);
    }
    content.push_str("```");
    let _ = channel.say(http, &content).await;
}
