/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Bootstrap Detector — セットアップが必要かどうかを判定するモジュール (Phase 2B-CORE)
//!
//! Aiome の初回起動時に LLM プロバイダが未設定の場合、通常のフルブートではなく
//! セットアップ専用の WebUI (Bootstrap Mode) で起動する必要がある。
//! このモジュールはその判定ロジックを提供する。

use std::path::Path;

/// 起動モード
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootMode {
    /// 通常モード: すべての設定が完了している
    Normal,
    /// セットアップモード: 初回起動またはLLM未設定
    Setup,
}

/// BootstrapDetector のチェック結果の詳細
#[derive(Debug, Clone)]
pub struct BootstrapDiagnosis {
    /// 起動モード
    pub mode: BootMode,
    /// DBファイルが存在するか
    pub db_exists: bool,
    /// LLM プロバイダが設定されているか (いずれか1つでも)
    pub llm_configured: bool,
    /// API_SERVER_SECRET が設定されているか
    pub api_secret_set: bool,
    /// SOUL.md が存在するか
    pub soul_exists: bool,
    /// 不足している項目のリスト
    pub missing_items: Vec<String>,
}

/// BootstrapDetector: 起動時にシステムのセットアップ状態を診断する
pub struct BootstrapDetector;

impl BootstrapDetector {
    /// 環境変数とファイルシステムの状態からセットアップ完了度を判定する。
    /// 引数によりインメモリの LLM / API シークレット設定状態を優先的に評価可能（パージ対応）。
    pub fn diagnose(
        app_data_root: &Path,
        api_secret_override: Option<bool>,
        llm_override: Option<bool>,
        admin_account_exists: Option<bool>,
    ) -> BootstrapDiagnosis {
        let mut missing = Vec::new();

        // 1. DB の存在チェック
        let db_path = app_data_root.join("aiome.db");
        let db_exists = db_path.exists();

        // 2. LLM プロバイダ設定チェック
        // Ollama (ローカル) または Cloud API Key のいずれかがセットされている必要がある
        let ollama_host = std::env::var("OLLAMA_HOST").ok();
        let gemini_key = std::env::var("GEMINI_API_KEY").ok();
        let openai_key = std::env::var("OPENAI_API_KEY").ok();
        let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();

        // Settings DB からの動的設定も考慮する必要があるが、
        // Bootstrap 判定は DB 接続前に行うため、env のみを参照する
        let llm_configured = llm_override.unwrap_or_else(|| {
            ollama_host.is_some()
                || gemini_key.is_some()
                || openai_key.is_some()
                || anthropic_key.is_some()
        });

        if !llm_configured {
            missing.push(
                "LLM provider (OLLAMA_HOST, GEMINI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY)"
                    .to_string(),
            );
        }

        let api_secret_set =
            api_secret_override.unwrap_or_else(|| std::env::var("API_SERVER_SECRET").is_ok());
        if !api_secret_set {
            missing.push("API_SERVER_SECRET".to_string());
        }

        // 4. SOUL.md 存在チェック
        let soul_path = app_data_root.join("SOUL.md");
        let soul_exists = soul_path.exists();
        // SOUL.md は Bootstrap Mode で自動生成するので、missing に追加しない

        // モード判定: LLM未設定 かつ DB未存在 ならセットアップモード
        // また、DBが存在していても管理者アカウントが未作成ならセットアップモードを維持
        let mode = if (!llm_configured && !db_exists) || !admin_account_exists.unwrap_or(true) {
            BootMode::Setup
        } else {
            BootMode::Normal
        };

        BootstrapDiagnosis {
            mode,
            db_exists,
            llm_configured,
            api_secret_set,
            soul_exists,
            missing_items: missing,
        }
    }
}

/// Factory Reset: アプリケーションデータを初期化する (Phase 2B-4)
pub struct FactoryReset;

