//! # ZombieKiller — 外部プロセスのタイムアウト管理
//!
//! ComfyUI や FFmpeg などの外部プロセスが無限にハングすることを防ぐ。
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
        command: String,
        timeout_secs: u64,
    },
    /// プロセスが非ゼロの終了コードで終了
    NonZeroExit {
        command: String,
        exit_code: i32,
        stderr: String,
    },
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
/// * `program` - 実行するプログラム名 (例: "ffmpeg", "curl")
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
/// ComfyUI API 等への通信に使用する。
pub fn http_client_with_timeout(timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(5))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_command() {
        let result =
            run_with_timeout("echo", &["hello"], Duration::from_secs(5)).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_timeout_kills_process() {
        // sleep 10 を 1秒のタイムアウトで実行 → 殺される
        let result =
            run_with_timeout("sleep", &["10"], Duration::from_secs(1)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessError::TimedOut { timeout_secs, .. } => {
                assert_eq!(timeout_secs, 1);
            }
            other => panic!("Expected TimedOut, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nonzero_exit() {
        let result =
            run_with_timeout("ls", &["/nonexistent_path_xyz"], Duration::from_secs(5))
                .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessError::NonZeroExit { exit_code, .. } => {
                assert_ne!(exit_code, 0);
            }
            other => panic!("Expected NonZeroExit, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_failed() {
        let result = run_with_timeout(
            "this_program_does_not_exist_xyz",
            &[],
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessError::SpawnFailed(_) => {} // expected
            other => panic!("Expected SpawnFailed, got: {:?}", other),
        }
    }
}
