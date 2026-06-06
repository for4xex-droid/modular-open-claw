/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

//! CSAM 4層防壁モジュール。
//!
//! コンテンツ安全性を担保するために以下の層でフィルタリングを行う:
//! - Layer 1: `EkycVerifier` — eKYC 本人確認 (SQLiteEkycStore による DB ステータス検証)
//! - Layer 2: `PhashScanner` — PhotoDNA等によるハッシュ照合 (Phase 2 モック)
//! - Layer 3: `BoneChecker` — VRM 頭身比率チェック (Phase 2 実動)
//! - Layer 4: `KarmaToxicityScanner` — テキスト毒性チェック (Phase 2 実動)

pub mod bone_check;
pub mod ncmec;

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use ncmec::NcmecReporter;
use std::sync::Arc;
use uuid::Uuid;

/// CSAM スキャン結果。
#[derive(Debug, Clone)]
pub enum ScanVerdict {
    /// 安全と判定された。
    Safe,
    Rejected {
        reason: String,
        layer: &'static str,
        requires_ncmec_report: bool,
    },
}

/// CSAM スキャナー・トレイト。
/// 各層のチェッカーが実装する共通インターフェース。
#[async_trait]
pub trait ContentSafetyChecker: Send + Sync {
    /// コンテンツのメタデータに基づいてスキャンを実行する。
    async fn scan(
        &self,
        item_id: &Uuid,
        metadata: &serde_json::Value,
    ) -> Result<ScanVerdict, NurtureError>;
    /// チェッカーの名称 (ログ用)。
    fn name(&self) -> &'static str;
}

/// 多層を束ねるパイプラインスキャナー。
/// 全層を順に通過させ、一つでも Rejected なら即座に停止する。
pub struct CsamPipeline {
    checkers: Vec<Box<dyn ContentSafetyChecker>>,
    ncmec_reporter: Option<Arc<dyn NcmecReporter>>,
}

impl CsamPipeline {
    pub fn new(checkers: Vec<Box<dyn ContentSafetyChecker>>) -> Self {
        Self {
            checkers,
            ncmec_reporter: None,
        }
    }

    pub fn with_reporter(mut self, reporter: Arc<dyn NcmecReporter>) -> Self {
        self.ncmec_reporter = Some(reporter);
        self
    }

    /// 全チェッカーを順に通過させる。
    pub async fn run_all(
        &self,
        item_id: &Uuid,
        metadata: &serde_json::Value,
    ) -> Result<ScanVerdict, NurtureError> {
        for checker in &self.checkers {
            let verdict = checker.scan(item_id, metadata).await?;
            if let ScanVerdict::Rejected {
                reason,
                requires_ncmec_report,
                ..
            } = &verdict
            {
                tracing::warn!(
                    "🚨 CSAM Pipeline: {} により Reject されました (item: {})",
                    checker.name(),
                    item_id
                );
                if *requires_ncmec_report {
                    if let Some(reporter) = &self.ncmec_reporter {
                        if let Err(e) = reporter.report_csam(item_id, reason, metadata).await {
                            tracing::error!("🚨 [CRITICAL] Failed to report to NCMEC Queue: {}", e);
                        }
                    } else {
                        tracing::error!("🚨 [CRITICAL] NCMEC Reporter is not configured, but a reportable CSAM incident occurred! Item: {}", item_id);
                    }
                }
                return Ok(verdict);
            }
            tracing::debug!(
                "✅ CSAM Layer passed: {} (item: {})",
                checker.name(),
                item_id
            );
        }
        Ok(ScanVerdict::Safe)
    }
}

// === Layer 1: eKYC 本人確認 (DB ステータス検証) ===

/// Stripe Identity eKYC 本人確認（Phase 2 ではモックから SQLite 実装へ）。
/// 本番では Stripe API または DB を用いて出品者の本人確認状態を検証する。
pub struct EkycVerifier {
    pub store: std::sync::Arc<dyn crate::identity::ekyc::EkycStore>,
}

