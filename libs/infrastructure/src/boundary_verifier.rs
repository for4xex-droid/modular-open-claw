/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use std::path::PathBuf;

/// O(1) で境界トートロジー（不変条件）を検証するエンジン
pub struct BoundaryVerifier {
    workspace_root: PathBuf,
    vault_path: Option<PathBuf>,
}

impl BoundaryVerifier {
    /// 新しいインスタンスを生成する
    pub fn new(workspace_root: PathBuf, vault_path: Option<PathBuf>) -> Self {
        Self {
            workspace_root,
            vault_path,
        }
    }

    /// グローバル設定からインスタンスを生成する
    pub fn from_global_config() -> Self {
        use crate::security::GLOBAL_SECURITY_CONFIG;
        Self::new(
            GLOBAL_SECURITY_CONFIG.workspace_root.clone(),
            GLOBAL_SECURITY_CONFIG.vault_path.clone(),
        )
    }

    /// コマンド文字列を検証し、通過した不変条件名のリストを返す
    pub fn verify_command(
        &self,
        cmd: &str,
        is_system_internal: bool,
    ) -> Result<Vec<String>, AiomeError> {
        let mut verified = Vec::new();

        // 1. メタ文字チェック (O(n))
        // インジェクション防止のトートロジー
        let meta_chars = [";", "&&", "||", "|", ">", "<", "`", "$("];
        for mc in &meta_chars {
            if cmd.contains(mc) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Boundary Invariant Violation: Command contains forbidden meta-character '{}'", mc),
                });
            }
        }
        verified.push("no_meta_characters".into());

        // 2. バイナリ・ホワイトリスト
        // (簡易実装: 最初の単語をチェック)
        let binary = cmd.split_whitespace().next().unwrap_or("");
        let allowed_binaries = ["ls", "cat", "cargo", "git", "echo", "pwd"];
        if !allowed_binaries.contains(&binary) {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Boundary Invariant Violation: Binary '{}' is not in the allowed whitelist",
                    binary
                ),
            });
        }
        verified.push("binary_in_whitelist".into());

        // 3. パス検証
        // コマンドに含まれるパスっぽい文字列を抽出してチェック
        for word in cmd.split_whitespace() {
            if word.starts_with('/') || word.contains('.') {
                let p = std::path::Path::new(word);

                // システムパスへのアクセス禁止
                let system_roots = ["/etc", "/usr", "/System", "/var", "/bin", "/sbin"];
                for root in &system_roots {
                    if word.starts_with(root) {
                        return Err(AiomeError::Infrastructure {
                            reason: format!("Boundary Invariant Violation: Access to system path '{}' is forbidden", word),
                        });
                    }
                }
                verified.push("path_not_system".into());

                // サンドボックス外へのアクセス禁止
                if !is_system_internal {
                    // .env ファイルの直接読み込み禁止
                    if word.ends_with(".env") || word.contains(".env") {
                        return Err(AiomeError::Infrastructure {
                            reason: "Boundary Invariant Violation: Direct access to .env files is forbidden".into(),
                        });
                    }
                }
                verified.push("no_env_access".into());

                // Vault へのアクセス制限
                if let Some(vault) = &self.vault_path {
                    if word.contains(vault.to_str().unwrap_or("")) {
                        if !is_system_internal {
                            return Err(AiomeError::Infrastructure {
                                reason: format!("Boundary Invariant Violation: Path '{}' is in the Vault and requires system internal context", word),
                            });
                        }
                    }
                }
                verified.push("no_vault_access".into());

                // ワークスペース内チェック
                if !is_system_internal && word.starts_with('/') {
                    if !word.starts_with(self.workspace_root.to_str().unwrap_or("")) {
                        // ワークスペース外の絶対パス
                        return Err(AiomeError::Infrastructure {
                            reason: format!(
                                "Boundary Invariant Violation: Path '{}' is outside sandbox jail",
                                word
                            ),
                        });
                    }
                }
                verified.push("path_in_sandbox".into());
            }
        }

        // 重複を除去して返す
        verified.sort();
        verified.dedup();
        Ok(verified)
    }
}
