/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::DreamState;
use aiome_core_contracts::events::CoreEvent;
use aiome_core_contracts::traits::JobQueue;
use rand::Rng;
use std::error::Error;
use tracing::{info, warn};
use uuid::Uuid;

impl DreamState {
    /// Biome進化夢想処理 (Phase 3)
    pub(super) async fn biome_evolution_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Biome Evolution Contemplation...");

        let biome_engine = match &self.biome_engine {
            Some(engine) => engine,
            None => return Ok(None),
        };

        // 24時間以内のカルマから突然変異ブースト (1.0x 〜 2.0x) を算出
        let domain_counts = job_queue
            .count_karma_by_domains_since(
                &["news", "commune", "nurture", "metaverse"],
                chrono::Utc::now() - chrono::Duration::hours(24),
            )
            .await?;
        let mut boost = 1.0_f32;
        for domain in &["news", "commune", "nurture", "metaverse"] {
            if domain_counts.get(*domain).copied().unwrap_or(0) >= 1 {
                boost += 0.25;
            }
        }

        // 1. BiomeEngine をロックしてブーストを適用し、tick() を実行
        let mut engine = biome_engine.write().await;
        engine.set_mutation_boost(boost.min(2.0));
        engine.tick();
        let gen = engine.generation();
        let rarity = engine.get_rarity();
        let substance = engine.roll_substance();
        drop(engine); // ロックを早期解放

        // 2. LLM を用いて進化の洞察を生成する
        let prompt = format!(
            "You are observing a cellular automaton simulation called Biome at generation {}. \
            Generate a short, creative observation log describing the current evolutionary status, \
            any minor crisis or Apex species emerging, and a brief recommendation for the agent's trait development.\n\
            Output a JSON block in this format:\n\
            {{\n\
              \"message\": \"observation log string\",\n\
              \"rarity\": \"Common|Rare|Legendary\",\n\
              \"recommendation\": \"trait target recommendation\"\n\
            }}",
            gen
        );

        let resp_result = self
            .llm
            .complete(
                &prompt,
                Some("You are a Biome Ecologist AI. Generate creative simulation insights."),
            )
            .await;