#[async_trait]
impl ContentSafetyChecker for EkycVerifier {
    async fn scan(
        &self,
        _item_id: &Uuid,
        metadata: &serde_json::Value,
    ) -> Result<ScanVerdict, NurtureError> {
        let actor_id_str = metadata.get("actor_id").and_then(|v| v.as_str());
        if let Some(s) = actor_id_str {
            match s.parse::<Uuid>() {
                Ok(actor_id) => {
                    let id = commerce_protocol::identity::ActorId(actor_id);
                    match self.store.is_verified(&id).await {
                        Ok(true) => return Ok(ScanVerdict::Safe),
                        Ok(false) => {
                            return Ok(ScanVerdict::Rejected {
                                layer: self.name(),
                                reason: "User has not completed KYC verification (AML Policy)"
                                    .to_string(),
                                requires_ncmec_report: false,
                            })
                        }
                        Err(e) => {
                            tracing::error!("KYC Store error: {}", e);
                            return Ok(ScanVerdict::Rejected {
                                layer: self.name(),
                                reason: "KYC system unavailable".to_string(),
                                requires_ncmec_report: false,
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("actor_id '{}' is not a valid UUID: {}", s, e);
                    return Ok(ScanVerdict::Rejected {
                        layer: self.name(),
                        reason: format!("Invalid actor_id format: {}", s),
                        requires_ncmec_report: false,
                    });
                }
            }
        }

        // Phase 2: KYC is strictly enforced. If actor_id is missing, reject.
        Ok(ScanVerdict::Rejected {
            layer: self.name(),
            reason: "Missing actor_id for KYC verification".to_string(),
            requires_ncmec_report: false,
        })
    }

    fn name(&self) -> &'static str {
        "eKYC (AML/Identity)"
    }
}

// === Layer 2: PhotoDNA モックスタブ ===

pub struct PhashScanner {
    pub pool: nurture_bridge::db::DatabasePool,
}
#[async_trait]
impl ContentSafetyChecker for PhashScanner {
    async fn scan(
        &self,
        _item_id: &Uuid,
        metadata: &serde_json::Value,
    ) -> Result<ScanVerdict, NurtureError> {
        let mut hash = metadata
            .get("p_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if hash.is_empty() {
            if let Some(b64) = metadata.get("image_base64").and_then(|v| v.as_str()) {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                if let Ok(bytes) = STANDARD.decode(b64) {
                    let hasher = nurture_bridge::csam::image_hash::ImageHasher::new();
                    if let Ok(computed) = hasher.compute_hash(&bytes) {
                        hash = computed;
                    }
                }
            }
        }

        let is_blacklisted = if !hash.is_empty() {
            nurture_bridge::csam::image_hash::ImageHasher::check_blacklist(&self.pool, &hash)
                .await
                .map_err(|e| {
                    tracing::error!("🚨 [PhashScanner] CSAM Blacklist DB check failed: {}", e);
                    NurtureError::Infrastructure(format!(
                        "CSAM blacklist verification unavailable: {}",
                        e
                    ))
                })?
        } else {
            false
        };

        if is_blacklisted {
            return Ok(ScanVerdict::Rejected {
                layer: self.name(),
                reason: "Malicious pHash detected (CSAM Blacklist)".to_string(),
                requires_ncmec_report: true,
            });
        }

        Ok(ScanVerdict::Safe)
    }

    fn name(&self) -> &'static str {
        "PhotoDNA (pHash)"
    }
}

// === Layer 4: Karma Toxicity Scanner (Phase 2) ===

/// KarmaPackage のテキスト内容の毒性・安全性チェック。
pub struct KarmaToxicityScanner {
    pub pool: nurture_bridge::db::DatabasePool,
}

#[async_trait]
impl ContentSafetyChecker for KarmaToxicityScanner {
    async fn scan(
        &self,
        _item_id: &Uuid,
        metadata: &serde_json::Value,
    ) -> Result<ScanVerdict, NurtureError> {
        // KarmaPackage の場合のみチェック
        if let Some(content) = metadata.get("karma_content").and_then(|v| v.as_str()) {
            // 設計判断: PhashScanner (CSAM 直接検知 = 法的義務) は DB 障害時に Fail-Closed (Err を返す)。
            // KarmaToxicityScanner (テキスト毒性 = 法的義務なし) は DB 障害時に Fail-Safe (デフォルトにフォールバック) で
            // サービス可用性を優先する。この非対称性は意図的。
            const DEFAULT_FORBIDDEN: &[&str] = &["dangerous", "illegal", "exploit"];
            let defaults =
                || -> Vec<String> { DEFAULT_FORBIDDEN.iter().map(|s| s.to_string()).collect() };

            let q = format!(
                "SELECT value FROM system_state WHERE key = {}",
                self.pool.ph(0)
            );

            let db_result: Result<Option<String>, _> = match &self.pool {
                nurture_bridge::db::DatabasePool::Sqlite(p) => {
                    sqlx::query_scalar(&q)
                        .bind("csam_toxicity_forbidden_words")
                        .fetch_optional(p)
                        .await
                }
                nurture_bridge::db::DatabasePool::Postgres(p) => {
                    sqlx::query_scalar(&q)
                        .bind("csam_toxicity_forbidden_words")
                        .fetch_optional(p)
                        .await
                }
            };

            let forbidden: Vec<String> = match db_result {
                Ok(Some(json_str)) => {
                    serde_json::from_str::<Vec<String>>(&json_str).unwrap_or_else(|e| {
                        tracing::warn!("⚠️ [KarmaToxicity] Failed to parse forbidden words JSON, falling back to defaults: {}", e);
                        defaults()
                    })
                }
                Ok(None) => defaults(),
                Err(e) => {
                    tracing::warn!("⚠️ [KarmaToxicity] system_state DB query failed, falling back to defaults: {}", e);
                    defaults()
                }
            };

            let lower_content = content.to_lowercase();
            for word in forbidden {
                if lower_content.contains(&word) {
                    return Ok(ScanVerdict::Rejected {
                        reason: format!("Forbidden word detected: {}", word),
                        layer: "KarmaToxicity",
                        requires_ncmec_report: false,
                    });
                }
            }
        }
        Ok(ScanVerdict::Safe)
    }

    fn name(&self) -> &'static str {
        "Karma Toxicity Scanner"
    }
}

/// デフォルト構成の CSAM パイプラインを構築する。
pub fn default_pipeline(
    ekyc_store: std::sync::Arc<dyn crate::identity::ekyc::EkycStore>,
    db_pool: nurture_bridge::db::DatabasePool,
) -> CsamPipeline {
    CsamPipeline::new(vec![
        Box::new(EkycVerifier { store: ekyc_store }),
        Box::new(PhashScanner {
            pool: db_pool.clone(),
        }),
        Box::new(bone_check::BoneChecker::default()),
        Box::new(KarmaToxicityScanner { pool: db_pool }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_safe_item() {
        let store = std::sync::Arc::new(crate::identity::ekyc::MockEkycStore {
            always_verified: true,
        });
        let db_pool = nurture_bridge::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        let pipeline = default_pipeline(store, db_pool);
        let item_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "kind": "VrmAvatar",
            "content": "test_base64_data",
            "head_to_body_ratio": 0.14,
            "file_size_bytes": 1024 * 1024,
            "actor_id": Uuid::new_v4().to_string()
        });
        let result = pipeline.run_all(&item_id, &metadata).await.unwrap();
        assert!(matches!(result, ScanVerdict::Safe));
    }

    #[tokio::test]
    async fn test_pipeline_rejects_child_proportion() {
        let store = std::sync::Arc::new(crate::identity::ekyc::MockEkycStore {
            always_verified: true,
        });
        let db_pool = nurture_bridge::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        let pipeline = default_pipeline(store, db_pool);
        let item_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "kind": "VrmAvatar",
            "content": "test_base64_data",
            "head_to_body_ratio": 0.30,
            "file_size_bytes": 512 * 1024,
            "actor_id": Uuid::new_v4().to_string()
        });
        let result = pipeline.run_all(&item_id, &metadata).await.unwrap();
        assert!(matches!(result, ScanVerdict::Rejected { .. }));
    }

