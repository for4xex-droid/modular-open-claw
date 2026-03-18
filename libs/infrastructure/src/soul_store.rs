use std::sync::Arc;
use sqlx::SqlitePool;
use soul::model::AgentSoul;
use aiome_core::error::AiomeError;
use tracing::{info, warn};

pub struct SqliteSoulStore {
    pool: Arc<SqlitePool>,
}

impl SqliteSoulStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn save_soul(&self, soul: &AgentSoul) -> Result<(), AiomeError> {
        let somatic_json = serde_json::to_string(&soul.somatic_markers)
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        let defenses_json = serde_json::to_string(&soul.defenses)
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        let predictive_json = serde_json::to_string(&soul.predictive_model)
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        let attachment_json = serde_json::to_string(&soul.attachment)
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        let instinct_json = serde_json::to_string(&soul.instinct)
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        let buffer_json = serde_json::to_string(&soul.experience_buffer)
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            r#"
            INSERT INTO agent_souls (
                id, generation, soul_hash, somatic_markers_json,
                defenses_json, predictive_model_json, attachment_json,
                instinct_json, experience_buffer_json, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                generation=excluded.generation,
                soul_hash=excluded.soul_hash,
                somatic_markers_json=excluded.somatic_markers_json,
                defenses_json=excluded.defenses_json,
                predictive_model_json=excluded.predictive_model_json,
                attachment_json=excluded.attachment_json,
                instinct_json=excluded.instinct_json,
                experience_buffer_json=excluded.experience_buffer_json,
                updated_at=datetime('now')
            "#
        )
        .bind(&soul.id)
        .bind(soul.generation)
        .bind(&soul.soul_hash)
        .bind(somatic_json)
        .bind(defenses_json)
        .bind(predictive_json)
        .bind(attachment_json)
        .bind(instinct_json)
        .bind(buffer_json)
        .execute(&*self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        info!("🛡️ [SoulStore] AgentSoul {} (gen {}) persisted.", soul.id, soul.generation);
        Ok(())
    }

    pub async fn load_soul(&self, id: &str) -> Result<Option<AgentSoul>, AiomeError> {
        use sqlx::Row;
        let record = sqlx::query(
            r#"
            SELECT
                generation, soul_hash, somatic_markers_json,
                defenses_json, predictive_model_json, attachment_json,
                instinct_json, experience_buffer_json
            FROM agent_souls
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        if let Some(r) = record {
            let parsed_somatic = serde_json::from_str(&r.get::<String, _>("somatic_markers_json"))
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            let parsed_defenses = serde_json::from_str(&r.get::<String, _>("defenses_json"))
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            let parsed_predictive = serde_json::from_str(&r.get::<String, _>("predictive_model_json"))
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            let parsed_attachment = serde_json::from_str(&r.get::<String, _>("attachment_json"))
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            let parsed_instinct = serde_json::from_str(&r.get::<String, _>("instinct_json"))
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            let parsed_buffer = serde_json::from_str(&r.get::<String, _>("experience_buffer_json"))
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

            Ok(Some(AgentSoul {
                id: id.to_string(),
                generation: r.get::<i64, _>("generation") as u32,
                soul_hash: r.get("soul_hash"),
                somatic_markers: parsed_somatic,
                defenses: parsed_defenses,
                predictive_model: parsed_predictive,
                attachment: parsed_attachment,
                instinct: parsed_instinct,
                experience_buffer: parsed_buffer,
            }))
        } else {
            Ok(None)
        }
    }
}
