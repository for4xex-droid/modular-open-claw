/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! 実行ポリシー。外部からの設定を読み込み、BoundaryVerifierに渡すための統合データ。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecPolicy {
    pub allowed_binaries: Vec<String>,
    #[serde(default)]
    pub environment_variables: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

impl ExecPolicy {
    pub fn new(allowed_binaries: Vec<String>) -> Self {
        Self {
            allowed_binaries,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_policy_json_deserialization() {
        let json = r#"{
            "allowed_binaries": ["ls", "cat"],
            "environment_variables": ["HOME", "PATH"],
            "timeout_seconds": 30
        }"#;

        let policy: ExecPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.allowed_binaries, vec!["ls", "cat"]);
        assert_eq!(policy.environment_variables, vec!["HOME", "PATH"]);
        assert_eq!(policy.timeout_seconds, Some(30));
    }

    #[test]
    fn test_exec_policy_default() {
        let policy = ExecPolicy::default();
        assert!(policy.allowed_binaries.is_empty());
        assert!(policy.environment_variables.is_empty());
        assert_eq!(policy.timeout_seconds, None);
    }
}