        let (message, recommendation) = match resp_result {
            Ok(resp) => {
                if let Ok(json_str) = crate::llm::utils::extract_json(&resp.content) {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str.as_ref()) {
                        let msg = data["message"]
                            .as_str()
                            .unwrap_or("Stable biome environment observed steadily.")
                            .to_string();
                        let rec = data["recommendation"].as_str().map(|s| s.to_string());
                        (msg, rec)
                    } else {
                        (
                            "Stable biome environment observed steadily.".to_string(),
                            None,
                        )
                    }
                } else {
                    (
                        "Stable biome environment observed steadily.".to_string(),
                        None,
                    )
                }
            }
            Err(e) => {
                warn!(
                    "⚠️ [DreamState] LLM failed in biome dream: {}. Using fallback.",
                    e
                );
                (
                    "Stable biome environment observed steadily.".to_string(),
                    None,
                )
            }
        };

        let rarity_str = match rarity {
            biome_engine::rarity::BiomeRarity::Legendary => "Legendary",
            biome_engine::rarity::BiomeRarity::Epic => "Epic",
            biome_engine::rarity::BiomeRarity::Rare => "Rare",
            biome_engine::rarity::BiomeRarity::Uncommon => "Uncommon",
            biome_engine::rarity::BiomeRarity::Common => "Common",
        }
        .to_string();

        info!(
            "🧬 [DreamState] Biome Gen {} - Rarity: {} - Msg: {}",
            gen, rarity_str, message
        );

        let run_id = Uuid::new_v4().to_string();

        // 3. Enqueue a system job to satisfy the FOREIGN KEY constraint in karma_logs.job_id
        let system_job_id = job_queue
            .enqueue(
                "data_processing",
                &format!("Biome Evolution Gen {}", gen),
                "auto",
                None,
                None,
                None,
                0,
            )
            .await?;
        job_queue
            .update_job_status(
                &system_job_id,
                aiome_core_contracts::traits::JobStatus::Completed,
            )
            .await?;

        // 4. CoreEvent::BiomeEvolution の配信
        if let Some(ref sender) = self.event_sender {
            let event = CoreEvent::BiomeEvolution {
                run_id: run_id.clone(),
                generation: gen,
                message: message.clone(),
                rarity: Some(rarity_str.clone()),
                recommendation: recommendation.clone(),
            };
            if let Err(e) = sender.send(event) {
                tracing::warn!(
                    "⚠️ [DreamState] Failed to broadcast BiomeEvolution event: {}",
                    e
                );
            }
        }

        // 5. Soul のロード
        let agent_id = job_queue.get_system_agent_id().await?;
        let soul_opt = job_queue.load_soul(&agent_id.to_string()).await?;

        let soul_hash = if let Some(ref soul_val) = soul_opt {
            soul_val["soul_hash"]
                .as_str()
                .unwrap_or("default_soul_hash")
                .to_string()
        } else {
            "default_soul_hash".to_string()
        };

        // 6. KarmaRegistry 書き込み (domain = "biome")
        if let Err(e) = job_queue
            .store_karma(
                &system_job_id,
                "biome",
                &format!("Gen {}: {}", gen, message),
                "Synthesized",
                &soul_hash,
                Some("biome"),
                Some("evolution"),
                None,
                false,
            )
            .await
        {
            warn!("⚠️ [DreamState] Failed to store biome karma: {}", e);
        }

        // 7. Soul への経験プッシュ・PredictiveModel 更新・永続化 (P3-13)
        if let Some(ref store) = self.soul_store {
            if let Some(soul_val) = soul_opt {
                match serde_json::from_value::<soul::AgentSoul>(soul_val) {
                    Ok(mut soul) => {
                        // Higgs粒子による形質の固定（ヒッグス凍結処理）
                        if substance == biome_engine::particle::SubstanceKind::Higgs {
                            let trait_idx = rand::thread_rng().gen_range(0..32);
                            let marker_id = format!("higgs-{}", uuid::Uuid::new_v4());

                            // SomaticMarker の追加
                            let marker = soul::somatic::SomaticMarker {
                                id: marker_id.clone(),
                                embedding: vec![0.0; 32],
                                valence: 0.8,
                                arousal: 0.8,
                                intensity: 1.0,
                                created_at: chrono::Utc::now().to_rfc3339(),
                                is_permanent: true,
                            };
                            soul.somatic_markers.push(marker);

                            // FrozenTraitSnapshot の追加
                            let snapshot = soul::biome_traits::FrozenTraitSnapshot {
                                trait_index: trait_idx,
                                frozen_value: 1.0,
                                somatic_marker_id: marker_id,
                                frozen_at_generation: gen,
                                created_at: chrono::Utc::now().to_rfc3339(),
                            };
                            soul.frozen_traits.push(snapshot);
                            info!("⚛️ [DreamState] Higgs particle mutation! Froze trait index {} at generation {}.", trait_idx, gen);
                        }

                        // 7a. Rarity に基づく outcome_valence 決定
                        let outcome_valence = match rarity_str.as_str() {
                            "Legendary" => 1.0,
                            "Rare" => 0.8,
                            _ => 0.5, // Common / 想定外文字列もフォールバック
                        };

                        // 7b. PredictiveModel の plasticity 更新 (タキオン連動)
                        // predicted = 0.5 (初期予測、未知のドメインなので中立)
                        // actual = outcome_valence (実際の進化結果)
                        soul.predictive_model
                            .update_plasticity("biome", outcome_valence, 0.5);

                        // 7c. Experience プッシュ (Legendary は core_memory)
                        let is_legendary = rarity_str == "Legendary";
                        let mut exp = soul::Experience::default();
                        exp.domain = "biome".to_string();
                        exp.content = format!("Biome Gen {} [{}]: {}", gen, rarity_str, message);
                        exp.outcome_valence = outcome_valence;
                        exp.original_prediction = 0.5;
                        exp.is_core_memory = is_legendary;
                        soul.push_experience(exp);

                        // 保存
                        if let Err(e) = store.save_soul(&soul).await {
                            warn!("⚠️ [DreamState] Failed to save AgentSoul after Biome evolution: {}", e);
                        } else if is_legendary {
                            info!("🏆 [DreamState] Saved Legendary Biome experience to AgentSoul!");
                        } else {
                            info!("🧬 [DreamState] Updated AgentSoul with Biome evolution data (plasticity + experience)");
                        }
                    }
                    Err(e) => {
                        warn!(
                            "⚠️ [DreamState] Failed to deserialize AgentSoul for Biome update: {}",
                            e
                        );
                    }
                }
            } else {
                warn!("⚠️ [DreamState] No AgentSoul found for system agent — skipping Biome experience push");
            }
        }

        Ok(Some(format!(
            "Gen {} Biome simulated. Rarity: {}. Message: {}",
            gen, rarity_str, message
        )))
    }
}