impl FactoryReset {
    /// 指定されたアプリデータディレクトリ内のユーザーデータを削除する。
    ///
    /// 削除対象:
    /// - aiome.db (メインデータベース)
    /// - artifacts/ (成果物)
    /// - wasm_storage/ (WASM スキル)
    /// - vault/ (DRM保護データ)
    /// - SOUL.md (魂ファイル)
    ///
    /// 保持対象:
    /// - .env (認証情報は保持、再入力の手間を回避)
    /// - logs/ (監査ログは法的保持義務の可能性)
    pub fn execute(app_data_root: &Path) -> Result<FactoryResetReport, FactoryResetError> {
        if !app_data_root.exists() {
            return Err(FactoryResetError::DirectoryNotFound(
                app_data_root.to_string_lossy().to_string(),
            ));
        }

        let mut report = FactoryResetReport {
            deleted_files: Vec::new(),
            deleted_dirs: Vec::new(),
            preserved_files: Vec::new(),
            errors: Vec::new(),
        };

        // ファイル削除対象
        let files_to_delete = ["aiome.db", "aiome.db-shm", "aiome.db-wal", "SOUL.md"];
        for file in &files_to_delete {
            let path = app_data_root.join(file);
            if path.exists() {
                match std::fs::remove_file(&path) {
                    Ok(()) => report.deleted_files.push(file.to_string()),
                    Err(e) => report
                        .errors
                        .push(format!("Failed to delete {}: {}", file, e)),
                }
            }
        }

        // ディレクトリ削除対象
        let dirs_to_delete = [
            "artifacts",
            "wasm_storage",
            "vault",
            "sandbox",
            "forge_template",
        ];
        for dir in &dirs_to_delete {
            let path = app_data_root.join(dir);
            if path.exists() {
                match std::fs::remove_dir_all(&path) {
                    Ok(()) => report.deleted_dirs.push(dir.to_string()),
                    Err(e) => report
                        .errors
                        .push(format!("Failed to delete dir {}: {}", dir, e)),
                }
            }
        }

        // 保持するファイルを記録
        let preserved = [".env", "logs"];
        for item in &preserved {
            let path = app_data_root.join(item);
            if path.exists() {
                report.preserved_files.push(item.to_string());
            }
        }

        // 部分成功も含めてレポートを返す
        Ok(report)
    }
}

/// Factory Reset の実行レポート
#[derive(Debug, Clone)]
pub struct FactoryResetReport {
    /// 削除に成功したファイル
    pub deleted_files: Vec<String>,
    /// 削除に成功したディレクトリ
    pub deleted_dirs: Vec<String>,
    /// 保持されたファイル/ディレクトリ
    pub preserved_files: Vec<String>,
    /// エラーが発生した項目
    pub errors: Vec<String>,
}

/// Factory Reset のエラー
#[derive(Debug, Clone)]
pub enum FactoryResetError {
    /// 指定されたディレクトリが存在しない
    DirectoryNotFound(String),
}

impl std::fmt::Display for FactoryResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DirectoryNotFound(path) => write!(f, "App data directory not found: {}", path),
        }
    }
}

