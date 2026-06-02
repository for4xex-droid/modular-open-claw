/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::alerts::{AlertLevel, AlertManager};
use aiome_core::error::AiomeError;
use std::sync::Arc;

/// サポート問い合わせの分類意図
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportIntent {
    GeneralChat,
    BugReport {
        summary: String,
        severity: SupportSeverity,
    },
    FeatureRequest(String),
    AccountIssue(String),
    Unknown(String),
}

/// 不具合報告の重要度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// サポートエスカレーション管理機
pub struct SupportEscalator {
    alert_manager: Arc<AlertManager>,
}

impl SupportEscalator {
    /// 新規エスカレーターを作成する
    pub fn new(alert_manager: Arc<AlertManager>) -> Self {
        Self { alert_manager }
    }

    /// 必要に応じてアラートによる人間エスカレーションをトリガーする
    /// BugReport で severity が High または Critical の場合にトリガー
    pub async fn escalate_if_needed(
        &self,
        intent: &SupportIntent,
        incident_id: &str,
    ) -> Result<(), AiomeError> {
        match intent {
            SupportIntent::BugReport {
                severity: SupportSeverity::High | SupportSeverity::Critical,
                ..
            } => {
                self.alert_manager
                    .trigger_alert(
                        &format!("🚨 Support Escalation: {}", incident_id),
                        &format!(
                            "[Incident {}] High/Critical bug report received",
                            incident_id
                        ),
                        AlertLevel::Critical,
                    )
                    .await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerts::{AlertLevel, AlertNotifier};

    #[derive(Debug, Default)]
    struct MockNotifier {
        alerts: std::sync::Arc<tokio::sync::Mutex<Vec<(String, String, AlertLevel)>>>,
    }

    #[async_trait::async_trait]
    impl AlertNotifier for MockNotifier {
        async fn send_alert(
            &self,
            title: &str,
            message: &str,
            level: AlertLevel,
        ) -> Result<(), AiomeError> {
            self.alerts
                .lock()
                .await
                .push((title.to_string(), message.to_string(), level));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_escalator_triggers_on_high_critical() {
        let alert_manager = Arc::new(AlertManager::new());
        let notifier = Arc::new(MockNotifier::default());
        alert_manager.register_notifier(notifier.clone()).await;

        let escalator = SupportEscalator::new(alert_manager);

        // 1. Critical の場合はトリガーされる
        let intent_critical = SupportIntent::BugReport {
            summary: "Crash on start".to_string(),
            severity: SupportSeverity::Critical,
        };
        let res = escalator
            .escalate_if_needed(&intent_critical, "inc-100")
            .await;
        assert!(res.is_ok());

        // Wait for async notify spawn
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        {
            let alerts = notifier.alerts.lock().await;
            assert_eq!(alerts.len(), 1);
            assert_eq!(alerts[0].0, "🚨 Support Escalation: inc-100");
            assert_eq!(
                alerts[0].1,
                "[Incident inc-100] High/Critical bug report received"
            );
            assert_eq!(alerts[0].2, AlertLevel::Critical);
        }

        // 2. High の場合もトリガーされる
        let intent_high = SupportIntent::BugReport {
            summary: "Auth failing".to_string(),
            severity: SupportSeverity::High,
        };
        let res = escalator.escalate_if_needed(&intent_high, "inc-101").await;
        assert!(res.is_ok());

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        {
            let alerts = notifier.alerts.lock().await;
            assert_eq!(alerts.len(), 2);
            assert_eq!(alerts[1].0, "🚨 Support Escalation: inc-101");
            assert_eq!(
                alerts[1].1,
                "[Incident inc-101] High/Critical bug report received"
            );
        }

        // 3. Low / Medium の場合はトリガーされない
        let intent_low = SupportIntent::BugReport {
            summary: "Typo in footer".to_string(),
            severity: SupportSeverity::Low,
        };
        let res = escalator.escalate_if_needed(&intent_low, "inc-102").await;
        assert!(res.is_ok());

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        {
            let alerts = notifier.alerts.lock().await;
            assert_eq!(alerts.len(), 2); // 変化なし
        }

        // 4. 一般会話でもトリガーされない
        let intent_chat = SupportIntent::GeneralChat;
        let res = escalator.escalate_if_needed(&intent_chat, "inc-103").await;
        assert!(res.is_ok());

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        {
            let alerts = notifier.alerts.lock().await;
            assert_eq!(alerts.len(), 2); // 変化なし
        }
    }
}
