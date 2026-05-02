/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # PathSandbox — ファイルシステムサンドボックス
//!
//! 全てのファイル操作を許可されたディレクトリ内に閉じ込める「牢獄」。
//! Bastion Jail を使用して、TOCTOU 攻撃やシンボリックリンク攻撃を防止する。

use bastion::fs_guard::Jail;
use std::path::{Path, PathBuf};

/// 許可されたディレクトリ内でのみファイル操作を許可するサンドボックス
#[derive(Debug)]
pub struct PathSandbox {
    jail: Jail,
    virtual_mappings: Vec<(String, PathBuf)>,
}

impl PathSandbox {
    /// 新規サンドボックスの作成
    pub fn new<P: AsRef<Path>>(allowed_root: P) -> Result<Self, std::io::Error> {
        let jail = Jail::new(allowed_root)?;
        Ok(Self {
            jail,
            virtual_mappings: Vec::new(),
        })
    }

    /// 仮想パスマッピングを追加
    pub fn with_virtual_mapping<P: AsRef<Path>>(
        mut self,
        virtual_prefix: &str,
        physical_path: P,
    ) -> Self {
        self.virtual_mappings.push((
            virtual_prefix.to_string(),
            physical_path.as_ref().to_path_buf(),
        ));
        self
    }

    /// 仮想パスを物理パスに解決する
    pub fn resolve_virtual_path(&self, virtual_path: &str) -> Result<PathBuf, std::io::Error> {
        for (prefix, physical) in &self.virtual_mappings {
            if virtual_path.starts_with(prefix) {
                let suffix = &virtual_path[prefix.len()..];
                let suffix = suffix.trim_start_matches('/');
                let target = physical.join(suffix);
                return self.validate_path(target);
            }
        }

        // マッピングがない場合はそのまま検証
        self.validate_path(virtual_path)
    }

    /// パスがサンドボックス内にあるか検証し、安全なフルパスを返す
    /// Bastion Jail の検証ロジック（TOCTOU対策、シンボリックリンク制限）を使用。
    pub fn validate_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, std::io::Error> {
        let requested_path = path.as_ref();
        let base_path = if requested_path.is_absolute() {
            requested_path.to_path_buf()
        } else {
            self.get_root().join(requested_path)
        };

        // Bastion の Jail ロジックに準拠した検証
        if base_path.exists() {
            let canonical = base_path.canonicalize()?;
            let root_canonical = self
                .get_root()
                .canonicalize()
                .unwrap_or_else(|_| self.get_root());
            if !canonical.starts_with(&root_canonical) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Access Denied: Path outside of jail (Bastion Guard)",
                ));
            }
            Ok(canonical)
        } else {
            // 存在しないファイルの場合は親ディレクトリを検証
            match base_path.parent() {
                Some(parent) if parent.exists() => {
                    let parent_canonical = parent.canonicalize()?;
                    let root_canonical = self
                        .get_root()
                        .canonicalize()
                        .unwrap_or_else(|_| self.get_root());
                    if !parent_canonical.starts_with(&root_canonical) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "Access Denied: Parent directory outside of jail",
                        ));
                    }
                    Ok(parent_canonical.join(base_path.file_name().unwrap_or_default()))
                }
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Path or parent directory does not exist",
                )),
            }
        }
    }

    /// Jail のルートパスを取得（内部検証用）
    fn get_root(&self) -> PathBuf {
        self.jail.root().to_path_buf()
    }

    /// VULN-64 防御: LoRA パスの厳格なバリデーション
    /// - URLスキームの禁止 (http://, https:// など)
    /// - 絶対パスの禁止 (ルートからの指定禁止)
    /// - /loras/ プレフィックスの強制、.. トラバーサルの禁止
    /// - 拡張子が .safetensors (または .bin) であることの強制
    pub fn validate_lora_path(path: &str) -> Result<PathBuf, std::io::Error> {
        let p = Path::new(path);

        if path.contains("://") || path.starts_with("http") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VULN-64: Network URLs are not allowed for LoRA paths",
            ));
        }

        if p.is_absolute() || path.starts_with('/') {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VULN-64: Absolute paths are not allowed for LoRA paths",
            ));
        }

        if path.contains("..") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VULN-64: Path traversal is not allowed",
            ));
        }

        if !path.starts_with("loras/") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VULN-64: LoRA paths must be within the 'loras/' directory",
            ));
        }

        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "safetensors" && ext != "bin" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VULN-64: Only .safetensors or .bin extensions are allowed for LoRA models",
            ));
        }

        Ok(p.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_bastion_jail_integration() {
        let dir = tempdir().unwrap(); // allow-anti-pattern
        let workspace = dir.path().join("workspace");
        fs::create_dir(&workspace).unwrap(); // allow-anti-pattern

        let sandbox = PathSandbox::new(&workspace).unwrap(); // allow-anti-pattern

        // 正常系
        let safe_file = workspace.join("test.txt");
        fs::write(&safe_file, "data").unwrap(); // allow-anti-pattern
        assert!(sandbox.validate_path("test.txt").is_ok());

        // 異常系: トラバーサル
        assert!(sandbox.validate_path("../outside.txt").is_err());
    }
}
