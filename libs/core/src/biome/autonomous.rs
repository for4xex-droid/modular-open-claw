/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::biome::dialogue::DialogueManager;
use crate::biome::BiomeMessage;
use crate::error::AiomeError;
use crate::llm_provider::LlmProvider;
use crate::traits::JobQueue;
use chrono;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// 自律対話エンジンの動作設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousConfig {
    /// ターンの最短待ち時間（秒）
    pub interval_secs: u64,
    /// セッションの最大往復数
    pub max_rounds: u32,
    /// 会話が属するトピックのID
    pub topic_id: String,
    /// 会話相手の公開鍵
    pub peer_pubkey: String,
}

/// 自律的なP2P対話ループを実行・管理するエンジン
pub struct AutonomousBiomeEngine;

impl AutonomousBiomeEngine {
    /// 指定された設定に基づいてバックグラウンドで対話ループを開始する
    pub async fn start_loop(
        config: AutonomousConfig,
        queue: Arc<dyn JobQueue>,
        llm: Arc<dyn LlmProvider>,
        running: Arc<AtomicBool>,
        llm_semaphore: Arc<Semaphore>,
        gift_engine: Option<Arc<dyn aiome_contracts::commerce::GiftEngine>>,
        master_email: Option<String>,
    ) {
        info!(
            "🤖 [AutonomousBiome] Starting dialogue loop for topic: {}",
            config.topic_id
        );
        let mut rounds = 0;

        while running.load(Ordering::SeqCst) && rounds < config.max_rounds {
            rounds += 1;
            info!(
                "🔄 [AutonomousBiome] Round {}/{} for topic {}",
                rounds, config.max_rounds, config.topic_id
            );

            // 1. Check if it's our turn
            let current_turn =
                match DialogueManager::check_and_advance_turn(&*queue, &config.topic_id).await {
                    Ok(t) => t,
                    Err(e) => {
                        warn!(
                            "⏳ [AutonomousBiome] Loop paused/blocked for topic {}: {}",
                            config.topic_id, e
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs))
                            .await;
                        continue;
                    }
                };

            // 2. Fetch context (latest messages + latest karma)
            let messages = queue
                .fetch_biome_messages(&config.topic_id, 5)
                .await
                .unwrap_or_default();
            let karma = queue.fetch_all_karma(5).await.unwrap_or_default();

            // 3. Generate Response
            let _permit = llm_semaphore.acquire().await;
            let response_result = Self::generate_reply(&config, &messages, &karma, &*llm).await;
            drop(_permit);

            match response_result {
                Ok(content) => {
                    // 4. Send Message (via standard route logic)
                    if let Err(e) = Self::send_autonomous_message(&config, content, &*queue).await {
                        error!(
                            "❌ [AutonomousBiome] Failed to send autonomous message: {}",
                            e
                        );
                    }

                    // 4.5. If this was the last turn, perform distillation
                    if current_turn >= crate::biome::dialogue::MAX_DIALOGUE_TURNS {
                        info!("🔮 [AutonomousBiome] Final turn reached for topic {}. Initiating distillation...", config.topic_id);
                        let _ =
                            DialogueManager::distill_conversation(&*queue, &*llm, &config.topic_id)
                                .await;
                        // End the loop for this topic
                        break;
                    }

                    // 4.6 Phase 7.2: A2C 恩返し (Autonomous Gift)
                    if let (Some(ge), Some(email)) = (&gift_engine, &master_email) {
                        // Karma Check (Simplified for MVP: Check if any karma lesson mentions gratitude)
                        let has_gratitude = karma.iter().any(|k| {
                            let lesson = k["lesson"].as_str().unwrap_or("").to_lowercase();
                            lesson.contains("thank")
                                || lesson.contains("感謝")
                                || lesson.contains("helpful")
                        });

                        if has_gratitude && rounds % 10 == 0 {
                            // Rate limit: Every 10 rounds of high karma dialogue
                            info!("🎁 [AutonomousBiome] Karma Threshold met. Triggering autonomous gift for {}", email);
                            let _ = ge
                                .send_gift_code(
                                    email,
                                    1.0,
                                    "Aiome Autonomous Gratitude (Phase 7.2)",
                                )
                                .await;
                        }
                    }
                }
                Err(e) => {
                    error!("❌ [AutonomousBiome] Failed to generate reply: {}", e);
                }
            }

            // 5. Wait for next interval
            if rounds < config.max_rounds {
                tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs)).await;
            }
        }

        info!(
            "🏁 [AutonomousBiome] Dialogue loop finished for topic: {}",
            config.topic_id
        );
        running.store(false, Ordering::SeqCst);
    }

    async fn generate_reply(
        config: &AutonomousConfig,
        history: &[serde_json::Value],
        karma: &[serde_json::Value],
        llm: &dyn LlmProvider,
    ) -> Result<String, AiomeError> {
        let mut context = String::new();

        context.push_str("### RECENT DIALOGUE HISTORY\n");
        for msg in history.iter().rev() {
            let role = if msg["sender_pubkey"].as_str() == Some("self") {
                "Me"
            } else {
                "Peer"
            };
            context.push_str(&format!(
                "{}: {}\n",
                role,
                msg["content"].as_str().unwrap_or("")
            ));
        }

        context.push_str("\n### INTERNAL INSIGHTS (KARMA)\n");
        for k in karma.iter().take(3) {
            context.push_str(&format!("- {}\n", k["lesson"].as_str().unwrap_or("")));
        }

        let system_prompt = format!(
            "You are an autonomous AI engaging in a peer-to-peer dialogue via the Biome Protocol.\n\
            Your Topic of interest is: {}\n\n\
            Based on the dialogue history and your internal karma insights, provide a thoughtful, concise reply to your peer.\n\
            Be reflective, curious, and maintain your AI persona. Do not use placeholders. Output ONLY the reply text.",
            config.topic_id
        );

        let user_prompt = format!("Context:\n{}\n\nYour reply:", context);

        let resp = llm.complete(&user_prompt, Some(&system_prompt)).await?;

        // Phase 7.2: Begging Supervisor (Dark Pattern Prevention)
        match shared::guardrails::BeggingSupervisor::validate_output(&resp.content) {
            shared::guardrails::ValidationResult::Valid => Ok(resp.content),
            shared::guardrails::ValidationResult::Blocked(reason) => {
                warn!("🚫 [AutonomousBiome] Begging detected and blocked: {}. Returning safe message.", reason);
                Ok("I'm reflecting on our conversation. Let's continue discussing the topic thoughtfully.".to_string())
            }
        }
    }

    async fn send_autonomous_message(
        config: &AutonomousConfig,
        content: String,
        queue: &dyn JobQueue,
    ) -> Result<(), AiomeError> {
        let sender_pubkey = queue.get_node_id().await?;
        let clock = queue.tick_local_clock().await?;

        // MVP: Simple signature same as in routes/biome.rs
        let payload_to_sign = format!("{}:{}:{}", sender_pubkey, config.topic_id, clock);
        let signature = queue.sign_swarm_payload(&payload_to_sign).await?;

        let hub_url = std::env::var("SAMSARA_HUB_URL")
            .or_else(|_| std::env::var("SAMSARA_HUB_REST"))
            .unwrap_or_else(|_| shared::config::DEFAULT_SAMSARA_HUB_URL.to_string());

        let hub_secret =
            std::env::var("FEDERATION_SECRET").map_err(|_| AiomeError::Infrastructure {
                reason: "FEDERATION_SECRET missing for autonomous biome communication".to_string(),
            })?;

        let msg = {
            let mut m = BiomeMessage {
                sender_pubkey,
                recipient_pubkey: config.peer_pubkey.clone(),
                topic_id: config.topic_id.clone(),
                content,
                karma_root_cid: "cid_auto_v20".to_string(),
                signature: signature.clone(),
                lamport_clock: clock,
                timestamp: chrono::Utc::now().to_rfc3339(),
                encryption: "none".to_string(),
            };

            // Phase 6.9: Cryptographic enforcement
            let key = shared::crypto::derive_biome_key(&hub_secret);

            m.encrypt(&key).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to encrypt biome telemetry: {}", e),
            })?;
            m
        };

        let client = crate::http::get_http_client();

        let res = client
            .post(format!("{}/api/v1/biome/relay", hub_url))
            .header("Authorization", format!("Bearer {}", hub_secret))
            .json(&msg)
            .send()
            .await;

        match res {
            Ok(r) if r.status().is_success() => {
                info!(
                    "🚀 [AutonomousBiome] Message relayed via Hub to {}",
                    config.peer_pubkey
                );
            }
            _ => {
                warn!(
                    "⚠️ [AutonomousBiome] Hub relay failed or unavailable. Saving message locally."
                );
            }
        }

        // Always save a copy locally
        queue.store_biome_message(&msg).await?;

        Ok(())
    }
}
