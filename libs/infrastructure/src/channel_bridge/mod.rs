/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

/// 自動補完モジュール
pub mod bridge_trait;
/// 自動補完モジュール
pub mod discord;
/// 自動補完モジュール
pub mod telegram;

pub use bridge_trait::ChannelBridge;
pub use discord::DiscordBridge;
pub use telegram::TelegramBridge;
