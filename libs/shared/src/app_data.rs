/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use std::path::{Path, PathBuf};

/// Aiome のアプリケーションデータを統合管理・解決する構造体。
/// 環境（Dev / Prod / Tauri）に応じて適切な物理パスを返します。
#[derive(Debug, Clone)]
pub struct AppDataResolver {
    root: PathBuf,
}

impl Default for AppDataResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AppDataResolver {
    /// 新規作成。内部的に環境変数をチェックしてルートパスを決定します。
    pub fn new() -> Self {
        let is_dev = std::env::var("AIOME_DEV_MODE")
            .map(|v| v == "1")
            .unwrap_or(false);

        let root = if is_dev {
            // 開発モード: カレントディレクトリの workspace を使用
            let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push("workspace");
            path
        } else {
            // 本番モード: OS標準のデータディレクトリ
            // macOS: ~/Library/Application Support/com.aiome.nexus
            dirs::data_local_dir()
                .map(|mut p| {
                    p.push("com.aiome.nexus");
                    p
                })
                .unwrap_or_else(|| {
                    // フォールバック: ホームディレクトリの .aiome
                    let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                    p.push(".aiome");
                    p
                })
        };

        Self { root }
    }

    /// ルートパスを取得します。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 指定したファイル名やディレクトリをルートパス相対で解決し、絶対パスを返します。
    /// セキュリティのため、".." を含むパスコンポーネントは無視されます。
    pub fn resolve<P: AsRef<Path>>(&self, sub_path: P) -> PathBuf {
        let mut path = self.root.clone();
        for component in sub_path.as_ref().components() {
            match component {
                std::path::Component::Normal(c) => path.push(c),
                std::path::Component::CurDir => {}
                _ => {
                    // Ignore ParentDir (..) and RootDir (/) to prevent traversal
                }
            }
        }
        path
    }

    /// データベースのファイルパスを解決します。
    pub fn db_path(&self) -> PathBuf {
        self.resolve("aiome.db")
    }

    /// データベースの接続文字列 (sqlite://) を解決します。
    pub fn db_url(&self) -> String {
        format!("sqlite://{}", self.db_path().to_string_lossy())
    }

    /// アーティファクト保存用ディレクトリを解決します。
    pub fn artifacts_dir(&self) -> PathBuf {
        self.resolve("artifacts")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_resolve_root_dev() {
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new();
        assert!(resolver.root().to_string_lossy().contains("workspace"));
    }

    #[test]
    #[serial]
    fn test_resolve_root_prod() {
        env::remove_var("AIOME_DEV_MODE");
        let resolver = AppDataResolver::new();
        #[cfg(target_os = "macos")]
        assert!(resolver
            .root()
            .to_string_lossy()
            .contains("Application Support"));
    }

    #[test]
    #[serial]
    fn test_db_path_resolution() {
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new();
        let path = resolver.db_path();
        // Since resolve() might return an absolute path, we just check for the components
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("workspace") || path_str.contains("aiome"));
        assert!(path_str.ends_with("aiome.db"));
    }

    #[test]
    #[serial]
    fn test_db_url_resolution() {
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new();
        let url = resolver.db_url();
        assert!(url.starts_with("sqlite://"));
        assert!(url.contains("aiome.db"));
    }
}
