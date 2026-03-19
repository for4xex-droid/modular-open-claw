use aiome_core::error::AiomeError;
use soul::model::AgentSoul;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{info, warn};

use tokio::sync::RwLock;

#[derive(Debug, Clone)]
/// AgentSoulの読み取り専用スナップショット
pub struct SoulSnapshot {
    /// attachment_style
    pub attachment_style: soul::attachment::AttachmentStyle,
    /// narrative_self
    pub narrative_self: Option<String>,
    /// prompt_fragment
    pub prompt_fragment: String,
    /// generation
    pub generation: u32,
}

/// AgentSoulのSQLite永続化ストア
pub struct SqliteSoulStore {
    pool: Arc<SqlitePool>,
    cache: Arc<RwLock<Option<SoulSnapshot>>>,
}

impl SqliteSoulStore {
    /// 新しいインスタンスを生成する
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// AgentSoulの状態をSQLiteに永続化する
    pub async fn save_soul(&self, soul: &AgentSoul) -> Result<(), AiomeError> {
        if soul.id.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Soul ID cannot be empty".into(),
            });
        }

        // I-2: Truncate experience buffer if too large (e.g., retain only the latest 10,000)
        let buffer_limit = 10_000;
        let mut buffer_to_save = soul.experience_buffer.clone();
        if buffer_to_save.len() > buffer_limit {
            let skip_count = buffer_to_save.len() - buffer_limit;
            buffer_to_save = buffer_to_save.into_iter().skip(skip_count).collect();
            warn!(
                "⚠️ [SoulStore] Truncated experience_buffer for {} from {} to {}",
                soul.id,
                soul.experience_buffer.len(),
                buffer_limit
            );
        }

        let somatic_json = serde_json::to_string(&soul.somatic_markers).map_err(|e| {
            AiomeError::Infrastructure {
                reason: e.to_string(),
            }
        })?;
        let defenses_json =
            serde_json::to_string(&soul.defenses).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let predictive_json = serde_json::to_string(&soul.predictive_model).map_err(|e| {
            AiomeError::Infrastructure {
                reason: e.to_string(),
            }
        })?;
        let attachment_json =
            serde_json::to_string(&soul.attachment).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let instinct_json =
            serde_json::to_string(&soul.instinct).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let anamnesis_json =
            serde_json::to_string(&soul.anamnesis).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let buffer_json =
            serde_json::to_string(&buffer_to_save).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        sqlx::query(
            r#"
            INSERT INTO agent_souls (
                id, generation, soul_hash, somatic_markers_json,
                defenses_json, predictive_model_json, attachment_json,
                instinct_json, anamnesis_json, experience_buffer_json,
                lora_adapter_path, lora_base_model, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                generation=excluded.generation,
                soul_hash=excluded.soul_hash,
                somatic_markers_json=excluded.somatic_markers_json,
                defenses_json=excluded.defenses_json,
                predictive_model_json=excluded.predictive_model_json,
                attachment_json=excluded.attachment_json,
                instinct_json=excluded.instinct_json,
                anamnesis_json=excluded.anamnesis_json,
                experience_buffer_json=excluded.experience_buffer_json,
                lora_adapter_path=excluded.lora_adapter_path,
                lora_base_model=excluded.lora_base_model,
                updated_at=datetime('now')
            "#,
        )
        .bind(&soul.id)
        .bind(soul.generation)
        .bind(&soul.soul_hash)
        .bind(somatic_json)
        .bind(defenses_json)
        .bind(predictive_json)
        .bind(attachment_json)
        .bind(instinct_json)
        .bind(anamnesis_json)
        .bind(buffer_json)
        .bind(&soul.lora_adapter_path)
        .bind(&soul.lora_base_model)
        .execute(&*self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        // Update cache (RS-1)
        {
            let mut cache = self.cache.write().await;
            *cache = Some(SoulSnapshot {
                attachment_style: soul.attachment.style.clone(),
                narrative_self: soul.anamnesis.narrative_self.clone(),
                prompt_fragment: soul.instinct.prompt_fragment.clone(),
                generation: soul.generation,
            });
        }

        info!(
            "🛡️ [SoulStore] AgentSoul {} (gen {}) persisted and cached.",
            soul.id, soul.generation
        );
        Ok(())
    }

    /// AgentSoulの読み取り専用スナップショットを取得する
    pub async fn get_snapshot(&self) -> Option<SoulSnapshot> {
        self.cache.read().await.clone()
    }

    /// AgentSoulをインメモリキャッシュに読み込む
    pub async fn load_into_cache(&self, id: &str) -> Result<bool, AiomeError> {
        if let Some(soul) = self.load_soul(id).await? {
            let mut cache = self.cache.write().await;
            *cache = Some(SoulSnapshot {
                attachment_style: soul.attachment.style.clone(),
                narrative_self: soul.anamnesis.narrative_self.clone(),
                prompt_fragment: soul.instinct.prompt_fragment.clone(),
                generation: soul.generation,
            });
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// SQLiteからAgentSoulの状態を復元する
    pub async fn load_soul(&self, id: &str) -> Result<Option<AgentSoul>, AiomeError> {
        use sqlx::Row;
        let record = sqlx::query(
            r#"
            SELECT
                generation, soul_hash, somatic_markers_json,
                defenses_json, predictive_model_json, attachment_json,
                instinct_json, anamnesis_json, experience_buffer_json,
                lora_adapter_path, lora_base_model
            FROM agent_souls
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        if let Some(r) = record {
            let parsed_somatic = serde_json::from_str(&r.get::<String, _>("somatic_markers_json"))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
            let parsed_defenses = serde_json::from_str(&r.get::<String, _>("defenses_json"))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
            let parsed_predictive = serde_json::from_str(
                &r.get::<String, _>("predictive_model_json"),
            )
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
            let parsed_attachment: soul::attachment::AttachmentModel =
                serde_json::from_str(&r.get::<String, _>("attachment_json")).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
            let parsed_instinct: soul::instinct::Instinct =
                serde_json::from_str(&r.get::<String, _>("instinct_json")).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
            let parsed_anamnesis = serde_json::from_str(&r.get::<String, _>("anamnesis_json"))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
            let parsed_buffer = serde_json::from_str(&r.get::<String, _>("experience_buffer_json"))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

            let agent_soul = AgentSoul {
                id: id.to_string(),
                generation: r.get::<i64, _>("generation") as u32,
                soul_hash: r.get("soul_hash"),
                somatic_markers: parsed_somatic,
                defenses: parsed_defenses,
                predictive_model: parsed_predictive,
                attachment: parsed_attachment,
                instinct: parsed_instinct,
                anamnesis: parsed_anamnesis,
                experience_buffer: parsed_buffer,
                lora_adapter_path: r.get("lora_adapter_path"),
                lora_base_model: r.get("lora_base_model"),
            };

            // RS-1: Update cache on explicit load
            {
                let mut cache = self.cache.write().await;
                if cache.is_none() {
                    *cache = Some(SoulSnapshot {
                        attachment_style: agent_soul.attachment.style.clone(),
                        narrative_self: agent_soul.anamnesis.narrative_self.clone(),
                        prompt_fragment: agent_soul.instinct.prompt_fragment.clone(),
                        generation: agent_soul.generation,
                    });
                }
            }

            Ok(Some(agent_soul))
        } else {
            Ok(None)
        }
    }
}
