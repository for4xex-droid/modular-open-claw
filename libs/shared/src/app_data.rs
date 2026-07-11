/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
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
            .unwrap_or_else(|e| panic!("Failed to resolve fallback AIOME_APP_DATA_DIR: {}", e))
        // allow-anti-pattern: fatal Default impl
    }
}

impl AppDataResolver {
    /// 新規作成。内部的に環境変数をチェックしてルートパスを決定します。
    ///
    /// `CELL_ID` 環境変数が設定されている場合、ルートパスの末尾にセル名前空間を追加します。
    /// パストラバーサル防止のため、`CELL_ID` は英数字・ハイフン・アンダースコアのみ許可されます。
    pub fn new() -> Result<Self, String> {
        let cell_id = match std::env::var("CELL_ID") {
            Ok(val) => val,
            Err(_) => {
                let is_test_binary = std::env::current_exe()
                    .map(|p| p.to_string_lossy().contains("/deps/"))
                    .unwrap_or(false);
                if is_test_binary {
                    "test-cell".to_string()
                } else {
                    return Err("🚨 FATAL: CELL_ID is required for AppDataResolver!".to_string());
                }
            }
        };
        if !Self::is_safe_cell_id(&cell_id) {
            return Err(format!("🚨 FATAL: CELL_ID '{}' contains invalid characters. Only [a-zA-Z0-9_-] (max 64 chars) are allowed.", cell_id));
        }

        if let Ok(data_dir) = std::env::var("AIOME_DATA_DIR") {
            if !data_dir.is_empty() {
                if data_dir.contains("..") {
                    return Err(format!(
                        "🚨 FATAL: AIOME_DATA_DIR '{}' contains path traversal sequences.",
                        data_dir
                    ));
                }
                return Ok(Self {
                    root: PathBuf::from(data_dir),
                });
            }
        }

        let is_dev = std::env::var("AIOME_DEV_MODE")
            .map(|v| v == "1")
            .unwrap_or(false);

        let root = if is_dev {
            // 開発モード: カレントディレクトリの workspace/CELL_ID を使用
            let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push("workspace");
            path.push(&cell_id);
            path
        } else {
            // 本番モード: OS標準のデータディレクトリ
            dirs::data_local_dir()
                .map(|mut p| {
                    p.push("com.aiome.nexus");
                    p.push(&cell_id);
                    p
                })
                .unwrap_or_else(|| {
                    let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                    p.push(".aiome");
                    p.push(&cell_id);
                    p
                })
        };

        Ok(Self { root })
    }

    /// `CELL_ID` がパストラバーサルを含まない安全な値かを検証します。
    /// 英数字・ハイフン・アンダースコアのみ許可。最大64文字。
    fn is_safe_cell_id(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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

    /// テスト前に CELL_ID / AIOME_DATA_DIR をクリアするヘルパー。
    fn clean_env_overrides() {
        env::remove_var("CELL_ID");
        env::remove_var("AIOME_DATA_DIR");
        env::remove_var("WORKSPACE_DIR");
    }

    #[test]
    #[serial]
    fn test_resolve_root_dev() {
        clean_env_overrides();
        env::set_var("CELL_ID", "test-cell");
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new().unwrap();
        assert!(resolver.root().to_string_lossy().contains("workspace"));
    }

    #[test]
    #[serial]
    fn test_resolve_root_prod() {
        clean_env_overrides();
        env::set_var("CELL_ID", "test-cell");
        env::remove_var("AIOME_DEV_MODE");
        let resolver = AppDataResolver::new().unwrap();
        #[cfg(target_os = "macos")]
        assert!(resolver
            .root()
            .to_string_lossy()
            .contains("Application Support"));
    }

    #[test]
    #[serial]
    fn test_db_path_resolution() {
        clean_env_overrides();
        env::set_var("CELL_ID", "test-cell");
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new().unwrap();
        let path = resolver.db_path();
        // Since resolve() might return an absolute path, we just check for the components
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("workspace") || path_str.contains("aiome"));
        assert!(path_str.ends_with("aiome.db"));
    }

    #[test]
    #[serial]
    fn test_db_url_resolution() {
        clean_env_overrides();
        env::set_var("CELL_ID", "test-cell");
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new().unwrap();
        let url = resolver.db_url();
        assert!(url.starts_with("sqlite://"));
        assert!(url.contains("aiome.db"));
    }

    #[test]
    #[serial]
    fn test_cell_id_namespacing() {
        clean_env_overrides();
        env::set_var("AIOME_DEV_MODE", "1");
        env::set_var("CELL_ID", "test-cell-42");
        let resolver = AppDataResolver::new().unwrap();

        // root に CELL_ID が含まれること
        let root_str = resolver.root().to_string_lossy().to_string();
        assert!(root_str.contains("test-cell-42"));

        // db_path もセルスコープ内に収束すること
        let db_str = resolver.db_path().to_string_lossy().to_string();
        assert!(db_str.contains("test-cell-42"));
        assert!(db_str.ends_with("test-cell-42/aiome.db"));

        // db_url もセルスコープを含むこと
        let url = resolver.db_url();
        assert!(url.contains("test-cell-42/aiome.db"));

        // artifacts_dir もセルスコープ内であること
        let art_str = resolver.artifacts_dir().to_string_lossy().to_string();
        assert!(art_str.contains("test-cell-42"));
        assert!(art_str.ends_with("test-cell-42/artifacts"));

        clean_env_overrides();
    }

    #[test]
    #[serial]
    fn test_cell_id_absent_defaults_to_test_cell() {
        clean_env_overrides();
        env::set_var("AIOME_DEV_MODE", "1");
        let resolver = AppDataResolver::new().unwrap();
        assert!(resolver.root().to_string_lossy().contains("test-cell"));
    }

    #[test]
    #[serial]
    fn test_cell_id_rejects_traversal() {
        clean_env_overrides();
        env::set_var("AIOME_DEV_MODE", "1");
        env::set_var("CELL_ID", "../../etc");
        let result = AppDataResolver::new();
        clean_env_overrides();
        assert!(
            result.is_err(),
            "Expected Err due to invalid characters in CELL_ID"
        );
    }

    #[test]
    fn test_is_safe_cell_id() {
        assert!(AppDataResolver::is_safe_cell_id("cell-0"));
        assert!(AppDataResolver::is_safe_cell_id("user_abc_123"));
        assert!(AppDataResolver::is_safe_cell_id("a"));
        assert!(!AppDataResolver::is_safe_cell_id(""));
        assert!(!AppDataResolver::is_safe_cell_id("../../etc"));
        assert!(!AppDataResolver::is_safe_cell_id("cell/0"));
        assert!(!AppDataResolver::is_safe_cell_id("cell 0"));
        assert!(!AppDataResolver::is_safe_cell_id(&"a".repeat(65)));
    }

    #[test]
    #[serial]
    fn test_resolve_root_override_via_env() {
        clean_env_overrides();
        let custom_path = env::temp_dir().join("aiome-custom-data-test");
        env::set_var("AIOME_DATA_DIR", custom_path.to_str().unwrap());
        env::set_var("CELL_ID", "override-test-cell");

        let resolver = AppDataResolver::new().unwrap();
        assert_eq!(resolver.root(), custom_path);

        clean_env_overrides();
    }
}