    #[tokio::test]
    async fn test_phash_scanner_rejects_blacklist() {
        let db_pool = nurture_bridge::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        nurture_bridge::sql_exec!(
            &db_pool,
            "CREATE TABLE csam_blacklist (image_hash TEXT PRIMARY KEY)"
        )
        .unwrap();
        nurture_bridge::sql_exec!(
            &db_pool,
            "INSERT INTO csam_blacklist (image_hash) VALUES ('dummy_malicious_hash_value_12345')"
        )
        .unwrap();

        let scanner = PhashScanner { pool: db_pool };

        let item_id = Uuid::new_v4();

        // This relies on the blacklist logic in ImageHasher which blacklists "dummy_malicious_hash_value_12345"
        let metadata = serde_json::json!({
            "p_hash": "dummy_malicious_hash_value_12345"
        });

        let verdict = scanner.scan(&item_id, &metadata).await.unwrap();
        assert!(matches!(
            verdict,
            ScanVerdict::Rejected {
                requires_ncmec_report: true,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_phash_scanner_fail_closed_on_db_error() {
        // Create an empty db pool WITHOUT the csam_blacklist table to force a DB error
        let db_pool = nurture_bridge::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();

        let scanner = PhashScanner { pool: db_pool };

        let item_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "p_hash": "dummy_malicious_hash_value_12345"
        });

        let result = scanner.scan(&item_id, &metadata).await;

        // Ensure it fails-closed (returns an error), rather than returning Ok(Safe)
        assert!(
            result.is_err(),
            "Expected PhashScanner to return an error when DB fails, but got {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(matches!(err, NurtureError::Infrastructure(_)));
    }

    #[tokio::test]
    async fn test_karma_toxicity_scanner_fallback() {
        // Empty DB without system_state table
        let db_pool = nurture_bridge::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        let scanner = KarmaToxicityScanner { pool: db_pool };

        let item_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "karma_content": "This is an illegal exploit!"
        });

        let result = scanner.scan(&item_id, &metadata).await.unwrap();
        assert!(matches!(result, ScanVerdict::Rejected { .. }));

        let metadata_safe = serde_json::json!({
            "karma_content": "This is a safe text."
        });
        let result_safe = scanner.scan(&item_id, &metadata_safe).await.unwrap();
        assert!(matches!(result_safe, ScanVerdict::Safe));
    }

    #[tokio::test]
    async fn test_karma_toxicity_scanner_db_words() {
        let db_pool = nurture_bridge::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        nurture_bridge::sql_exec!(
            &db_pool,
            "CREATE TABLE system_state (key TEXT PRIMARY KEY, value TEXT, updated_at DATETIME)"
        )
        .unwrap();
        nurture_bridge::sql_exec!(&db_pool, "INSERT INTO system_state (key, value, updated_at) VALUES ('csam_toxicity_forbidden_words', '[\"custom_bad_word\", \"another_bad\"]', datetime('now'))").unwrap();

        let scanner = KarmaToxicityScanner { pool: db_pool };

        let item_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "karma_content": "This contains a custom_bad_word inside."
        });

        let result = scanner.scan(&item_id, &metadata).await.unwrap();
        assert!(matches!(result, ScanVerdict::Rejected { .. }));

        // DB に設定が存在する場合、デフォルトの禁止ワードを完全に上書きする設計。
        // "illegal" はデフォルトだが、DB の明示的な設定が優先される。
        let metadata_old_bad = serde_json::json!({
            "karma_content": "This is illegal but not in custom DB."
        });
        let result_old = scanner.scan(&item_id, &metadata_old_bad).await.unwrap();
        // If it's an override, it should be Safe.
        assert!(matches!(result_old, ScanVerdict::Safe));
    }
}
