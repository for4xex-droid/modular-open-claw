/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use crate::alerts::{AlertLevel, AlertManager, AlertNotifier, DiscordNotifier};
    use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    use aiome_core::error::AiomeError;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    // テスト用の Mock 通知機
    struct MockNotifier {
        sent: Arc<Mutex<Vec<(AlertLevel, String, String)>>>,
        should_fail: bool,
    }

    #[async_trait]
    impl AlertNotifier for MockNotifier {
        async fn send_alert(
            &self,
            title: &str,
            message: &str,
            level: AlertLevel,
        ) -> Result<(), AiomeError> {
            if self.should_fail {
                return Err(AiomeError::Infrastructure {
                    reason: "Simulated network failure".to_string(),
                });
            }
            let mut sent = self.sent.lock().await;
            sent.push((level, title.to_string(), message.to_string()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_alert_routing_by_level() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let notifier = Arc::new(MockNotifier {
            sent: sent.clone(),
            should_fail: false,
        });

        let manager = AlertManager::new();
        manager.register_notifier(notifier).await;

        // アラート送信
        manager
            .trigger_alert("Test Subject", "Something happened", AlertLevel::Warning)
            .await
            .unwrap();

        // 非同期送信タスクが完了するのを少し待つ
        tokio::time::sleep(Duration::from_millis(60)).await;

        // 送信レコードの確認
        let records = sent.lock().await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].0, AlertLevel::Warning);
        assert_eq!(records[0].1, "Test Subject");
        assert_eq!(records[0].2, "Something happened");
    }

    #[tokio::test]
    async fn test_circuit_breaker_triggers_alert() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let notifier = Arc::new(MockNotifier {
            sent: sent.clone(),
            should_fail: false,
        });

        let manager = Arc::new(AlertManager::new());
        manager.register_notifier(notifier).await;

        // CircuitBreaker の準備
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
        };

        // CircuitBreaker に AlertManager をアタッチ (record_failure 時に通知するよう連携)
        let cb = CircuitBreaker::new_with_alerts("test-service", config, manager);

        // 1回失敗させる ➔ トリップ（Open）してアラートがトリガーされるはず
        cb.record_failure().await;

        // 非同期送信タスクが完了するのを少し待つ
        tokio::time::sleep(Duration::from_millis(60)).await;

        // アラートが Critical レベルで送信されたことを検証
        let records = sent.lock().await;
        assert!(!records.is_empty());
        assert_eq!(records[0].0, AlertLevel::Critical);
        assert!(records[0].1.contains("test-service"));
        assert!(records[0].2.contains("entered Open state"));
    }

    #[tokio::test]
    async fn test_alert_notifier_network_failure_failsafe() {
        let sent_ok = Arc::new(Mutex::new(Vec::new()));

        // 正常な notifier と、エラーを起こす notifier を両方登録
        let notifier_ok = Arc::new(MockNotifier {
            sent: sent_ok.clone(),
            should_fail: false,
        });
        let notifier_fail = Arc::new(MockNotifier {
            sent: Arc::new(Mutex::new(Vec::new())),
            should_fail: true,
        });

        let manager = AlertManager::new();
        manager.register_notifier(notifier_fail).await;
        manager.register_notifier(notifier_ok).await;

        // 送信が全体でパニック/エラー終了せず、正常な notifier には届くことを確認 (Fail-Safe)
        let res = manager
            .trigger_alert(
                "Failsafe Test",
                "Testing failure tolerance",
                AlertLevel::Info,
            )
            .await;

        assert!(res.is_ok());

        // 非同期送信タスクが完了するのを少し待つ
        tokio::time::sleep(Duration::from_millis(60)).await;

        let records = sent_ok.lock().await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].0, AlertLevel::Info);
    }

    #[tokio::test]
    async fn test_alert_manager_debounce() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let notifier = Arc::new(MockNotifier {
            sent: sent.clone(),
            should_fail: false,
        });

        let manager = AlertManager::new();
        manager.register_notifier(notifier).await;

        // 1回目のアラート送信 (Warning)
        manager
            .trigger_alert("Debounce Subject", "First message", AlertLevel::Warning)
            .await
            .unwrap();

        // 2回目の同一アラート送信 (Warning) -> デバウンスされて無視されるはず
        manager
            .trigger_alert("Debounce Subject", "Second message", AlertLevel::Warning)
            .await
            .unwrap();

        // 異なるレベルのアラート送信 (Info) -> 抑制されず送信されるはず
        manager
            .trigger_alert("Debounce Subject", "Info level message", AlertLevel::Info)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(60)).await;

        let records = sent.lock().await;
        // 最初のアラート(Warning)と、異なるレベルのアラート(Info)の計2回だけが届くはず
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].0, AlertLevel::Warning);
        assert_eq!(records[0].2, "First message");
        assert_eq!(records[1].0, AlertLevel::Info);
        assert_eq!(records[1].2, "Info level message");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_discord_notifier_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Discord webhook は通常成功すると 204 No Content を返す
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        std::env::set_var("DISCORD_WEBHOOK_URL", mock_server.uri());

        let notifier = DiscordNotifier::new();
        let res = notifier
            .send_alert(
                "Test Embed Title",
                "Embedded message detail",
                AlertLevel::Warning,
            )
            .await;

        assert!(res.is_ok());

        std::env::remove_var("DISCORD_WEBHOOK_URL");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_discord_notifier_missing_webhook_url() {
        std::env::remove_var("DISCORD_WEBHOOK_URL");

        let notifier = DiscordNotifier::new();
        // Webhook URL がなくてもエラーを出さず、Ok(()) でスキップされることを確認 (フェイルセーフ)
        let res = notifier
            .send_alert("Skip Alert", "Webhook URL is missing", AlertLevel::Info)
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_discord_notifier_network_failure() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Webhook 呼び出し時に 500 エラーを返すように設定 (異常系障害注入)
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        std::env::set_var("DISCORD_WEBHOOK_URL", mock_server.uri());

        let notifier = DiscordNotifier::new();
        let res = notifier
            .send_alert(
                "Failure Title",
                "Should fail due to server error",
                AlertLevel::Critical,
            )
            .await;

        // ネットワーク/サーバーエラー時には Err を返すことを検証
        assert!(res.is_err());

        std::env::remove_var("DISCORD_WEBHOOK_URL");
    }
}
