/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::llm_provider::LlmProvider;
use aiome_core::security::PermissionManifest;
use aiome_core::security::RuntimeJail;
use async_trait::async_trait;
use infrastructure::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use infrastructure::immune_system::AdaptiveImmuneSystem;
use infrastructure::security::BastionGuard;
use infrastructure::slo_engine::{SloConfig, SloEngine};
use std::time::Duration;

#[tokio::test]
async fn test_circuit_breaker_lifecycle() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_millis(100),
    };
    let cb = CircuitBreaker::new(config);

    // Initial state: Closed
    assert!(cb.check_state().await.is_ok());

    // 3 failures -> Open
    cb.record_failure().await;
    cb.record_failure().await;
    cb.record_failure().await;

    assert!(cb.check_state().await.is_err());

    // Wait for timeout -> HalfOpen
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(cb.check_state().await.is_ok()); // HalfOpen allows one check

    // Success in HalfOpen -> Closed
    cb.record_success().await;
    assert!(cb.check_state().await.is_ok());

    // Check that failures were reset
    cb.record_failure().await;
    assert!(cb.check_state().await.is_ok());
}

#[tokio::test]
async fn test_slo_engine_budget() {
    let config = SloConfig {
        error_budget_max: 5,
        warning_threshold: 3,
    };
    let engine = SloEngine::new(config, chrono::Duration::seconds(10));

    // Initially 0 consumed
    let (consumed, max) = engine.get_budget_status().await;
    assert_eq!(consumed, 0);
    assert_eq!(max, 5);

    // Record 3 errors -> Warning threshold
    engine.record_error().await;
    engine.record_error().await;
    engine.record_error().await;

    let (consumed, _) = engine.get_budget_status().await;
    assert_eq!(consumed, 3);

    // Record 2 more -> Exhausted
    engine.record_error().await;
    engine.record_error().await;

    let (consumed, _) = engine.get_budget_status().await;
    assert_eq!(consumed, 5);
}

#[tokio::test]
async fn test_slo_engine_reset() {
    let config = SloConfig {
        error_budget_max: 5,
        warning_threshold: 3,
    };
    // Very short period for testing
    let engine = SloEngine::new(config, chrono::Duration::milliseconds(100));

    engine.record_error().await;
    assert_eq!(engine.get_budget_status().await.0, 1);

    // Wait for period to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Status call should trigger reset
    assert_eq!(engine.get_budget_status().await.0, 0);
}

#[tokio::test]
async fn test_bastion_guard_enforcement() {
    use infrastructure::security::{BastionGuard, PermissionManifest};
    let _ = std::fs::create_dir_all("workspace");

    // 1. Blocked by default manifest
    let manifest = PermissionManifest::default();
    let guard = BastionGuard::new(manifest);
    assert!(guard.safe_exec("ls").is_err()); // allow_shell_execution is false

    // 2. Blocked by injection filter
    let mut manifest = PermissionManifest::default();
    manifest.allow_shell_execution = true;
    let guard = BastionGuard::new(manifest);
    assert!(guard.safe_exec("ls && cat /etc/passwd").is_err());

    // 3. Blocked by sensitive path
    assert!(guard.safe_exec("cat /etc/shadow").is_err());

    // 4. Blocked by non-whitelisted binary
    assert!(guard.safe_exec("rm -rf .").is_err());

    // 5. Success with allowed binary (ls)
    // Note: This actually runs 'ls' on the host, which is usually fine for tests.
    let result = guard.safe_exec("ls");
    assert!(
        result.is_ok(),
        "safe_exec failed with: {:?}",
        result.unwrap_err()
    );
}
