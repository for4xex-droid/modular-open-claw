/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::escalator::{SupportIntent, SupportSeverity};
use crate::intent::IntentFirewall;
use aiome_core::error::AiomeError;
use aiome_core_contracts::events::ControlCommand;
use std::sync::Arc;

/// サポートメッセージ分類機
pub struct SupportClassifier {
    firewall: Arc<IntentFirewall>,
}

impl SupportClassifier {
    /// 新規分類機を作成する
    pub fn new(firewall: Arc<IntentFirewall>) -> Self {
        Self { firewall }
    }

    /// メッセージをクレンジング（PII除去）し、サポート意図に分類する
    pub async fn classify(&self, message: &str) -> Result<SupportIntent, AiomeError> {
        let clean = self.firewall.strip_pii(message);

        // TDD用の簡易的キーワード分類（本番はLLMによる精緻な分類が入る）
        if clean.contains("!bug") || clean.contains("error") || clean.contains("fail") {
            let severity = if clean.contains("fatal") || clean.contains("critical") {
                SupportSeverity::Critical
            } else if clean.contains("severe") || clean.contains("high") {
                SupportSeverity::High
            } else if clean.contains("minor") || clean.contains("low") {
                SupportSeverity::Low
            } else {
                SupportSeverity::Medium
            };

            Ok(SupportIntent::BugReport {
                summary: clean,
                severity,
            })
        } else if clean.contains("!help") || clean.contains("support") {
            Ok(SupportIntent::GeneralChat)
        } else {
            Ok(SupportIntent::Unknown(clean))
        }
    }

    /// SupportIntent から ControlCommand へのマッピングを試みる（RED検証用）
    pub fn to_control_command(
        &self,
        intent: &SupportIntent,
        channel_id: u64,
    ) -> Option<ControlCommand> {
        match intent {
            SupportIntent::BugReport { summary, severity } => {
                // ここで ControlCommand::SupportReport を参照する！
                // まだ events.rs に定義されていないため、意図的なコンパイルエラー（RED）となります。
                Some(ControlCommand::SupportReport {
                    message: summary.clone(),
                    channel_id,
                    severity: format!("{:?}", severity),
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_classification_and_command_mapping() {
        let firewall = Arc::new(IntentFirewall::new().unwrap());
        let classifier = SupportClassifier::new(firewall);

        // 1. PII 除去が動作しているか検証
        let raw_msg = "My email is test@example.com and the server crashed with fatal error";
        let intent = classifier.classify(raw_msg).await.unwrap();

        match intent {
            SupportIntent::BugReport { summary, severity } => {
                assert!(!summary.contains("test@example.com"));
                assert!(summary.contains("[EMAIL]"));
                assert_eq!(severity, SupportSeverity::Critical);

                // コマンドへのマッピング確認
                let cmd = classifier.to_control_command(
                    &SupportIntent::BugReport {
                        summary: summary.clone(),
                        severity,
                    },
                    12345,
                );
                assert!(cmd.is_some());
            }
            _ => panic!("Expected BugReport"),
        }
    }
}
