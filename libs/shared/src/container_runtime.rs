/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! コンテナランタイム検出のシングルソースオブトゥルース (SSOT)。
//!
//! 検出順序:
//! 1. 環境変数 `CONTAINER_RUNTIME` (`podman` | `docker`) — 明示的オーバーライド
//! 2. `podman --version` が成功 → `"podman"`
//! 3. フォールバック → `"docker"`
//!
//! 一度検出されたランタイムは `OnceLock` でキャッシュされ、プロセス寿命間で再利用されます。

use std::sync::OnceLock;

static DETECTED_RUNTIME: OnceLock<String> = OnceLock::new();

/// コンテナランタイム名を返します (`"podman"` or `"docker"`)。
///
/// 結果はプロセス内でキャッシュされるため、2回目以降の呼び出しコストはゼロです。
pub fn detect_runtime() -> &'static str {
    DETECTED_RUNTIME.get_or_init(|| {
        // 1. 環境変数による明示的オーバーライド
        if let Ok(val) = std::env::var("CONTAINER_RUNTIME") {
            let val = val.trim().to_lowercase();
            if val == "podman" || val == "docker" {
                return val;
            }
            // 不明な値はログで警告しフォールバック
            eprintln!("[container_runtime] WARNING: Unknown CONTAINER_RUNTIME='{}'. Falling back to auto-detection. Valid values: 'podman', 'docker'.", val);
        }

        // 2. Podman の存在チェック
        let mut cmd = std::process::Command::new("podman");
        cmd.arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::security::harden_command(&mut cmd);

        if cmd.status().map(|s| s.success()).unwrap_or(false)
        {
            return "podman".to_string();
        }

        // 3. デフォルト
        "docker".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_runtime_returns_known_value() {
        let rt = detect_runtime();
        assert!(
            rt == "podman" || rt == "docker",
            "Expected 'podman' or 'docker', got '{}'",
            rt
        );
    }

    #[test]
    fn test_detect_runtime_is_idempotent() {
        let rt1 = detect_runtime();
        let rt2 = detect_runtime();
        assert_eq!(rt1, rt2, "detect_runtime must be idempotent");
    }
}
