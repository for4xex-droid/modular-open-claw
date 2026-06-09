/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::SoulStore;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use soul::{self, AgentSoul};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::DatabasePool;
use crate::sql_exec;

/// A record for SoulVersion history tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct SoulVersion {
    pub hash: String,
    pub soul_id: String,
    pub parent_hash: Option<String>,
    pub somatic_markers: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A snapshot of soul state for quick caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulSnapshot {
    pub attachment_style: String,
    pub narrative_self: String,
    pub prompt_fragment: String,
    pub generation: u32,
    pub lora_adapter_path: Option<String>,
    pub lora_base_model: Option<String>,
    pub lora_hash: Option<String>,
}

/// A record representing an archived LoRA model from a past generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedLoraModel {
    pub id: i64,
    pub generation: u32,
    pub lora_hash: String,
    pub adapter_path: String,
    pub base_model: String,
}

/// Soul 永続化ストア (Universal: SQLite/PostgreSQL 対応)
pub struct UniversalSoulStore {
    pool: DatabasePool,
    cache: Arc<RwLock<Option<SoulSnapshot>>>,
}

impl UniversalSoulStore {
    /// 新しいインスタンスを生成する
    pub fn new(pool: DatabasePool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// 会話コンテキスト等からSoulスナップショットを直接注入する（初期化用）
    pub async fn inject_snapshot(&self, snapshot: SoulSnapshot) {
        let mut cache = self.cache.write().await;
        *cache = Some(snapshot);
    }

    /// キャッシュからスナップショットを取得する
    pub async fn get_snapshot(&self) -> Option<SoulSnapshot> {
        let cache = self.cache.read().await;
        cache.clone()
    }

    /// Soulの状態を永続化する
    pub async fn save_soul(&self, soul: &AgentSoul) -> Result<(), AiomeError> {
        let markers_json = serde_json::to_string(&soul.somatic_markers).unwrap_or_default();
        let defenses_json = serde_json::to_string(&soul.defenses).unwrap_or_default();
        let predictive_json = serde_json::to_string(&soul.predictive_model).unwrap_or_default();
        let attachment_json = serde_json::to_string(&soul.attachment).unwrap_or_default();
        let instinct_json = serde_json::to_string(&soul.instinct).unwrap_or_default();
        let anamnesis_json = serde_json::to_string(&soul.anamnesis).unwrap_or_default();
        let buffer_json = serde_json::to_string(&soul.experience_buffer).unwrap_or_default();
        let semantic_json = serde_json::to_string(&soul.semantic_index).unwrap_or_default();
        let persona_boundaries_json =
            serde_json::to_string(&soul.persona_boundaries).unwrap_or_default();
        let frozen_traits_json = serde_json::to_string(&soul.frozen_traits).unwrap_or_default();

        let q = format!(
            r#"
            INSERT INTO agent_souls (
                id, generation, soul_hash, somatic_markers_json,
                defenses_json, predictive_model_json, attachment_json,
                instinct_json, anamnesis_json, experience_buffer_json,
                lora_adapter_path, lora_base_model, lora_hash, last_begging_at,
                semantic_index_json, persona_boundaries_json, frozen_traits_json
            ) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}, {12}, {13}, {14}, {15}, {16})
            ON CONFLICT(id) DO UPDATE SET
                generation = excluded.generation,
                soul_hash = excluded.soul_hash,
                somatic_markers_json = excluded.somatic_markers_json,
                defenses_json = excluded.defenses_json,
                predictive_model_json = excluded.predictive_model_json,
                attachment_json = excluded.attachment_json,
                instinct_json = excluded.instinct_json,
                anamnesis_json = excluded.anamnesis_json,
                experience_buffer_json = excluded.experience_buffer_json,
                lora_adapter_path = excluded.lora_adapter_path,
                lora_base_model = excluded.lora_base_model,
                lora_hash = excluded.lora_hash,
                last_begging_at = excluded.last_begging_at,
                semantic_index_json = excluded.semantic_index_json,
                persona_boundaries_json = excluded.persona_boundaries_json,
                frozen_traits_json = excluded.frozen_traits_json
            "#,
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2),
            self.pool.ph(3),
            self.pool.ph(4),
            self.pool.ph(5),
            self.pool.ph(6),
            self.pool.ph(7),
            self.pool.ph(8),
            self.pool.ph(9),
            self.pool.ph(10),
            self.pool.ph(11),
            self.pool.ph(12),
            self.pool.ph(13),
            self.pool.ph(14),
            self.pool.ph(15),
            self.pool.ph(16)
        );

        sql_exec!(
            &self.pool,
            &q,
            &soul.id,
            soul.generation as i64,
            &soul.soul_hash,
            markers_json,
            defenses_json,
            predictive_json,
            attachment_json,
            instinct_json,
            anamnesis_json,
            buffer_json,
            soul.lora_adapter_path.clone(),
            soul.lora_base_model.clone(),
            soul.lora_hash.clone(),
            soul.last_begging_at,
            semantic_json,
            persona_boundaries_json,
            frozen_traits_json
        )?;

        // Update cache on save
        {
            let mut cache = self.cache.write().await;
            *cache = Some(SoulSnapshot {
                attachment_style: format!("{:?}", soul.attachment.style),
                narrative_self: soul.anamnesis.narrative_self.clone().unwrap_or_default(),
                prompt_fragment: soul.instinct.prompt_fragment.clone(),
                generation: soul.generation,
                lora_adapter_path: soul.lora_adapter_path.clone(),
                lora_base_model: soul.lora_base_model.clone(),
                lora_hash: soul.lora_hash.clone(),
            });
        }

        Ok(())
    }

    /// アーカイブされたLoRAモデルの履歴を取得する
    pub async fn get_archived_lora_models(
        &self,
        soul_id: &str,
    ) -> Result<Vec<ArchivedLoraModel>, AiomeError> {
        use sqlx::Row;
        let q = format!(
            "SELECT id, generation, lora_hash, adapter_path, base_model FROM archived_lora_models WHERE soul_id = {0} ORDER BY generation DESC",
            self.pool.ph(0)
        );

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = crate::sql_fetch_raw!(p, &q, soul_id)?;
                let mut results = Vec::new();
                for r in rows {
                    results.push(ArchivedLoraModel {
                        id: r.get::<i64, _>("id"),
                        generation: r.get::<i64, _>("generation") as u32,
                        lora_hash: r.get("lora_hash"),
                        adapter_path: r.get("adapter_path"),
                        base_model: r.get("base_model"),
                    });
                }
                Ok(results)
            }
            DatabasePool::Postgres(p) => {
                let rows = crate::sql_fetch_raw!(p, &q, soul_id)?;
                let mut results = Vec::new();
                for r in rows {
                    let id_val: i64 = try_extract_i64_from_pg_row(&r, "id");
                    let gen_val: i64 = try_extract_i64_from_pg_row(&r, "generation");
                    results.push(ArchivedLoraModel {
                        id: id_val,
                        generation: gen_val as u32,
                        lora_hash: r.get("lora_hash"),
                        adapter_path: r.get("adapter_path"),
                        base_model: r.get("base_model"),
                    });
                }
                Ok(results)
            }
        }
    }

    /// Soulの履歴を保存する (Event Sourcing Light)
    pub async fn record_version(
        &self,
        soul_id: &str,
        hash: &str,
        parent: Option<&str>,
        markers: &serde_json::Value,
    ) -> Result<(), AiomeError> {
        let markers_json = serde_json::to_string(markers).unwrap_or_default();
        let q = format!(
            "INSERT INTO soul_versions (hash, soul_id, parent_hash, somatic_markers_json) VALUES ({0}, {1}, {2}, {3})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );

        sql_exec!(&self.pool, &q, hash, soul_id, parent, markers_json)?;

        Ok(())
    }

    /// 指定されたSoul IDのバージョン履歴を取得する
    pub async fn list_versions(
        &self,
        soul_id: &str,
        limit: i64,
    ) -> Result<Vec<SoulVersion>, AiomeError> {
        use sqlx::Row;
        let q = format!(
            "SELECT hash, soul_id, parent_hash, somatic_markers_json, created_at FROM soul_versions WHERE soul_id = {0} ORDER BY created_at DESC LIMIT {1}",
            self.pool.ph(0), self.pool.ph(1)
        );

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = crate::sql_fetch_raw!(p, &q, soul_id, limit)?;
                let mut results = Vec::new();
                for r in rows {
                    let markers_json: String = r.get("somatic_markers_json");
                    results.push(SoulVersion {
                        hash: r.get("hash"),
                        soul_id: r.get("soul_id"),
                        parent_hash: r.get("parent_hash"),
                        somatic_markers: serde_json::from_str(&markers_json).unwrap_or_default(),
                        created_at: r.get("created_at"),
                    });
                }
                Ok(results)
            }
            DatabasePool::Postgres(p) => {
                let rows = crate::sql_fetch_raw!(p, &q, soul_id, limit)?;
                let mut results = Vec::new();
                for r in rows {
                    let markers_json: String = r.get("somatic_markers_json");
                    results.push(SoulVersion {
                        hash: r.get("hash"),
                        soul_id: r.get("soul_id"),
                        parent_hash: r.get("parent_hash"),
                        somatic_markers: serde_json::from_str(&markers_json).unwrap_or_default(),
                        created_at: r.get("created_at"),
                    });
                }
                Ok(results)
            }
        }
    }

    /// バージョンハッシュが存在するか確認する
    pub async fn version_exists(&self, hash: &str) -> Result<bool, AiomeError> {
        let q = format!(
            "SELECT EXISTS(SELECT 1 FROM soul_versions WHERE hash = {})",
            self.pool.ph(0)
        );
        let exists: bool = match &self.pool {
            DatabasePool::Sqlite(p) => crate::sql_scalar!(p, &q, hash)?,
            DatabasePool::Postgres(p) => crate::sql_scalar!(p, &q, hash)?,
        };
        Ok(exists)
    }

    // Helper to map Sqlite row to AgentSoul
    fn map_sqlite_row(
        &self,
        r: sqlx::sqlite::SqliteRow,
        id: &str,
    ) -> Result<AgentSoul, AiomeError> {
        use sqlx::Row;
        let markers_json: String = r.get("somatic_markers_json");
        let defenses_json: String = r.get("defenses_json");
        let predictive_json: String = r.get("predictive_model_json");
        let attachment_json: String = r.get("attachment_json");
        let instinct_json: String = r.get("instinct_json");
        let anamnesis_json: String = r.get("anamnesis_json");
        let buffer_json: String = r.get("experience_buffer_json");
        let semantic_json: Option<String> = r.try_get("semantic_index_json").ok();
        let persona_json: Option<String> = r.try_get("persona_boundaries_json").ok();
        let frozen_traits_json: Option<String> = r.try_get("frozen_traits_json").ok();

        Ok(AgentSoul {
            id: id.to_string(),
            generation: r.get::<i64, _>("generation") as u32,
            soul_hash: r.get("soul_hash"),
            somatic_markers: serde_json::from_str(&markers_json).unwrap_or_default(),
            defenses: serde_json::from_str(&defenses_json).unwrap_or_default(),
            predictive_model: serde_json::from_str(&predictive_json).unwrap_or_default(),
            attachment: serde_json::from_str(&attachment_json).unwrap_or_default(),
            instinct: serde_json::from_str(&instinct_json).unwrap_or_default(),
            anamnesis: serde_json::from_str(&anamnesis_json).unwrap_or_default(),
            experience_buffer: serde_json::from_str(&buffer_json).unwrap_or_default(),
            lora_adapter_path: r.get("lora_adapter_path"),
            lora_base_model: r.get("lora_base_model"),
            lora_hash: r.get("lora_hash"),
            last_begging_at: r.get("last_begging_at"),
            semantic_index: semantic_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            persona_boundaries: persona_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            frozen_traits: frozen_traits_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        })
    }

    // Helper to map Postgres row to AgentSoul
    fn map_postgres_row(
        &self,
        r: sqlx::postgres::PgRow,
        id: &str,
    ) -> Result<AgentSoul, AiomeError> {
        use sqlx::Row;
        let markers_json: String = r.get("somatic_markers_json");
        let defenses_json: String = r.get("defenses_json");
        let predictive_json: String = r.get("predictive_model_json");
        let attachment_json: String = r.get("attachment_json");
        let instinct_json: String = r.get("instinct_json");
        let anamnesis_json: String = r.get("anamnesis_json");
        let buffer_json: String = r.get("experience_buffer_json");
        let semantic_json: Option<String> = r.try_get("semantic_index_json").ok();
        let persona_json: Option<String> = r.try_get("persona_boundaries_json").ok();
        let frozen_traits_json: Option<String> = r.try_get("frozen_traits_json").ok();

        Ok(AgentSoul {
            id: id.to_string(),
            generation: r.get::<i64, _>("generation") as u32,
            soul_hash: r.get("soul_hash"),
            somatic_markers: serde_json::from_str(&markers_json).unwrap_or_default(),
            defenses: serde_json::from_str(&defenses_json).unwrap_or_default(),
            predictive_model: serde_json::from_str(&predictive_json).unwrap_or_default(),
            attachment: serde_json::from_str(&attachment_json).unwrap_or_default(),
            instinct: serde_json::from_str(&instinct_json).unwrap_or_default(),
            anamnesis: serde_json::from_str(&anamnesis_json).unwrap_or_default(),
            experience_buffer: serde_json::from_str(&buffer_json).unwrap_or_default(),
            lora_adapter_path: r.get("lora_adapter_path"),
            lora_base_model: r.get("lora_base_model"),
            lora_hash: r.get("lora_hash"),
            last_begging_at: r.get("last_begging_at"),
            semantic_index: semantic_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            persona_boundaries: persona_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            frozen_traits: frozen_traits_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        })
    }

    /// AgentSoulの状態を復元する
    pub async fn load_soul(&self, id: &str) -> Result<Option<AgentSoul>, AiomeError> {
        let q = format!(
            r#"
            SELECT
                generation, soul_hash, somatic_markers_json,
                defenses_json, predictive_model_json, attachment_json,
                instinct_json, anamnesis_json, experience_buffer_json,
                lora_adapter_path, lora_base_model, lora_hash, last_begging_at,
                semantic_index_json, persona_boundaries_json, frozen_traits_json
            FROM agent_souls
            WHERE id = {}
            "#,
            self.pool.ph(0)
        );

        let soul = match &self.pool {
            DatabasePool::Sqlite(p) => {
                if let Some(r) = crate::sql_fetch_raw_optional!(p, &q, id)? {
                    Some(self.map_sqlite_row(r, id)?)
                } else {
                    None
                }
            }
            DatabasePool::Postgres(p) => {
                if let Some(r) = crate::sql_fetch_raw_optional!(p, &q, id)? {
                    Some(self.map_postgres_row(r, id)?)
                } else {
                    None
                }
            }
        };

        if let Some(ref agent_soul) = soul {
            // RS-1: Update cache on explicit load
            let mut cache = self.cache.write().await;
            if cache.is_none() {
                *cache = Some(SoulSnapshot {
                    attachment_style: format!("{:?}", agent_soul.attachment.style),
                    narrative_self: agent_soul
                        .anamnesis
                        .narrative_self
                        .clone()
                        .unwrap_or_default(),
                    prompt_fragment: agent_soul.instinct.prompt_fragment.clone(),
                    generation: agent_soul.generation,
                    lora_adapter_path: agent_soul.lora_adapter_path.clone(),
                    lora_base_model: agent_soul.lora_base_model.clone(),
                    lora_hash: agent_soul.lora_hash.clone(),
                });
            }
        }

        Ok(soul)
    }
}

