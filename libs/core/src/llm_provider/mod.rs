/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! LLM Providers integration library.
//! This module coordinates and re-exports various LLM backends (Ollama, Gemini, OpenAI, Claude, etc.)
//! conforming to standard core traits.

pub use aiome_core_contracts::llm::{
    EmbeddingProvider, LlmMessage, LlmProvider, LlmRequest, LlmResponse, StopReason,
};

// --- Submodules ---
/// Abyss Vault (Key Proxy) provider implementation.
pub mod abyss_vault;
/// Anthropic Claude provider implementation.
pub mod claude;
/// Google Gemini provider implementation.
pub mod gemini;
/// LM Studio provider implementation.
pub mod lm_studio;
/// Mock provider for testing.
pub mod mock;
/// Ollama provider implementation.
pub mod ollama;
/// OpenAI provider implementation.
pub mod openai;
/// Ruri-v3 local embedding provider implementation.
pub mod ruri;

/// Gemini Interactions API implementation.
pub mod interactions;
/// Live session management for Gemini.
pub mod live_session;

#[cfg(test)]
mod tests;

// --- Public Re-exports ---
pub use abyss_vault::AbyssVaultProvider;
pub use claude::ClaudeProvider;
pub use gemini::GeminiProvider;
pub use lm_studio::LmStudioProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use ruri::RuriProvider;

#[cfg(any(test, debug_assertions))]
pub use mock::MockLlmProvider;
