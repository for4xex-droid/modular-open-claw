/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/// `bridge_trait` モジュール
pub mod bridge_trait;
/// `discord` モジュール
pub mod discord;
/// `telegram` モジュール
pub mod telegram;

pub use bridge_trait::ChannelBridge;
pub use discord::DiscordBridge;
pub use telegram::TelegramBridge;