#[async_trait]
impl aiome_core_contracts::traits::SoulStore for UniversalSoulStore {
    async fn load_soul(&self, id: &str) -> Result<Option<serde_json::Value>, AiomeError> {
        if let Some(soul) = self.load_soul(id).await? {
            let val = serde_json::to_value(soul).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to serialize Soul to JSON: {}", e),
            })?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    async fn store_soul_fragment(
        &self,
        fragment_yaml: &str,
        version_hash: &str,
    ) -> Result<(), AiomeError> {
        let q = format!(
            "INSERT INTO soul_fragments (fragment_yaml, version_hash) VALUES ({}, {})",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        sql_exec!(&self.pool, &q, fragment_yaml, version_hash)?;
        Ok(())
    }

    async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
        let q = "SELECT fragment_yaml, version_hash FROM soul_fragments ORDER BY created_at DESC LIMIT 1";
        use sqlx::Row;
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let row = crate::sql_fetch_raw_optional!(p, q)?;
                Ok(row.map(|r| (r.get(0), r.get(1))))
            }
            DatabasePool::Postgres(p) => {
                let row = crate::sql_fetch_raw_optional!(p, q)?;
                Ok(row.map(|r| (r.get(0), r.get(1))))
            }
        }
    }

    async fn archive_lora_model(
        &self,
        soul_id: &str,
        generation: u32,
        lora_hash: &str,
        adapter_path: &str,
        base_model: &str,
    ) -> Result<(), AiomeError> {
        let q = format!(
            "INSERT INTO archived_lora_models (soul_id, generation, lora_hash, adapter_path, base_model) VALUES ({0}, {1}, {2}, {3}, {4})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4)
        );

        sql_exec!(
            &self.pool,
            &q,
            soul_id,
            generation as i64,
            lora_hash,
            adapter_path,
            base_model
        )?;

        Ok(())
    }
}

fn try_extract_i64_from_pg_row(row: &sqlx::postgres::PgRow, col: &str) -> i64 {
    use sqlx::Row;
    if let Ok(val) = row.try_get::<i32, _>(col) {
        return val as i64;
    }
    if let Ok(val) = row.try_get::<i64, _>(col) {
        return val;
    }
    0
}