impl std::error::Error for FactoryResetError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    // ===== BootstrapDetector Tests =====

    #[test]
    #[serial]
    fn test_fresh_install_returns_setup_mode() {
        // Arrange: 何も設定されていない初回起動状態
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("API_SERVER_SECRET");
        let tmp = TempDir::new().unwrap();

        // Act
        let result = BootstrapDetector::diagnose(tmp.path(), None, None, None);

        // Assert
        assert_eq!(result.mode, BootMode::Setup);
        assert!(!result.db_exists);
        assert!(!result.llm_configured);
        assert!(!result.api_secret_set);
        assert!(!result.soul_exists);
        assert!(!result.missing_items.is_empty());
    }

    #[test]
    #[serial]
    fn test_configured_system_returns_normal_mode() {
        // Arrange: まず全 LLM 変数をクリアしてから Ollama のみ設定
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::set_var("OLLAMA_HOST", "http://127.0.0.1:11434");
        std::env::set_var("API_SERVER_SECRET", "test_secret_123");
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("aiome.db"), "dummy_db").unwrap();
        std::fs::write(tmp.path().join("SOUL.md"), "# My Soul").unwrap();

        // Act
        let result = BootstrapDetector::diagnose(tmp.path(), None, None, None);

        // Assert
        assert_eq!(result.mode, BootMode::Normal);
        assert!(result.db_exists);
        assert!(result.llm_configured);
        assert!(result.api_secret_set);
        assert!(result.soul_exists);
        assert!(result.missing_items.is_empty());

        // Cleanup
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("API_SERVER_SECRET");
    }

    #[test]
    #[serial]
    fn test_gemini_key_alone_is_sufficient_for_llm() {
        // Arrange: Gemini API Key のみ設定
        std::env::remove_var("OLLAMA_HOST");
        std::env::set_var("GEMINI_API_KEY", "test-gemini-key");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("aiome.db"), "dummy_db").unwrap();

        // Act
        let result = BootstrapDetector::diagnose(tmp.path(), None, None, None);

        // Assert: LLM が設定済みなので Normal
        assert_eq!(result.mode, BootMode::Normal);
        assert!(result.llm_configured);

        // Cleanup
        std::env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    #[serial]
    fn test_llm_configured_but_no_db_returns_normal() {
        // Arrange: LLM は設定済みだが DB はまだない (初回起動で DB は自動生成される)
        std::env::set_var("OLLAMA_HOST", "http://127.0.0.1:11434");
        let tmp = TempDir::new().unwrap();

        // Act
        let result = BootstrapDetector::diagnose(tmp.path(), None, None, None);

        // Assert: LLM があれば Normal (DB は boot_sequence で自動作成)
        assert_eq!(result.mode, BootMode::Normal);

        // Cleanup
        std::env::remove_var("OLLAMA_HOST");
    }

    #[test]
    #[serial]
    fn test_admin_account_missing_returns_setup_mode() {
        // Arrange: LLMもDBもあるが、アカウント未作成
        std::env::set_var("OLLAMA_HOST", "http://127.0.0.1:11434");
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("aiome.db"), "dummy_db").unwrap();

        // Act: admin_account_exists = Some(false)
        let result = BootstrapDetector::diagnose(tmp.path(), None, None, Some(false));

        // Assert
        assert_eq!(result.mode, BootMode::Setup);

        // Cleanup
        std::env::remove_var("OLLAMA_HOST");
    }

    #[test]
    #[serial]
    fn test_missing_items_list_accuracy() {
        // Arrange
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("API_SERVER_SECRET");
        let tmp = TempDir::new().unwrap();

        // Act
        let result = BootstrapDetector::diagnose(tmp.path(), None, None, None);

        // Assert: LLM と API_SERVER_SECRET の 2 項目が不足
        assert_eq!(result.missing_items.len(), 2);
        assert!(result
            .missing_items
            .iter()
            .any(|m| m.contains("LLM provider")));
        assert!(result
            .missing_items
            .iter()
            .any(|m| m.contains("API_SERVER_SECRET")));
    }

    // ===== FactoryReset Tests =====

    #[test]
    fn test_factory_reset_deletes_data_files() {
        // Arrange: データファイルを作成
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("aiome.db"), "database_content").unwrap();
        std::fs::write(tmp.path().join("SOUL.md"), "# My Soul").unwrap();
        std::fs::create_dir_all(tmp.path().join("artifacts")).unwrap();
        std::fs::write(tmp.path().join("artifacts/test.txt"), "artifact").unwrap();
        std::fs::create_dir_all(tmp.path().join("wasm_storage")).unwrap();

        // Act
        let report = FactoryReset::execute(tmp.path()).unwrap();

        // Assert
        assert!(report.deleted_files.contains(&"aiome.db".to_string()));
        assert!(report.deleted_files.contains(&"SOUL.md".to_string()));
        assert!(report.deleted_dirs.contains(&"artifacts".to_string()));
        assert!(report.deleted_dirs.contains(&"wasm_storage".to_string()));
        assert!(report.errors.is_empty());

        // ファイルが実際に削除されていることを確認
        assert!(!tmp.path().join("aiome.db").exists());
        assert!(!tmp.path().join("SOUL.md").exists());
        assert!(!tmp.path().join("artifacts").exists());
    }

    #[test]
    fn test_factory_reset_preserves_env_and_logs() {
        // Arrange
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".env"), "API_SERVER_SECRET=keep_this").unwrap();
        std::fs::create_dir_all(tmp.path().join("logs")).unwrap();
        std::fs::write(tmp.path().join("logs/audit.log"), "audit_data").unwrap();
        std::fs::write(tmp.path().join("aiome.db"), "database").unwrap();

        // Act
        let report = FactoryReset::execute(tmp.path()).unwrap();

        // Assert: .env と logs は保持されている
        assert!(tmp.path().join(".env").exists());
        assert!(tmp.path().join("logs/audit.log").exists());
        assert!(report.preserved_files.contains(&".env".to_string()));
        assert!(report.preserved_files.contains(&"logs".to_string()));

        // DB は削除されている
        assert!(!tmp.path().join("aiome.db").exists());
    }

    #[test]
    fn test_factory_reset_empty_dir_is_noop() {
        // Arrange: 空のディレクトリ
        let tmp = TempDir::new().unwrap();

        // Act
        let report = FactoryReset::execute(tmp.path()).unwrap();

        // Assert: 何も削除されていないが、エラーもない
        assert!(report.deleted_files.is_empty());
        assert!(report.deleted_dirs.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_factory_reset_nonexistent_dir_returns_error() {
        // Arrange
        let path = Path::new("/tmp/absolutely_nonexistent_aiome_dir_12345");

        // Act
        let result = FactoryReset::execute(path);

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FactoryResetError::DirectoryNotFound(_)));
    }

    #[test]
    #[serial]
    fn test_factory_reset_then_bootstrap_detects_setup() {
        // Arrange: 全 env をクリアしてから設定済み状態を作り、Factory Reset を実行
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("aiome.db"), "database").unwrap();
        std::fs::write(tmp.path().join("SOUL.md"), "# Soul").unwrap();

        // LLM が設定されていない状態で DB を消すと Setup モードになるはず

        // Act: Factory Reset
        let _report = FactoryReset::execute(tmp.path()).unwrap();

        // Assert: Reset 後は Setup モードに戻る
        let diagnosis = BootstrapDetector::diagnose(tmp.path(), None, None, None);
        assert_eq!(diagnosis.mode, BootMode::Setup);
        assert!(!diagnosis.db_exists);
        assert!(!diagnosis.soul_exists);
    }
}
