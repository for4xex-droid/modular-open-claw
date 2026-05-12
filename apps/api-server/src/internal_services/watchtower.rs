/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::agent_engine::AgentEngine;
use crate::AppState;
use aiome_core_contracts::events::{ControlCommand, CoreEvent};
use infrastructure::channel_bridge::{ChannelBridge, DiscordBridge, TelegramBridge};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Watchtower サービスを起動する。
/// 外部チャットブリッジを管理し、システムイベントの配信とコマンドの処理を行う。
pub async fn run(state: AppState) -> anyhow::Result<()> {
    info!("👁️ [Watchtower] Initializing Unified Watchtower Service...");

    let discord_token = state.config.get_inner().discord_token.clone().map(|s| {
        use secrecy::ExposeSecret;
        s.expose_secret().to_string()
    });

    let telegram_token = state.config.get_inner().telegram_token.clone().map(|s| {
        use secrecy::ExposeSecret;
        s.expose_secret().to_string()
    });

    let (command_tx, mut command_rx) = mpsc::channel::<ControlCommand>(100);
    let mut bridges: Vec<Arc<dyn ChannelBridge>> = Vec::new();

    if let Some(token) = discord_token {
        info!("🔌 [Watchtower] Initializing Discord Bridge...");
        bridges.push(Arc::new(DiscordBridge::new(token)));
    }

    if let Some(token) = telegram_token {
        info!("🔌 [Watchtower] Initializing Telegram Bridge...");
        bridges.push(Arc::new(TelegramBridge::new(token)));
    }

    if bridges.is_empty() {
        warn!("⚠️ [Watchtower] No channel tokens found. Watchtower will run in silent mode.");
    }

    // ブリッジタスクの起動
    for bridge in &bridges {
        let b = bridge.clone();
        let tx = command_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = b.run(tx).await {
                error!("❌ [Watchtower] Bridge {} error: {:?}", b.name(), e);
            }
        });
    }

    let mut event_rx = state.event_sender.get_inner().subscribe();
    let bridges = Arc::new(bridges);

    // メインイベントループ
    loop {
        tokio::select! {
            // 1. 内部システムからのイベントをブリッジへ転送
            Ok(event) = event_rx.recv() => {
                handle_core_event(event, bridges.as_slice()).await;
            }

            // 2. ブリッジ（Discord/Telegram）からのコマンドを処理
            Some(cmd) = command_rx.recv() => {
                handle_control_command(cmd, &state, bridges.as_slice()).await;
            }
        }
    }
}

/// 内部イベントを外部ブリッジへ中継する。
async fn handle_core_event(event: CoreEvent, bridges: &[Arc<dyn ChannelBridge>]) {
    match event {
        CoreEvent::ChatResponse {
            response,
            channel_id,
            ..
        } => {
            info!(
                "📨 [Watchtower] Relaying ChatResponse to channel {}",
                channel_id
            );
            for bridge in bridges {
                let _ = bridge
                    .send_message(&channel_id.to_string(), &response)
                    .await;
            }
        }
        CoreEvent::ProactiveTalk {
            message,
            channel_id,
        } => {
            info!(
                "📨 [Watchtower] Relaying ProactiveTalk to channel {}",
                channel_id
            );
            // Default proactive channel fallback logic if needed
            let target_channel = if channel_id == 0 {
                std::env::var("DISCORD_DEFAULT_CHANNEL_ID")
                    .unwrap_or_else(|_| channel_id.to_string())
            } else {
                channel_id.to_string()
            };

            for bridge in bridges {
                let _ = bridge.send_message(&target_channel, &message).await;
            }
        }
        _ => {}
    }
}

/// ブリッジからのコマンドを処理し、エージェントに対話を促す。
async fn handle_control_command(
    cmd: ControlCommand,
    state: &AppState,
    bridges: &[Arc<dyn ChannelBridge>],
) {
    match cmd {
        ControlCommand::Chat {
            message,
            channel_id,
        }
        | ControlCommand::CommandChat {
            message,
            channel_id,
        } => {
            info!(
                "📩 [Watchtower] Handling Chat command from bridge (channel: {})",
                channel_id
            );

            // AgentEngine を使用して対話を実行
            let state_clone = state.clone();
            let prompt = message.clone();

            tokio::spawn(async move {
                match AgentEngine::chat(
                    &state_clone,
                    &prompt,
                    Some(channel_id.to_string()),
                    state_clone.system_agent_id,
                )
                .await
                {
                    Ok(reply) => {
                        info!(
                            "✅ [Watchtower] Agent replied via internal bridge: {}",
                            reply
                        );
                        // CoreEvent::ChatResponse は AgentEngine::chat 内でブロードキャストされるため、
                        // ここで手動で送信する必要はない。
                    }
                    Err(e) => {
                        error!(
                            "❌ [Watchtower] AgentEngine failed to handle bridge chat: {:?}",
                            e
                        );
                        // [reflexion] サニタイズされたエラーを送信（詳細な技術情報は伏せる）
                        let user_error =
                            "⚠️ システム一時エラーが発生しました。時間を置いて再度お試しください。";
                        let _ =
                            state_clone
                                .event_sender
                                .get_inner()
                                .send(CoreEvent::ChatResponse {
                                    response: user_error.to_string(),
                                    channel_id,
                                    resource_path: None,
                                });
                    }
                }
            });
        }
        ControlCommand::StopGracefully => {
            warn!(
                "🛑 [Watchtower] StopGracefully received. (Not implemented in integrated mode yet)"
            );
        }
        _ => {
            debug!("💡 [Watchtower] Unhandled control command: {:?}", cmd);
        }
    }
}

use tracing::debug;
