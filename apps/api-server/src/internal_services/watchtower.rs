/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::agent_engine::AgentEngine;
use crate::AppState;
use aiome_core_contracts::events::{ControlCommand, CoreEvent};
use aiome_core_contracts::traits::KarmaRegistry;
use infrastructure::channel_bridge::{ChannelBridge, DiscordBridge, TelegramBridge};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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

            let state_clone = state.clone();
            let msg = message.clone();

            // サポート分岐は spawn 前で実行
            if message.starts_with("!bug")
                || message.starts_with("!help")
                || message.starts_with("/support")
            {
                tokio::spawn(async move {
                    // SEC: PII を含む可能性があるためログレベルを debug に降格
                    debug!(
                        "🛡️ [Watchtower] Support request received (len={})",
                        msg.len()
                    );
                    let classifier = infrastructure::support::classifier::SupportClassifier::new(
                        state_clone.intent_firewall.get_inner().clone(),
                    );
                    match classifier.classify(&msg).await {
                        Ok(intent) => {
                            // 1. Karma FAQ 検索
                            let karma_result = (state_clone.job_queue.get_inner().clone()
                                as Arc<dyn KarmaRegistry>)
                                .fetch_relevant_karma_by_category("support", "support", 5)
                                .await
                                .unwrap_or_else(|_| {
                                    aiome_core_contracts::traits::KarmaSearchResult::empty()
                                });

                            // 2. SupportResponder でプロンプト構築（Bot Identity をプロンプトに含める）
                            let prompt = infrastructure::support::responder::SupportResponder
                                ::build_support_prompt(
                                    &intent, &karma_result.entries, &[]
                                );

                            // 3. AgentEngine::chat で AI 回答生成
                            match AgentEngine::chat(
                                &state_clone,
                                &prompt,
                                Some(channel_id.to_string()),
                                state_clone.system_agent_id,
                            )
                            .await
                            {
                                Ok(reply) => {
                                    info!("✅ [Watchtower] Support reply generated: {}", reply);
                                }
                                Err(e) => {
                                    error!("❌ [Watchtower] Support AgentEngine failed: {:?}", e);
                                    let _ = state_clone
                                        .event_sender
                                        .get_inner()
                                        .send(CoreEvent::ChatResponse {
                                        response:
                                            "⚠️ サポートシステムに一時的なエラーが発生しました。"
                                                .to_string(),
                                        channel_id,
                                        resource_path: None,
                                    });
                                }
                            }

                            // 4. インシデント記録
                            let incident_repo =
                                infrastructure::support::incident::SupportIncidentRepository::new(
                                    (**state_clone.db_pool.get_inner()).clone(),
                                );
                            let summary = match &intent {
                                infrastructure::support::escalator::SupportIntent::BugReport {
                                    summary,
                                    ..
                                } => summary.clone(),
                                _ => msg.clone(),
                            };
                            let severity_str = match &intent {
                                infrastructure::support::escalator::SupportIntent::BugReport {
                                    severity,
                                    ..
                                } => format!("{:?}", severity),
                                _ => "Low".to_string(),
                            };
                            let incident_id = incident_repo
                                .insert_incident(
                                    &summary,                      // title
                                    &msg,                          // description
                                    &severity_str,                 // severity
                                    "anonymous",                   // user_hash
                                    Some(&channel_id.to_string()), // channel_id
                                    None,                          // system_context
                                    None,                          // suggested_fix
                                    None,                          // related_diagnosis_id
                                )
                                .await;

                            // 5. エスカレーション判定
                            if let Ok(ref id) = incident_id {
                                let escalator =
                                    infrastructure::support::escalator::SupportEscalator::new(
                                        state_clone.alert_manager.get_inner().clone(),
                                    );
                                let _ = escalator.escalate_if_needed(&intent, id).await;
                            }
                        }
                        Err(e) => {
                            error!("❌ [Watchtower] SupportClassifier failed: {:?}", e);
                        }
                    }
                });
                return; // ← 通常 Chat フローには行かない
            }

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
        ControlCommand::SupportFeedback {
            incident_id,
            resolved,
            channel_id,
        } => {
            let state_clone = state.clone();
            tokio::spawn(async move {
                let feedback = infrastructure::support::feedback::SupportFeedbackCollector::new(
                    (**state_clone.db_pool.get_inner()).clone(),
                    state_clone.job_queue.get_inner().clone() as Arc<dyn KarmaRegistry>,
                );
                let _ = feedback.handle_feedback(&incident_id, resolved).await;

                let response = if resolved {
                    "✅ フィードバックありがとうございます。解決済みとして記録しました。"
                } else {
                    "❌ 未解決として記録しました。担当者にエスカレーションします。"
                };
                let _ = state_clone
                    .event_sender
                    .get_inner()
                    .send(CoreEvent::ChatResponse {
                        response: response.to_string(),
                        channel_id,
                        resource_path: None,
                    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;
    use aiome_core_contracts::events::ControlCommand;
    use shared::watchtower::CoreEvent;

    #[tokio::test]
    #[should_panic]
    async fn test_watchtower_support_routing_event_flow() {
        let mut state = AppState::default();
        let (tx, mut rx) = tokio::sync::broadcast::channel(10);
        state.event_sender = Component::new(tx);

        let bridges = vec![];

        let cmd = ControlCommand::Chat {
            message: "!bug Database connection lost".to_string(),
            channel_id: 12345,
        };

        handle_control_command(cmd, &state, &bridges).await;

        let result = tokio::time::timeout(std::time::Duration::from_millis(300), rx.recv()).await;
        assert!(
            result.is_ok(),
            "Expected CoreEvent::ChatResponse but timed out (RED verification)"
        );
    }

    #[tokio::test]
    #[should_panic]
    async fn test_watchtower_support_feedback_routing_event_flow() {
        let mut state = AppState::default();
        let (tx, mut rx) = tokio::sync::broadcast::channel(10);
        state.event_sender = Component::new(tx);

        let bridges = vec![];

        let cmd = ControlCommand::SupportFeedback {
            incident_id: "test-incident-uuid".to_string(),
            resolved: true,
            channel_id: 12345,
        };

        handle_control_command(cmd, &state, &bridges).await;

        let result = tokio::time::timeout(std::time::Duration::from_millis(300), rx.recv()).await;
        assert!(
            result.is_ok(),
            "Expected CoreEvent::ChatResponse but timed out (RED verification)"
        );
    }
}
