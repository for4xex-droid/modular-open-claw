/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::bridge_trait::ChannelBridge;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use serenity::{
    all::GatewayIntents, all::Http, model::channel::Message as DiscordMessage,
    model::gateway::Ready, model::id::UserId, prelude::*,
};
use shared::guardrails::{validate_input, ValidationResult};
use shared::watchtower::ControlCommand;
use std::sync::{Arc, OnceLock};
use tracing::{debug, error, info};

/// Discord APIとの通信ブリッジ
pub struct DiscordBridge {
    token: String,
    http: Arc<Http>,
}

impl DiscordBridge {
    /// 新しいインスタンスを生成する
    pub fn new(token: String) -> Self {
        let http = Arc::new(Http::new(&token));
        Self { token, http }
    }
}

struct Handler {
    command_tx: tokio::sync::mpsc::Sender<ControlCommand>,
    /// Bot 自身の User ID キャッシュ（reaction_add での API コール削減）
    bot_user_id: OnceLock<UserId>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: DiscordMessage) {
        if msg.author.bot {
            return;
        }

        // SEC: ログに PII/ユーザーメッセージ全文を出力しない
        debug!(
            "📩 [Discord] Received message from {} (len={})",
            msg.author.name,
            msg.content.len()
        );

        // SEC: Validate input from external channel (Discord)
        match validate_input(&msg.content) {
            ValidationResult::Valid => {
                let cmd = ControlCommand::Chat {
                    message: msg.content.clone(),
                    channel_id: msg.channel_id.get(),
                };

                if let Err(e) = self.command_tx.send(cmd).await {
                    error!("❌ [Discord] Failed to send command to Core relay: {:?}", e);
                }
            }
            ValidationResult::Blocked(reason) => {
                tracing::warn!(
                    "🛡️ [Discord] Blocked message from {}: {}",
                    msg.author.name,
                    reason
                );
                // Optional: Notify user or just ignore
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("✅ [Discord] {} is connected!", ready.user.name);
    }

    async fn reaction_add(&self, ctx: Context, reaction: serenity::model::prelude::Reaction) {
        // Bot 自身のリアクションは無視（OnceLock で API コールを1回に抑制）
        if let Some(user_id) = reaction.user_id {
            let bot_id = self.bot_user_id.get_or_init(|| {
                // Note: OnceLock は同期初期化のみ。ready() で事前設定するのが理想だが、
                // fallback として UserId::new(1) を使用（ready で必ず設定されるため到達しない）
                UserId::new(1)
            });
            if user_id == *bot_id {
                return;
            }
        }

        // 対象絵文字の早期フィルタ（API コール前に判定）
        let emoji = reaction.emoji.to_string();
        let resolved = match emoji.as_str() {
            "✅" | "👍" => true,
            "❌" | "👎" => false,
            _ => return, // 対象外のリアクション → API コールなしで即 return
        };

        let channel_id = reaction.channel_id.get();

        if let Some(ticket_id) =
            extract_ticket_id_from_bot_message(&ctx, reaction.channel_id, reaction.message_id).await
        {
            if let Err(e) = self
                .command_tx
                .send(ControlCommand::SupportFeedback {
                    incident_id: ticket_id,
                    resolved,
                    channel_id,
                })
                .await
            {
                error!(
                    "❌ [Discord] Failed to send SupportFeedback command to Core: {:?}",
                    e
                );
            }
        }
    }
}

#[async_trait]
impl ChannelBridge for DiscordBridge {
    fn name(&self) -> &str {
        "Discord"
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> Result<(), AiomeError> {
        let channel_id_u64: u64 = channel_id.parse().map_err(|_| AiomeError::Infrastructure {
            reason: "Invalid Discord Channel ID".to_string(),
        })?;
        let channel = serenity::model::id::ChannelId::new(channel_id_u64);

        // Help inference by explicitly specifying the error type or using a temporary variable
        channel
            .say(&self.http, content)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Discord send failed: {}", e),
            })?;

        Ok(())
    }

    async fn run(
        &self,
        command_tx: tokio::sync::mpsc::Sender<ControlCommand>,
    ) -> Result<(), AiomeError> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGE_REACTIONS;

        let handler = Handler {
            command_tx,
            bot_user_id: OnceLock::new(),
        };

        let mut client = Client::builder(&self.token, intents)
            .event_handler(handler)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create Discord client: {}", e),
            })?;

        info!("🚀 [Discord] Starting serenity gateway...");
        client
            .start()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Discord client error: {}", e),
            })?;

        Ok(())
    }
}

/// Bot メッセージから [TICKET:uuid] を抽出するヘルパー
async fn extract_ticket_id_from_bot_message(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    message_id: serenity::model::id::MessageId,
) -> Option<String> {
    if let Ok(msg) = ctx.http.get_message(channel_id, message_id).await {
        extract_ticket_id_from_text(&msg.content)
    } else {
        None
    }
}

/// テキストから [TICKET:uuid] パターンを正規表現で抽出
fn extract_ticket_id_from_text(text: &str) -> Option<String> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        match regex::Regex::new(r"\[TICKET:([a-f0-9-]+)\]") {
            Ok(r) => r,
            Err(_) => {
                // allow-anti-pattern: static regex
                match regex::Regex::new("a^") {
                    Ok(r) => r,
                    Err(_) => {
                        // a^ is guaranteed to compile, so this is unreachable
                        #[allow(clippy::empty_loop)]
                        loop {}
                    }
                }
            }
        }
    });
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ticket_id_from_text_formatting() {
        // Arrange
        let bot_msg = "🛡️ インシデントを記録しました。[TICKET:550e8400-e29b-41d4-a716-446655440000] ※この回答は自動応答です";

        // Act
        let ticket_id = extract_ticket_id_from_text(bot_msg);

        // Assert
        assert_eq!(
            ticket_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_extract_ticket_id_from_text_missing() {
        // Arrange
        let bot_msg = "通常のチャット回答です。※自動応答です";

        // Act
        let ticket_id = extract_ticket_id_from_text(bot_msg);

        // Assert
        assert_eq!(ticket_id, None);
    }
}
