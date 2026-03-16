/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

pub mod bridge_trait;
pub mod discord;
pub mod telegram;

pub use bridge_trait::ChannelBridge;
pub use discord::DiscordBridge;
pub use telegram::TelegramBridge;
