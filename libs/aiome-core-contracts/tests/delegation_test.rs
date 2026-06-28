/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::delegation::{DelegationFailureKind, DelegationResult};

#[test]
fn test_delegation_result_success() {
    let result = DelegationResult {
        stdout: "success".to_string(),
        stderr: "".to_string(),
        exit_code: 0,
        duration_ms: 120,
    };
    assert!(result.is_success());
    assert_eq!(result.failure_kind(), DelegationFailureKind::None);
}

#[test]
fn test_delegation_result_timeout() {
    let result = DelegationResult {
        stdout: "".to_string(),
        stderr: "execution timed out".to_string(),
        exit_code: 124,
        duration_ms: 5000,
    };
    assert!(!result.is_success());
    assert_eq!(result.failure_kind(), DelegationFailureKind::Timeout);
}

#[test]
fn test_delegation_result_oom() {
    let result = DelegationResult {
        stdout: "".to_string(),
        stderr: "Out of memory".to_string(),
        exit_code: 137,
        duration_ms: 200,
    };
    assert!(!result.is_success());
    assert_eq!(result.failure_kind(), DelegationFailureKind::Oom);
}
