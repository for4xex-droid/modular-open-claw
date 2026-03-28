/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

//! # ZombieKiller — 外部プロセスのタイムアウト管理
//!
//! 外部プロセスが無限にハングすることを防ぐ。
//! 全ての外部呼び出しに「冷徹な損切りロジック」を強制する。

use std::process::Output;
use std::time::Duration;
use tokio::process::Command;

/// 外部プロセスのタイムアウトエラー
#[derive(Debug)]
pub enum ProcessError {
    /// プロセスの起動に失敗
    SpawnFailed(std::io::Error),
    /// タイムアウトにより強制終了
    TimedOut {
        /// 実行しようとしたコマンド
        command: String,
        /// 設定されたタイムアウト秒数
        timeout_secs: u64,
    },
    /// プロセスが非ゼロの終了コードで終了
    NonZeroExit {
        /// 実行しようとしたコマンド
        command: String,
        /// 返された終了コード
        exit_code: i32,
        /// パースされた標準エラー出力
        stderr: String,
    },
}

impl From<ProcessError> for aiome_contracts::error::AiomeError {
    fn from(e: ProcessError) -> Self {
        match e {
            ProcessError::TimedOut { timeout_secs, .. } => {
                aiome_contracts::error::AiomeError::RemoteServiceTimeout { timeout_secs }
            }
            ProcessError::SpawnFailed(err) => {
                if err.kind() == std::io::ErrorKind::PermissionDenied {
                    aiome_contracts::error::AiomeError::SecurityViolation {
                        reason: format!("Process spawn denied: {}", err),
                    }
                } else {
                    aiome_contracts::error::AiomeError::OsError {
                        source: anyhow::anyhow!("{}", err),
                    }
                }
            }
            ProcessError::NonZeroExit {
                command,
                exit_code,
                stderr,
            } => aiome_contracts::error::AiomeError::SubprocessFailed {
                reason: format!(
                    "Command '{}' failed with code {}. Stderr: {}",
                    command, exit_code, stderr
                ),
            },
        }
    }
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::SpawnFailed(e) => write!(f, "Process spawn failed: {}", e),
            ProcessError::TimedOut {
                command,
                timeout_secs,
            } => {
                write!(
                    f,
                    "⏰ Process '{}' timed out after {}s — killed",
                    command, timeout_secs
                )
            }
            ProcessError::NonZeroExit {
                command,
                exit_code,
                stderr,
            } => {
                write!(
                    f,
                    "💀 Process '{}' exited with code {}: {}",
                    command, exit_code, stderr
                )
            }
        }
    }
}

impl std::error::Error for ProcessError {}

/// タイムアウト付きで外部プロセスを実行する
///
/// # Arguments
/// * `program` - 実行するプログラム名 (例: "curl", "python")
/// * `args` - コマンドライン引数
/// * `timeout` - タイムアウト時間
///
/// # Returns
/// タイムアウト内に正常終了した場合のみ `Ok(Output)` を返す。
/// タイムアウトした場合はプロセスを kill して `Err` を返す。
pub async fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, ProcessError> {
    if !crate::security::GLOBAL_SECURITY_CONFIG
        .allowed_binaries
        .contains(&program.to_string())
    {
        return Err(ProcessError::SpawnFailed(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Security Violation: Binary '{}' is not in the whitelist.",
                program
            ),
        )));
    }

    let mut child = Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(ProcessError::SpawnFailed)?;

    let cmd_str = format!("{} {}", program, args.join(" "));
    let timeout_secs = timeout.as_secs();

    // stdout/stderr を先に取り出す（所有権の問題を回避）
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // タイムアウト付きで完了を待つ
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => {
            // プロセスは時間内に終了した — 出力を読み取る
            let stdout = match stdout_handle {
                Some(mut out) => {
                    let mut buf = Vec::new();
                    tokio::io::AsyncReadExt::read_to_end(&mut out, &mut buf)
                        .await
                        .unwrap_or_default();
                    buf
                }
                None => Vec::new(),
            };
            let stderr = match stderr_handle {
                Some(mut err) => {
                    let mut buf = Vec::new();
                    tokio::io::AsyncReadExt::read_to_end(&mut err, &mut buf)
                        .await
                        .unwrap_or_default();
                    buf
                }
                None => Vec::new(),
            };

            let output = Output {
                status,
                stdout,
                stderr,
            };

            if output.status.success() {
                Ok(output)
            } else {
                let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
                Err(ProcessError::NonZeroExit {
                    command: cmd_str,
                    exit_code: output.status.code().unwrap_or(-1),
                    stderr: stderr_str,
                })
            }
        }
        Ok(Err(e)) => {
            // wait 自体が失敗
            Err(ProcessError::SpawnFailed(e))
        }
        Err(_) => {
            // タイムアウト！ プロセスを殺す
            let _ = child.kill().await;
            Err(ProcessError::TimedOut {
                command: cmd_str,
                timeout_secs,
            })
        }
    }
}

/// HTTP リクエスト用のタイムアウト付きクライアントを生成
///
/// 外部API等への通信に使用する。
pub fn http_client_with_timeout(timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(5))
        // SEC-5 FIX: SSRF prevention via redirect blocking
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

/// Vec<String> 版のタイムアウト付き実行
pub async fn run_with_timeout_vec(
    program: &str,
    args: Vec<String>,
    timeout: Duration,
) -> Result<Output, ProcessError> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_with_timeout(program, &args_str, timeout).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_command() {
        let result = run_with_timeout("echo", &["hello"], Duration::from_secs(5)).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_whitelist_rejection() {
        // `sleep` is NOT in the whitelist → should be rejected at the gate
        let result = run_with_timeout("sleep", &["10"], Duration::from_secs(1)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessError::SpawnFailed(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
                assert!(e.to_string().contains("not in the whitelist"));
            }
            other => panic!("Expected SpawnFailed(PermissionDenied), got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nonzero_exit() {
        let result =
            run_with_timeout("ls", &["/nonexistent_path_xyz"], Duration::from_secs(5)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessError::NonZeroExit { exit_code, .. } => {
                assert_ne!(exit_code, 0);
            }
            other => panic!("Expected NonZeroExit, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_failed_whitelist() {
        // Non-existent program also fails at whitelist check
        let result = run_with_timeout(
            "this_program_does_not_exist_xyz",
            &[],
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessError::SpawnFailed(e) => {
                // Whitelist rejection comes first
                assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
            }
            other => panic!("Expected SpawnFailed, got: {:?}", other),
        }
    }
}
