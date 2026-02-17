//! # PathSandbox — ファイルシステムサンドボックス
//!
//! 全てのファイル操作を許可されたディレクトリ内に閉じ込める「牢獄」。
//! LLM のハルシネーションやプロンプトインジェクションによる
//! ディレクトリ・トラバーサル攻撃を防止する。

use std::path::{Path, PathBuf};

/// ファイルシステム操作を許可ディレクトリに制限するサンドボックス
#[derive(Debug, Clone)]
pub struct PathSandbox {
    /// 許可されたベースディレクトリ（正規化済み絶対パス）
    allowed_roots: Vec<PathBuf>,
}

/// サンドボックス違反エラー
#[derive(Debug, Clone)]
pub struct SandboxViolation {
    pub requested_path: String,
    pub reason: String,
}

impl std::fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "🚫 Sandbox violation: '{}' — {}",
            self.requested_path, self.reason
        )
    }
}

impl PathSandbox {
    /// 許可するベースディレクトリを指定してサンドボックスを作成
    ///
    /// 各パスは `canonicalize` で正規化される。
    /// パスが存在しない場合はそのパスを無視する。
    pub fn new(roots: &[&str]) -> Self {
        let allowed_roots = roots
            .iter()
            .filter_map(|r| std::fs::canonicalize(r).ok())
            .collect();
        Self { allowed_roots }
    }

    /// 指定パスがサンドボックス内にあるか検証し、正規化されたパスを返す
    ///
    /// # Safety checks:
    /// 1. `..` を含むパスを拒否（トラバーサル防止）
    /// 2. `canonicalize` で実体パスに解決
    /// 3. 許可された root のプレフィックス一致を検証
    pub fn validate(&self, path: &str) -> Result<PathBuf, SandboxViolation> {
        let path_obj = Path::new(path);

        // Step 1: 明らかな traversal パターンを即座にブロック
        let path_str = path_obj.to_string_lossy();
        if path_str.contains("..") {
            return Err(SandboxViolation {
                requested_path: path.to_string(),
                reason: "Path contains '..' — directory traversal blocked".to_string(),
            });
        }

        // Step 2: canonicalize で実体パスに解決（シンボリックリンクも解決）
        let canonical = std::fs::canonicalize(path_obj).map_err(|_| SandboxViolation {
            requested_path: path.to_string(),
            reason: "Path does not exist or cannot be resolved".to_string(),
        })?;

        // Step 3: 許可された root のいずれかの配下であることを確認
        let is_allowed = self
            .allowed_roots
            .iter()
            .any(|root| canonical.starts_with(root));

        if !is_allowed {
            return Err(SandboxViolation {
                requested_path: path.to_string(),
                reason: format!(
                    "Path '{}' is outside all allowed roots: {:?}",
                    canonical.display(),
                    self.allowed_roots
                ),
            });
        }

        Ok(canonical)
    }

    /// ファイル書き込み前にサンドボックスを検証するヘルパー
    ///
    /// 親ディレクトリが許可範囲内かを確認する（ファイルがまだ存在しない場合）
    pub fn validate_write_target(&self, path: &str) -> Result<PathBuf, SandboxViolation> {
        let path_obj = Path::new(path);

        // traversal チェック
        let path_str = path_obj.to_string_lossy();
        if path_str.contains("..") {
            return Err(SandboxViolation {
                requested_path: path.to_string(),
                reason: "Path contains '..' — directory traversal blocked".to_string(),
            });
        }

        // 親ディレクトリが存在し、許可範囲内であることを確認
        let parent = path_obj.parent().ok_or_else(|| SandboxViolation {
            requested_path: path.to_string(),
            reason: "Path has no parent directory".to_string(),
        })?;

        let canonical_parent =
            std::fs::canonicalize(parent).map_err(|_| SandboxViolation {
                requested_path: path.to_string(),
                reason: format!(
                    "Parent directory '{}' does not exist",
                    parent.display()
                ),
            })?;

        let is_allowed = self
            .allowed_roots
            .iter()
            .any(|root| canonical_parent.starts_with(root));

        if !is_allowed {
            return Err(SandboxViolation {
                requested_path: path.to_string(),
                reason: format!(
                    "Parent '{}' is outside all allowed roots",
                    canonical_parent.display()
                ),
            });
        }

        Ok(path_obj.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_blocks_traversal_pattern() {
        let sandbox = PathSandbox::new(&["/tmp"]);
        let result = sandbox.validate("/tmp/../etc/passwd");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.reason.contains("traversal"));
    }

    #[test]
    fn test_blocks_double_dot_in_middle() {
        let sandbox = PathSandbox::new(&["/tmp"]);
        let result = sandbox.validate("/tmp/safe/../../etc/shadow");
        assert!(result.is_err());
    }

    #[test]
    fn test_allows_path_within_sandbox() {
        // 現在のディレクトリをサンドボックスとして使用
        let cwd = env::current_dir().unwrap();
        let cwd_str = cwd.to_str().unwrap();
        let sandbox = PathSandbox::new(&[cwd_str]);

        // Cargo.toml は必ず存在する
        let result = sandbox.validate("Cargo.toml");
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocks_path_outside_sandbox() {
        // /tmp をサンドボックスにして、/etc は拒否されることを確認
        let sandbox = PathSandbox::new(&["/tmp"]);
        let result = sandbox.validate("/etc/hosts");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_nonexistent_path() {
        let sandbox = PathSandbox::new(&["/tmp"]);
        let result = sandbox.validate("/tmp/this_path_definitely_does_not_exist_12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_target_blocks_traversal() {
        let sandbox = PathSandbox::new(&["/tmp"]);
        let result = sandbox.validate_write_target("/tmp/../etc/evil.txt");
        assert!(result.is_err());
    }
}
