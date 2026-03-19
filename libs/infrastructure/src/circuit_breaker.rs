/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

//! # Circuit Breaker パターン実装
//!
//! LLM呼び出しや外部サービス連携における障害伝播を防ぐための
//! Circuit Breaker パターンの実装。状態管理と指数バックオフ的
//! フェイルファーストを提供する。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Circuit Breaker の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 正常稼働中
    Closed,
    /// 障害検知により遮断中（即座にエラー返却）
    Open,
    /// 復旧テスト中（次の1回で判定）
    HalfOpen,
}

/// Circuit Breaker の設定
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Open 状態に遷移するまでの連続失敗数
    pub failure_threshold: usize,
    /// Open → HalfOpen に遷移するまでの待機時間
    pub reset_timeout: std::time::Duration,
}

/// Circuit Breaker 本体
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String, // G-29: 識別用のサービス名
    state: Arc<RwLock<CircuitState>>,
    failures: Arc<AtomicUsize>,
    config: CircuitBreakerConfig,
    last_failure_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl CircuitBreaker {
    /// 新しい CircuitBreaker を生成する
    pub fn new(name: &str, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.to_string(),
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failures: Arc::new(AtomicUsize::new(0)),
            config,
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    /// 現在の状態をチェックし、リクエストを通すべきか判定する
    pub async fn check_state(&self) -> Result<(), &'static str> {
        let mut state = self.state.write().await;

        if *state == CircuitState::Open {
            let last_fail = *self.last_failure_time.read().await;
            if let Some(time) = last_fail {
                if time.elapsed() > self.config.reset_timeout {
                    tracing::info!(
                        "CircuitBreaker[{}]: Half-Open state entered. Testing service.",
                        self.name
                    );
                    *state = CircuitState::HalfOpen;
                    return Ok(());
                }
            }
            return Err("CircuitBreaker is OPEN. Failing fast.");
        }
        Ok(())
    }

    /// 成功を記録し、HalfOpen なら Closed に復旧する
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        if *state == CircuitState::HalfOpen {
            tracing::info!(
                "CircuitBreaker[{}]: Service recovered. State -> Closed.",
                self.name
            );
            *state = CircuitState::Closed;
            self.failures.store(0, Ordering::Relaxed);
        } else {
            self.failures.store(0, Ordering::Relaxed);
        }
    }

    /// 失敗を記録し、閾値超過なら Open に遷移する
    pub async fn record_failure(&self) {
        let fails = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        let mut state = self.state.write().await;

        if *state == CircuitState::HalfOpen || fails >= self.config.failure_threshold {
            if *state != CircuitState::Open {
                tracing::warn!(
                    "CircuitBreaker[{}]: Threshold reached. State -> Open.",
                    self.name
                );
                *state = CircuitState::Open;
            }
            let mut last_fail = self.last_failure_time.write().await;
            *last_fail = Some(std::time::Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_circuit_breaker_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new("test-service", config);

        assert!(cb.check_state().await.is_ok());

        cb.record_failure().await;
        assert!(cb.check_state().await.is_ok());

        cb.record_failure().await; // Threshold reached
        assert!(cb.check_state().await.is_err());
        assert_eq!(cb.name, "test-service");
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
        };
        let cb = CircuitBreaker::new("reset-service", config);

        cb.record_failure().await;
        assert!(cb.check_state().await.is_err());

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(cb.check_state().await.is_ok()); // Half-Open
        cb.record_success().await;
        assert!(cb.check_state().await.is_ok()); // Closed
    }
}
