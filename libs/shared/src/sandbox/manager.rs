/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! サンドボックスを一元管理する SandboxManager

use super::path::PathSandbox;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 複数のサンドボックス（仮想マウント等）を統合管理するマネージャ
#[derive(Debug, Clone)]
pub struct SandboxManager {
    primary_sandbox: Arc<PathSandbox>,
}

impl SandboxManager {
    /// SandboxManager を作成
    pub fn new<P: AsRef<Path>>(allowed_root: P) -> Result<Self, std::io::Error> {
        let sandbox = PathSandbox::new(allowed_root)?;
        Ok(Self {
            primary_sandbox: Arc::new(sandbox),
        })
    }

    /// primary_sandbox に対するアクセッサ
    pub fn primary(&self) -> &PathSandbox {
        &self.primary_sandbox
    }

    /// 仮想パスを物理パスに解決
    pub fn resolve(&self, virtual_path: &str) -> Result<PathBuf, std::io::Error> {
        self.primary_sandbox.resolve_virtual_path(virtual_path)
    }

    /// 仮想パスの安全な解決と、ファイル読み取りを行うユーティリティ
    pub fn read_to_string(&self, virtual_path: &str) -> Result<String, std::io::Error> {
        let resolved = self.resolve(virtual_path)?;
        std::fs::read_to_string(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_sandbox_manager_resolve() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir(&root).unwrap();

        let safe_file = root.join("test.txt");
        fs::write(&safe_file, "manager test").unwrap();

        let manager = SandboxManager::new(&root).unwrap();
        let resolved = manager.resolve("test.txt").unwrap();
        assert_eq!(resolved.file_name().unwrap(), "test.txt");

        let content = manager.read_to_string("test.txt").unwrap();
        assert_eq!(content, "manager test");
    }
}
