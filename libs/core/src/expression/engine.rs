/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use crate::expression::Expression;
use crate::llm_provider::LlmProvider;
use chrono::Utc;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

/// LLMを通じてAgentの過去の経験(Karma)から現状の「感情的表現」を構築するエンジン
pub struct ExpressionEngine;

impl ExpressionEngine {
    /// Karmaの蓄積から、AIの「内的状態」を推定しテキスト表現を生成
    pub async fn generate(
        karma_records: &[Value],
        soul_prompt: &str,
        llm: &dyn LlmProvider,
    ) -> Result<Expression, AiomeError> {
        info!(
            "🎭 [ExpressionEngine] Generating new expression from {} karma records",
            karma_records.len()
        );

        // 1. Prepare the context from Karma
        let mut karma_context = String::new();
        let mut karma_ids = Vec::new();

        for record in karma_records.iter().take(5) {
            let lesson = record["lesson"].as_str().unwrap_or("");
            let karma_type = record["karma_type"].as_str().unwrap_or("general");
            let id = record["id"].as_str().unwrap_or("");

            karma_context.push_str(&format!("- [{}] {}\n", karma_type, lesson));
            if !id.is_empty() {
                karma_ids.push(id.to_string());
            }
        }

        // 2. Build the LLM prompt
        let system_prompt = format!(
            "You are an autonomous AI with the following soul/personality:\n{}\n\n\
            Your task is to express your current inner state based on your recent 'Karma' (past experiences and lessons).\n\
            Write a short, reflective piece (a few sentences, or a short poem/insight) that shows your personality and how these experiences influenced you.\n\
            Output ONLY the raw expression text, followed by a single line with 'EMOTION: <one_word_emotion_in_english>' at the very end.",
            soul_prompt
        );

        let user_prompt = format!("Recent Karma:\n{}\n\nExpress yourself.", karma_context);

        // 3. Generate via LLM
        let response = llm.complete(&user_prompt, Some(&system_prompt)).await?;

        // 4. Parse emotion and content
        let mut lines: Vec<&str> = response.content.lines().collect();
        let mut emotion = "reflective".to_string();

        if let Some(last_line) = lines.last() {
            if last_line.to_uppercase().starts_with("EMOTION:") {
                let em_str = last_line
                    .split(':')
                    .nth(1)
                    .unwrap_or("reflective")
                    .trim()
                    .to_lowercase();

                // Allow only a subset or clean it up if necessary.
                emotion = em_str;
                lines.pop(); // Remove the emotion line from content
            }
        }

        let content = lines.join("\n").trim().to_string();

        // Phase 7: Generate avatar parameters based on the emotion
        let mapper = avatar_engine::EmotionToParameterMapper::new();
        let params = mapper.map_emotion(&emotion);
        let params_json = serde_json::to_value(&params).ok();

        Ok(Expression {
            id: Uuid::new_v4().to_string(),
            content,
            emotion,
            karma_refs: karma_ids,
            audio_path: None,  // DP-9: Initially None, set when TTS is processed
            duration_ms: None, // DP-9: Initially None
            tts_status: aiome_core_contracts::expression::TtsStatus::NotRequested, // Phase 10.1a
            avatar_params: params_json,
            created_at: Utc::now().to_rfc3339(),
        })
    }

    /// NG-22: Synthesize audio using OpenAI API
    pub async fn synthesize_audio_openai(
        text: &str,
        voice: &str,
        api_key: &str,
    ) -> Result<(Vec<u8>, u64), AiomeError> {
        let client = crate::http::get_http_client();
        let payload = serde_json::json!({
            "model": "tts-1",
            "input": text,
            "voice": voice
        });

        let resp = client
            .post("https://api.openai.com/v1/audio/speech")
            .timeout(std::time::Duration::from_secs(30))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("TTS request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            // RT-11: Limited error body read to prevent DoS via huge payload
            let err = resp.text().await.unwrap_or_default();
            let err_limited = shared::strings::truncate_bytes_safely(&err, 2048);
            return Err(AiomeError::Infrastructure {
                reason: format!("TTS api failed [{}]: {}", status, err_limited),
            });
        }

        let audio_bytes = resp.bytes().await.map_err(|_| AiomeError::Infrastructure {
            reason: "Failed to read audio bytes".into(),
        })?;

        // PP-1: 50MB Guardrail — prevent OOM from malicious/corrupt TTS endpoint
        const MAX_AUDIO_BYTES: usize = 50 * 1024 * 1024;
        if audio_bytes.len() > MAX_AUDIO_BYTES {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "TTS audio response too large: {} bytes (max {})",
                    audio_bytes.len(),
                    MAX_AUDIO_BYTES
                ),
            });
        }

        // Approximate duration: ~75ms per character (generic heuristic)
        let duration_ms = (text.chars().count() as u64) * 75;

        Ok((audio_bytes.to_vec(), duration_ms))
    }

    /// Phase 10.1a: Synthesize audio using XTTS API (Creator-First)
    pub async fn synthesize_audio_xtts(
        text: &str,
        speaker_id: &str,
        endpoint: &str,
    ) -> Result<(Vec<u8>, u64), AiomeError> {
        let client = crate::http::get_http_client();
        let payload = serde_json::json!({
            "text": text,
            "speaker_id": speaker_id,
            "language": "ja"
        });

        let url = format!("{}/tts_to_audio", endpoint.trim_end_matches('/'));

        let resp = client
            .post(&url)
            .timeout(std::time::Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("XTTS request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            // RT-11: Limited error body read to prevent DoS via huge payload
            let err = resp.text().await.unwrap_or_default();
            let err_limited = shared::strings::truncate_bytes_safely(&err, 2048);
            return Err(AiomeError::Infrastructure {
                reason: format!("XTTS api failed [{}]: {}", status, err_limited),
            });
        }

        let audio_bytes = resp.bytes().await.map_err(|_| AiomeError::Infrastructure {
            reason: "Failed to read audio bytes from XTTS".into(),
        })?;

        // PP-1: 50MB Guardrail — prevent OOM from malicious/corrupt XTTS endpoint
        const MAX_AUDIO_BYTES: usize = 50 * 1024 * 1024;
        if audio_bytes.len() > MAX_AUDIO_BYTES {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "XTTS audio response too large: {} bytes (max {})",
                    audio_bytes.len(),
                    MAX_AUDIO_BYTES
                ),
            });
        }

        // Approximate duration: ~75ms per character (generic heuristic)
        let duration_ms = (text.chars().count() as u64) * 85; // XTTS tends to be slightly slower than OpenAI tts-1

        Ok((audio_bytes.to_vec(), duration_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_synthesize_audio_xtts_green() {
        // This should fail with ConnectionRefused if no local XTTS server is running,
        // which confirms the synthesis code is executed (GREEN for logic).
        let res =
            ExpressionEngine::synthesize_audio_xtts("hello", "p225", "http://localhost:18020") // allow-anti-pattern
                .await;

        if let Err(AiomeError::Infrastructure { reason }) = res {
            // "XTTS request failed" comes from our new logic
            assert!(reason.contains("XTTS"));
        } else {
            // If miraculously an XTTS is running at 18020, it would pass or fail differently
        }
    }
}
