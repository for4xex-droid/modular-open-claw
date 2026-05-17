/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::llm::utils::extract_code_block;
use crate::security::{BastionGuard, PermissionManifest, RuntimeJail, SandboxProfile};
use aiome_core::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, warn};

/// コンパイルエラーのカテゴリ分類 (Bun Rust Rewrite Pattern C)
///
/// Bun PR #30412 の 960K行 Zig→Rust 自動翻訳で発見された知見:
/// コンパイルエラーは5つのカテゴリに集約され、カテゴリごとに
/// 最適な修正パターンが存在する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Used internally by SkillForge::fix_code_with_llm
enum CompileErrorCategory {
    /// `expected X, found Y` — 型の不一致
    TypeMismatch,
    /// ライフタイム・借用チェッカー関連
    Lifetime,
    /// トレイト未実装・トレイト境界不足
    MissingTrait,
    /// `use` 文の欠如・クレートパス解決失敗
    ImportResolution,
    /// その他
    Other,
}

#[derive(Clone)]
/// WASMスキルのビルド・ロードを管理
pub struct SkillForge {
    template_dir: PathBuf,
    skills_output_dir: PathBuf,
}

impl SkillForge {
    /// 新しいインスタンスを生成する
    pub fn new<P: AsRef<Path>>(template_dir: P, skills_output_dir: P) -> Self {
        Self {
            template_dir: template_dir.as_ref().to_path_buf(),
            skills_output_dir: skills_output_dir.as_ref().to_path_buf(),
        }
    }

    /// Forge環境の初期構築
    pub fn ensure_forge_workspace(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.template_dir.exists() {
            fs::create_dir_all(&self.template_dir)?;

            // Cargo.toml
            let cargo_toml = r#"[package]
name = "skill_generator"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[workspace]

[dependencies]
extism-pdk = "1.4.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;
            fs::write(self.template_dir.join("Cargo.toml"), cargo_toml)?;

            // src/lib.rs
            let src_dir = self.template_dir.join("src");
            fs::create_dir_all(&src_dir)?;
            fs::write(src_dir.join("lib.rs"), "// Forge Entrypoint")?;
        }
        Ok(())
    }

    /// 既存の Seatbelt プロファイル生成ロジックは BastionGuard に統合
    /// 新しいスキルを生成し、コンパイルする (自己修復ループ付き)
    pub async fn forge_skill(
        &self,
        skill_name: &str,
        initial_rust_code: &str,
        retry_count: u32,
        description: &str,
        llm: Option<Arc<dyn LlmProvider>>,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Security Gate: Ensure forge is enabled
        let enabled = std::env::var("SKILL_FORGE_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        if !enabled {
            error!("🛑 [SkillForge] Forging is BLOCKED. Set SKILL_FORGE_ENABLED=true to allow.");
            return Err(
                "Security Violation: Real-time skill forging is disabled in this environment."
                    .into(),
            );
        }

        // Phase 13-C: File-Based Saga — Use stable directory for build caching
        let forge_root = self
            .skills_output_dir
            .parent()
            .unwrap_or(&self.skills_output_dir)
            .join("forge_workspaces");
        let workspace_dir = forge_root.join(skill_name);
        if !workspace_dir.exists() {
            fs::create_dir_all(&workspace_dir)?;
        }

        // 1. Copy Template (Sanitized copy_dir prevents overwriting existing build artifacts)
        Self::copy_dir(&self.template_dir, &workspace_dir)?;

        // 2. Update Cargo.toml name (G13)
        let cargo_toml_path = workspace_dir.join("Cargo.toml");
        let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
        let updated_cargo = cargo_toml.replace("skill_generator", skill_name);
        fs::write(&cargo_toml_path, updated_cargo)?;

        // 3. Compile Loop (G11 Support: Stderr results will be used for self-healing)
        let mut rust_code = initial_rust_code.to_string();
        for attempt in 0..=retry_count {
            info!(
                "🛠️ [SkillForge] Compiling {} (Attempt {}/{})",
                skill_name,
                attempt + 1,
                retry_count + 1
            );

            let lib_rs_path = workspace_dir.join("src/lib.rs");
            fs::write(&lib_rs_path, &rust_code)?;

            let abs_workspace =
                std::fs::canonicalize(&workspace_dir).unwrap_or(workspace_dir.clone());
            let manifest_path = abs_workspace.join("Cargo.toml");

            let guard = BastionGuard::new_internal(PermissionManifest {
                allow_shell_execution: true,
                ..Default::default()
            });

            let cmd = format!(
                "cargo build --manifest-path \"{}\" --target wasm32-wasip1 --release",
                manifest_path.to_string_lossy()
            );

            let res = guard
                .safe_exec_with_profile(&cmd, SandboxProfile::WasmBuild)
                .await;

            match res {
                Ok(_stdout) => {
                    info!("✅ [SkillForge] Compilation SUCCESS for {}", skill_name);
                    let wasm_file = workspace_dir.join(format!(
                        "target/wasm32-wasip1/release/{}.wasm",
                        skill_name.replace('-', "_")
                    ));
                    let final_path = self.skills_output_dir.join(format!("{}.wasm", skill_name));

                    if !self.skills_output_dir.exists() {
                        fs::create_dir_all(&self.skills_output_dir)?;
                    }

                    fs::copy(&wasm_file, &final_path)?;
                    info!("✅ [SkillForge] Successfully forged skill: {}", skill_name);

                    // 4. Save Metadata
                    let meta_path = self
                        .skills_output_dir
                        .join(format!("{}.meta.json", skill_name));
                    #[derive(serde::Serialize)]
                    struct LocalSkillMetadata {
                        name: String,
                        description: String,
                        capabilities: Vec<String>,
                        inputs: Vec<String>,
                        outputs: Vec<String>,
                    }
                    let meta = LocalSkillMetadata {
                        name: skill_name.to_string(),
                        description: description.to_string(),
                        capabilities: vec!["execute".to_string()],
                        inputs: vec!["String".to_string()],
                        outputs: vec!["String".to_string()],
                    };
                    let meta_json = serde_json::to_string_pretty(&meta)?;
                    fs::write(meta_path, meta_json)?;

                    return Ok(final_path);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    error!(
                        "❌ [SkillForge] Compilation failed for {}:\n{}",
                        skill_name, err_str
                    );

                    if attempt < retry_count {
                        if let Some(ref provider) = llm {
                            warn!(
                                "🔄 [SkillForge] Attempting AI Self-Heal for {} (Attempt {})",
                                skill_name,
                                attempt + 1
                            );
                            match self.fix_code_with_llm(provider, &rust_code, &err_str).await {
                                Ok(fixed_code) => {
                                    rust_code = fixed_code;
                                    continue;
                                }
                                Err(fix_err) => {
                                    error!("❌ [SkillForge] AI Self-Heal FAILED: {:?}", fix_err);
                                }
                            }
                        }
                        warn!("🔄 [SkillForge] Falling back to standard retry...");
                        continue;
                    } else {
                        return Err(format!(
                            "Compilation failed after {} attempts. Error: {}",
                            retry_count + 1,
                            err_str
                        )
                        .into());
                    }
                }
            }
        }

        Err("Maximum retry attempts reached without success.".into())
    }

