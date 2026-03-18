/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use async_trait::async_trait;

#[async_trait]
/// 外部チャットプラットフォームとの通信インターフェース
pub trait ChannelBridge: Send + Sync {
    /// ブリッジの名前 (Discord, Telegram等)
    fn name(&self) -> &str;

    /// メッセージを送信
    async fn send_message(&self, channel_id: &str, content: &str) -> Result<(), AiomeError>;

    /// 接続を開始し、イベントループに入る
    async fn run(
        &self,
        command_tx: tokio::sync::mpsc::Sender<shared::watchtower::ControlCommand>,
    ) -> Result<(), AiomeError>;
}
