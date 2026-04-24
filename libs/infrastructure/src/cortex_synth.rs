/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::belief_consistency_gate::BeliefConsistencyGate;
use crate::db::DatabasePool;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

pub struct CortexSynthesizer {
    llm_provider: Arc<dyn LlmProvider>,
    pool: DatabasePool,
    belief_gate: Option<Arc<BeliefConsistencyGate>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthDataset {
    pub pairs: Vec<SynthPair>,
    pub source_stats: SynthStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthPair {
    pub instruction: String,
    pub response: String,
    #[serde(default)]
    pub source_article_id: String,
    pub quality_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthStats {
    pub total_articles: u32,
    pub total_pairs: u32,
    pub avg_quality: f64,
}

impl CortexSynthesizer {
    pub fn new(
        llm_provider: Arc<dyn LlmProvider>,
        pool: DatabasePool,
        belief_gate: Option<Arc<BeliefConsistencyGate>>,
    ) -> Self {
        Self {
            llm_provider,
            pool,
            belief_gate,
        }
    }

    pub async fn generate_dataset(&self) -> Result<SynthDataset, AiomeError> {
        let pool = self.pool.get_sqlite_pool_or_err()?;

        let mut pairs = Vec::new();
        let mut total_articles = 0;
        let mut last_id = "".to_string();

        loop {
            let articles = sqlx::query(
                "SELECT id, title, content_md FROM cortex_wiki_articles WHERE id > ? ORDER BY id ASC LIMIT 50"
            )
            .bind(&last_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if articles.is_empty() {
                break;
            }

            for row in articles {
                use sqlx::Row;
                let id = row.try_get::<String, _>("id").unwrap_or_default();
                last_id = id.clone(); // Update cursor to the current last id
                let title = row.try_get::<String, _>("title").unwrap_or_default();
                let content = row.try_get::<String, _>("content_md").unwrap_or_default();
                let content: String = content.chars().take(8000).collect();
                let sanitized = shared::guardrails::sanitize_for_prompt(&content);
                let scrubbed = shared::guardrails::mask_pii(&sanitized);

                let prompt = format!(
                    "Generate instruction-response pairs based on this article. Return ONLY a JSON array of objects with 'instruction', 'response', and 'quality_score' (0.0 to 1.0).\n<ARTICLE title=\"{}\">\n{}\n</ARTICLE>",
                    title, scrubbed
                );

                let res_timeout = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    self.llm_provider.complete(
                        &prompt,
                        Some("You are a synthetic data generator. Output pure JSON array."),
                    ),
                )
                .await;

                match res_timeout {
                    Ok(Ok(res)) => {
                        let json_str = crate::llm::utils::extract_json(&res.content)
                            .unwrap_or_else(|_| "[]".to_string());

                        let extracted_pairs: Vec<SynthPair> = match serde_json::from_str(&json_str)
                        {
                            Ok(pairs) => pairs,
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to parse LLM JSON for article {}: error={}, snippet={}",
                                    id,
                                    e,
                                    &json_str.chars().take(200).collect::<String>()
                                );
                                vec![]
                            }
                        };
                        for mut p in extracted_pairs {
                            p.source_article_id = id.clone();
                            if p.quality_score >= 0.6 {
                                let mut is_acceptable = true;
                                if let Some(gate) = &self.belief_gate {
                                    match gate.check_belief_consistency(&p.response).await {
                                        Ok(crate::belief_consistency_gate::BeliefCheckResult::Contradicted { flag }) => {
                                            tracing::warn!("Pair rejected by BeliefGate: {}", flag);
                                            is_acceptable = false;
                                        }
                                        Ok(crate::belief_consistency_gate::BeliefCheckResult::RevisionCandidate { evidence }) => {
                                            tracing::info!("Pair marked as RevisionCandidate with {} evidences", evidence.len());
                                            is_acceptable = false; // Exclude from baseline dataset to maintain purity
                                        }
                                        Err(e) => {
                                            tracing::warn!("BeliefGate error: {}", e);
                                            // Fallback: allow upon gate error to avoid total pipeline stall
                                        }
                                        _ => {} // Consistent
                                    }
                                }
                                if is_acceptable {
                                    pairs.push(p);
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("LLM error while processing article {}: {}", id, e)
                    }
                    Err(_) => tracing::warn!("LLM timeout while processing article {}", id),
                }

                total_articles += 1;
            }
        }

        let total_pairs = pairs.len() as u32;
        let avg_quality = if total_pairs > 0 {
            pairs.iter().map(|p| p.quality_score).sum::<f64>() / (total_pairs as f64)
        } else {
            0.0
        };

        Ok(SynthDataset {
            pairs,
            source_stats: SynthStats {
                total_articles,
                total_pairs,
                avg_quality,
            },
        })
    }

    pub fn export_to_jsonl(&self, dataset: &SynthDataset, path: &Path) -> Result<(), AiomeError> {
        let file = std::fs::File::create(path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create export file: {}", e),
        })?;
        let mut writer = std::io::BufWriter::new(file);

        use std::io::Write;
        for pair in &dataset.pairs {
            let line = serde_json::json!({
                "conversations": [
                    { "from": "human", "value": pair.instruction },
                    { "from": "gpt", "value": pair.response }
                ]
            });
            let line_str =
                serde_json::to_string(&line).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to serialize ShareGPT pair to JSON: {}", e),
                })?;
            writeln!(writer, "{}", line_str).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write export file: {}", e),
            })?;
        }
        writer.flush().map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to flush export file: {}", e),
        })?;
        Ok(())
    }
}
