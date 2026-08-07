/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use aiome_core_contracts::traits::{Job, JobQueue, JobStatus, Publisher, SettingsOps};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// `mock_x` モジュール
#[cfg(any(test, debug_assertions))]
pub mod mock_x;

pub mod wordpress;

/// [B-2] Publish Pipeline Orchestrator
/// 各種パブリッシャーを管理し、ジョブステータスに基づいて配信を実行する。
pub struct PublishPipeline {
    publishers: Vec<Box<dyn Publisher>>,
    /// 外部送信ゲート。Some の場合、`feature_flag.seo_publish` が true でなければ投稿しない。
    /// None はテスト用（ゲートなし＝従来挙動）。本番組み立て（core_services）は必ず Some を渡すこと。
    settings: Option<Arc<dyn SettingsOps>>,
}

impl PublishPipeline {
    /// 新しいインスタンスを生成する
    pub fn new(publishers: Vec<Box<dyn Publisher>>) -> Self {
        Self {
            publishers,
            settings: None,
        }
    }

    /// 外部送信ゲート（SettingsOps）を接続する
    pub fn with_settings(mut self, settings: Arc<dyn SettingsOps>) -> Self {
        self.settings = Some(settings);
        self
    }

    /// `run_job` を実行する
    pub async fn run_job(
        &self,
        platform: &str,
        content: &str,
        media_paths: &[PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        // Fail-Closed 外部送信ゲート: ユーザーが Settings で明示的に有効化しない限り投稿しない
        if let Some(settings) = &self.settings {
            if !settings
                .is_feature_enabled(shared::feature_flags::SEO_PUBLISH_FLAG)
                .await
            {
                info!(
                    "⏭️ [PublishPipeline] Publish to '{}' skipped: feature_flag.seo_publish is disabled (enable it in Settings → Feature Flags).",
                    platform
                );
                return Err(AiomeError::Infrastructure {
                    reason: "publish skipped: feature_flag.seo_publish is disabled".to_string(),
                });
            }
        }

        let publisher = self
            .publishers
            .iter()
            .find(|p| p.platform_name() == platform)
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: format!("Publisher not found for platform: {}", platform),
            })?;

        info!("📤 [PublishPipeline] Publishing to {}...", platform);
        publisher.publish(content, media_paths, metadata).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::traits::SettingsOps;

    /// feature_flag.seo_publish に固定値を返すテスト用 SettingsOps
    #[derive(Debug)]
    struct FixedFlagSettings {
        seo_publish: Option<&'static str>,
    }

    #[async_trait]
    impl SettingsOps for FixedFlagSettings {
        async fn do_get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
            if key == "feature_flag.seo_publish" {
                return Ok(self.seo_publish.map(|s| s.to_string()));
            }
            Ok(None)
        }
        async fn do_set_setting(
            &self,
            _key: &str,
            _value: &str,
            _category: &str,
            _is_secret: bool,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_get_all_settings(
            &self,
        ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
            Ok(vec![])
        }
        async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
            Ok(false)
        }
        async fn set_auto_expression_enabled(&self, _enabled: bool) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    fn pipeline_with_flag(value: Option<&'static str>) -> PublishPipeline {
        PublishPipeline::new(vec![])
            .with_settings(Arc::new(FixedFlagSettings { seo_publish: value }))
    }

    #[tokio::test]
    async fn test_run_job_blocked_when_flag_disabled() {
        // Negative: フラグ false → ゲートで遮断（"seo_publish" を含む）
        let p = pipeline_with_flag(Some("false"));
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("seo_publish"), "got: {err}");
    }

    #[tokio::test]
    async fn test_run_job_blocked_when_flag_unset() {
        // Fail-Closed: フラグ未設定 → 遮断
        let p = pipeline_with_flag(None);
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("seo_publish"), "got: {err}");
    }

    #[tokio::test]
    async fn test_run_job_passes_gate_when_flag_enabled() {
        // Positive: フラグ true → ゲート通過（publishers 空なので "Publisher not found" に到達）
        let p = pipeline_with_flag(Some("true"));
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Publisher not found"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn test_run_job_no_settings_keeps_legacy_behavior() {
        // 互換: settings 未接続（テスト構成）はゲートなし
        let p = PublishPipeline::new(vec![]);
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Publisher not found"),
            "got: {err}"
        );
    }
}