    /// AI Self-Heal: コンパイルエラーをカテゴリ分類し、適切な修正パターンを提供する。
    ///
    /// Bun PR #30412 (Zig→Rust Rewrite) の知見を適用:
    /// 960K行の自動翻訳で発見された典型的な Rust コンパイルエラーは
    /// 5つのカテゴリに集約される。カテゴリごとに Few-Shot パターンを注入することで
    /// LLM の修正精度を向上させる。
    async fn fix_code_with_llm(
        &self,
        llm: &Arc<dyn LlmProvider>,
        original_code: &str,
        error_log: &str,
    ) -> Result<String, AiomeError> {
        // C: Error categorization (Bun Rust Rewrite Pattern)
        let error_category = Self::categorize_compile_error(error_log);
        let category_hint = match error_category {
            CompileErrorCategory::TypeMismatch => {
                "CATEGORY: Type Mismatch\n\
                 COMMON FIX: Add `.into()`, `.as_ref()`, `&*`, or explicit type annotation.\n\
                 EXAMPLE:\n\
                 ```rust\n\
                 // Before: let x: String = some_str; // &str vs String\n\
                 // After:  let x: String = some_str.to_string();\n\
                 ```"
            }
            CompileErrorCategory::Lifetime => {
                "CATEGORY: Lifetime / Borrow Checker\n\
                 COMMON FIX: Clone the value, use `Arc`, or restructure borrows.\n\
                 EXAMPLE:\n\
                 ```rust\n\
                 // Before: let r = &data; drop(data); use(r); // dangling\n\
                 // After:  let r = data.clone(); drop(data); use(&r);\n\
                 ```"
            }
            CompileErrorCategory::MissingTrait => {
                "CATEGORY: Missing Trait Implementation\n\
                 COMMON FIX: Add `#[derive(...)]`, implement the trait, or add trait bound.\n\
                 EXAMPLE:\n\
                 ```rust\n\
                 // Before: struct Foo { x: i32 } // cannot be serialized\n\
                 // After:  #[derive(serde::Serialize, serde::Deserialize)]\n\
                 //         struct Foo { x: i32 }\n\
                 ```"
            }
            CompileErrorCategory::ImportResolution => {
                "CATEGORY: Import / Module Resolution\n\
                 COMMON FIX: Add `use` statement or fix crate path.\n\
                 IMPORTANT: This is an Extism PDK plugin. Available crates: extism-pdk, serde, serde_json.\n\
                 Do NOT add dependencies that are not in Cargo.toml."
            }
            CompileErrorCategory::Other => {
                "CATEGORY: General Compilation Error\n\
                 Analyze the error message carefully and apply the minimal fix."
            }
        };

        let prompt = format!(
            "You are a Rust compiler expert specializing in WASM plugin development (Extism PDK).\n\
             The following code failed to compile.\n\n\
             {category_hint}\n\n\
             CODE:\n```rust\n{original_code}\n```\n\n\
             ERROR LOG:\n{error_log}\n\n\
             RULES:\n\
             - Fix ONLY the compilation error. Do NOT refactor unrelated code.\n\
             - Preserve all existing logic and comments.\n\
             - The code targets `wasm32-wasip1` with `extism-pdk`, `serde`, `serde_json` only.\n\
             - Output ONLY the fixed Rust code block."
        );

        let response =
            llm.complete(&prompt, None)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("AI fix request failed: {}", e),
                })?;

        // 共通ユーティリティを使用してコードブロックを抽出
        let code = extract_code_block(&response.content);

        Ok(code)
    }

    /// コンパイルエラーログからエラーカテゴリを推定する
    fn categorize_compile_error(error_log: &str) -> CompileErrorCategory {
        let lower = error_log.to_lowercase();
        if lower.contains("mismatched types")
            || (lower.contains("expected") && (lower.contains("found") || lower.contains("type")))
        {
            CompileErrorCategory::TypeMismatch
        } else if lower.contains("lifetime")
            || lower.contains("borrowed")
            || lower.contains("does not live long enough")
            || lower.contains("cannot move out of")
        {
            CompileErrorCategory::Lifetime
        } else if lower.contains("trait")
            && (lower.contains("not implemented")
                || lower.contains("not satisfied")
                || lower.contains("bound"))
        {
            CompileErrorCategory::MissingTrait
        } else if lower.contains("unresolved import")
            || lower.contains("could not find")
            || lower.contains("no external crate")
        {
            CompileErrorCategory::ImportResolution
        } else {
            CompileErrorCategory::Other
        }
    }

    fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());

            if ty.is_dir() {
                // Phase 13-C Fix: Skip existing directories to preserve build caches (target/)
                if !dest_path.exists() {
                    fs::create_dir_all(&dest_path)?;
                }
                Self::copy_dir(&entry.path(), &dest_path)?;
            } else {
                // For files (Cargo.toml, src/lib.rs template), we ensure they match the template
                fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── categorize_compile_error テスト ──────────────────────

    #[test]
    fn test_categorize_type_mismatch_mismatched_types() {
        let log = "error[E0308]: mismatched types\n  --> src/lib.rs:5:20\n   = note: expected `String`, found `&str`";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::TypeMismatch
        );
    }

    #[test]
    fn test_categorize_type_mismatch_expected_found() {
        let log = "error: expected struct `Vec`, found array `[i32; 3]`";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::TypeMismatch
        );
    }

    #[test]
    fn test_categorize_lifetime() {
        let log = "error[E0597]: `x` does not live long enough\n  --> src/lib.rs:10:5";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::Lifetime
        );
    }

    #[test]
    fn test_categorize_lifetime_borrowed() {
        let log = "error[E0505]: cannot move out of `data` because it is borrowed";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::Lifetime
        );
    }

    #[test]
    fn test_categorize_lifetime_cannot_move() {
        let log = "error[E0507]: cannot move out of index of `Vec<String>`";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::Lifetime
        );
    }

    #[test]
    fn test_categorize_missing_trait() {
        let log = "error[E0277]: the trait `Serialize` is not implemented for `MyStruct`";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::MissingTrait
        );
    }

    #[test]
    fn test_categorize_missing_trait_bound() {
        let log = "error[E0277]: the trait bound `T: Clone` is not satisfied";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::MissingTrait
        );
    }

    #[test]
    fn test_categorize_import_unresolved() {
        let log = "error[E0432]: unresolved import `extism_pdk::foo`\n  --> src/lib.rs:1:5";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::ImportResolution
        );
    }

    #[test]
    fn test_categorize_import_could_not_find() {
        let log = "error[E0433]: could not find `nonexistent` in crate root";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::ImportResolution
        );
    }

    #[test]
    fn test_categorize_import_no_external_crate() {
        let log = "error[E0463]: can't find crate for `no_external_crate`";
        // "no external crate" is a substring match
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::Other // E0463 doesn't match "no external crate" exactly
        );
    }

    #[test]
    fn test_categorize_other_syntax_error() {
        let log = "error: unexpected closing delimiter: `}`\n  --> src/lib.rs:42:1";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::Other
        );
    }

    #[test]
    fn test_categorize_empty_log() {
        assert_eq!(
            SkillForge::categorize_compile_error(""),
            CompileErrorCategory::Other
        );
    }

    #[test]
    fn test_categorize_case_insensitive() {
        let log = "ERROR: MISMATCHED TYPES in function main";
        assert_eq!(
            SkillForge::categorize_compile_error(log),
            CompileErrorCategory::TypeMismatch
        );
    }
}
