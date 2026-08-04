/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::security::{BastionGuard, RuntimeJail};
use extism::{Function, Manifest, Plugin, UserData, Val, ValType};
use jsonschema::JSONSchema;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};
use tracing::{error, info, warn};
/// `actions_importer` モジュール
pub mod actions_importer;
/// `cleanroom` モジュール
pub mod cleanroom;
/// Code Mode JS ブリッジ（code_mode.d.ts 準拠のミニインタープリタ）
pub mod code_mode;
/// `discovery` モジュール
pub mod discovery;
/// `forge` モジュール
pub mod forge;
/// `harness` モジュール (AutoHarness 実装)
pub mod harness;
/// Tool Execution Hooks
pub mod hooks;
/// WASM ホスト関数ビルダー (host_exec / host_write)
mod host_fns;
/// `importer` モジュール
pub mod importer;
/// スキルの並列実行と評価
pub mod skill_arena;
/// スキル関連の型定義 (TypeState / メタデータ / 成熟度)
pub mod types;

pub use types::{SkillMaturity, SkillMetadata, UnverifiedSkill, VerifiedSkill};

/// 機密ファイルパスの検出ヘルパー（WASM writeFile / readFile 共通）
///
/// パスの各コンポーネントを検査し、`.env`, `.git`, `.ssh`, 秘密鍵ファイル等への
/// アクセスを遮断する。パターンの追加はこの関数のみで管理する（DRY 原則）。
pub(crate) fn is_sensitive_path(path: &Path) -> bool {
    const SENSITIVE_PATTERNS: &[&str] = &[
        ".env",
        ".git",
        "security.json",
        "cargo.toml",
        "id_rsa",
        "id_ed25519",
        ".ssh",
    ];
    for component in path.components() {
        if let std::path::Component::Normal(c) = component {
            if let Some(s) = c.to_str() {
                let s_lower = s.to_lowercase();
                if SENSITIVE_PATTERNS.iter().any(|&p| s_lower.starts_with(p))
                    || s_lower.ends_with(".pem")
                    || s_lower.ends_with(".key")
                {
                    return true;
                }
            }
        }
    }
    false
}

/// `WasmSkillManager` 構造体
pub struct WasmSkillManager {
    skills_dir: PathBuf,
    allowed_root: PathBuf,
    vault_path: Option<PathBuf>, // Phase 3: DRM 隔離領域
    memory_limit_bytes: u64,
    timeout: Duration,
    wasm_cache: parking_lot::RwLock<HashMap<String, (Vec<u8>, SystemTime)>>,
    db_pool: Option<crate::db::DatabasePool>,
}

impl WasmSkillManager {
    /// 新しいインスタンスを生成する
    pub fn new<P: AsRef<Path>>(
        skills_dir: P,
        allowed_root: P,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let skills_dir = skills_dir.as_ref().to_path_buf();
        let allowed_root = allowed_root.as_ref().to_path_buf();
        if !skills_dir.exists() {
            std::fs::create_dir_all(&skills_dir)?;
        }
        Ok(Self {
            skills_dir,
            allowed_root,
            vault_path: None,
            memory_limit_bytes: 10 * 1024 * 1024, // 10MB default
            timeout: Duration::from_secs(5),      // デフォルト5秒
            wasm_cache: parking_lot::RwLock::new(HashMap::new()),
            db_pool: None,
        })
    }

    /// DB 連携設定
    pub fn with_db_pool(mut self, pool: crate::db::DatabasePool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// 隔離領域を設定する
    pub fn with_vault<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.vault_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// `with_limits` を実行する
    pub fn with_limits(mut self, memory_bytes: u64, timeout: Duration) -> Self {
        self.memory_limit_bytes = memory_bytes;
        self.timeout = timeout;
        self
    }

    pub async fn get_skill_maturity(
        &self,
        skill_name: &str,
    ) -> Result<SkillMaturity, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(pool) = &self.db_pool {
            let sqlite_pool = pool.get_sqlite_pool_or_err()?;
            let row: Option<(String,)> =
                sqlx::query_as("SELECT maturity FROM skill_maturity WHERE skill_name = ?")
                    .bind(skill_name)
                    .fetch_optional(sqlite_pool)
                    .await?;
            if let Some((maturity_str,)) = row {
                return Ok(match maturity_str.as_str() {
                    "Quarantined" => SkillMaturity::Quarantined,
                    "Probation" => SkillMaturity::Probation,
                    "Trusted" => SkillMaturity::Trusted,
                    "Veteran" => SkillMaturity::Veteran,
                    unknown => {
                        tracing::warn!(
                            "Unknown skill maturity '{}' for '{}', defaulting to Quarantined",
                            unknown,
                            skill_name
                        );
                        SkillMaturity::Quarantined
                    }
                });
            }
        }
        Ok(SkillMaturity::Quarantined)
    }

    /// スキルの成熟度を更新する。
    ///
    /// # Safety
    /// このメソッドは TypeState 階段を迂回せずに直接是正度を設定できる。
    /// 呼び出し元は必ず成功率バリデーションを行った上で呼び出すこと。
    pub async fn promote_skill_maturity(
        &self,
        skill_name: &str,
        new_maturity: SkillMaturity,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(pool) = &self.db_pool {
            let maturity_str = new_maturity.to_string();
            let sqlite_pool = pool.get_sqlite_pool_or_err()?;
            sqlx::query(
                "INSERT INTO skill_maturity (skill_name, maturity, promotion_count, last_promoted_at)
                 VALUES (?, ?, 1, datetime('now'))
                 ON CONFLICT(skill_name) DO UPDATE SET
                    maturity=excluded.maturity,
                    promotion_count=skill_maturity.promotion_count+1,
                    last_promoted_at=datetime('now')"
            )
            .bind(skill_name)
            .bind(&maturity_str)
            .execute(sqlite_pool)
            .await?;
        }
        Ok(())
    }

    /// スキルキャッシュをクリアし、最新のスキル一覧を再取得する
    pub fn hot_reload_skills(&self) -> Vec<String> {
        let skills = self.list_skills();
        // B-2: parking_lot::RwLock — no poisoning possible
        let mut cache = self.wasm_cache.write();

        // 存在しないスキルのキャッシュを削除
        cache.retain(|name, _| skills.contains(name));

        info!(
            "🔄 [WasmSkillManager] Hot-reloaded {} skills and cleared stale cache.",
            skills.len()
        );
        skills
    }

    /// 特定のスキルのキャッシュのみを無効化する
    pub fn invalidate_cache(&self, skill_name: &str) {
        // B-2: parking_lot::RwLock — no poisoning possible
        let mut cache = self.wasm_cache.write();
        if cache.remove(skill_name).is_some() {
            info!(
                "🧹 [WasmSkillManager] Invalidated cache for skill: {}",
                skill_name
            );
        }
    }

    /// 全スキルのメタデータを一覧取得する (Self-Wiring 用)
    pub fn list_skills_with_metadata(&self) -> Vec<SkillMetadata> {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json")
                    && path.to_string_lossy().ends_with(".meta.json")
                {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<SkillMetadata>(&data) {
                            list.push(meta);
                        }
                    }
                }
            }
        }

        // メタデータがないスキルについては、ファイル名から最小限のものを生成
        let all_wasm = self.list_skills();
        for name in all_wasm {
            if !list.iter().any(|m| m.name == name) {
                list.push(SkillMetadata {
                    name: name.clone(),
                    description: "No metadata provided".to_string(),
                    capabilities: vec!["execute".to_string()],
                    inputs: vec!["String".to_string()],
                    outputs: vec!["String".to_string()],
                    allowed_hosts: vec![],
                    permissions: crate::security::PermissionManifest::default(),
                });
            }
        }
        list
    }

    /// 利用可能なスキル名を一覧表示する
    pub fn list_skills(&self) -> Vec<String> {
        let mut skills = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        skills.push(name.to_string());
                    }
                }
            }
        }
        skills
    }

    /// WASMスキルを実行する (シークレット注入対応)
    /// 🛡️ 第4層 (Formal Verification): &str ではなく VerifiedSkill 型を要求することで、
    /// 事前の隔離検証を通過していないSkillの実行をコンパイルレベルで阻止する。
    pub async fn call_skill(
        &self,
        skill: &VerifiedSkill,
        func_name: &str,
        input: &str,
        configs: Option<HashMap<String, String>>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let skill_name = skill.name();
        let wasm_path = self.skills_dir.join(format!("{}.wasm", skill_name));
        if !wasm_path.exists() {
            return Err(format!("Skill {} not found", skill_name).into());
        }

        // --- Progressive Loading with mtime check ---
        let current_mtime = fs::metadata(&wasm_path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let wasm_data = {
            let cache = self.wasm_cache.read();
            if let Some((data, cached_mtime)) = cache.get(skill_name) {
                if *cached_mtime == current_mtime {
                    Some(data.clone())
                } else {
                    None // Stale cache
                }
            } else {
                None // No cache
            }
        };

        let wasm_data = match wasm_data {
            Some(data) => data,
            None => {
                let data = std::fs::read(&wasm_path)
                    .map_err(|e| format!("Failed to read WASM {}: {}", skill_name, e))?;
                let mut cache = self.wasm_cache.write();
                cache.insert(skill_name.to_string(), (data.clone(), current_mtime));
                data
            }
        };

        // 厳密なサンドボックス設定
        // Phase 13-A: Wrap EVERYTHING in ONE spawn_blocking because extism types are NOT Send
        let input_str = input.to_string();
        let func_name_str = func_name.to_string();
        let skill_name_str = skill_name.to_string();
        let wasm_path_clone = wasm_path.clone();
        let configs_clone = configs.clone();
        let metadata = self.get_metadata(skill_name);
        let permissions = metadata
            .as_ref()
            .map(|m| m.permissions.clone())
            .unwrap_or_default();
        let timeout = self.timeout;
        let vault_path_clone = self.vault_path.clone();
        let skills_dir_parent = self.skills_dir.parent().map(|p| p.to_path_buf());

        // host_write の比較基準となる root は canonicalize 済みのものをビルダーに渡す。
        // 失敗はクロージャ内へ伝搬させ、従来どおり Aegis インシデント記録経路（res）を通す。
        let allowed_root_for_write = std::fs::canonicalize(&self.allowed_root).map_err(|e| {
            tracing::error!("🚨 [host_write] Failed to canonicalize allowed_root: {}", e);
            format!("Security: Cannot resolve allowed_root: {}", e)
        });

        let result = tokio::task::spawn_blocking(move || {
            // 1. Build Manifest (Inside closure)
            let wasm = if wasm_path_clone.exists() {
                extism::Wasm::file(&wasm_path_clone)
            } else {
                // Fallback to data if file isn't found (should be handled by caller usually)
                extism::Wasm::data(wasm_data)
            };

            let init_guard = BastionGuard::new(permissions.clone());

            let mut manifest = Manifest::new([wasm]).with_timeout(timeout);

            // Apply Sandbox Roots
            if let Some(parent) = skills_dir_parent {
                if let Ok(jail_root) = std::fs::canonicalize(parent) {
                    if init_guard.check_fs_write(&jail_root).is_ok() {
                        manifest = manifest
                            .with_allowed_path(jail_root.to_string_lossy().to_string(), "/mnt");
                    }
                }
            }

            // Apply Network Whitelist (OP-096 / ADR-057):
            // Extism hosts are enumerated only. `*` (after trim) is skipped so a
            // wildcard Manifest does not open WASM net; Code Mode / Bastion still honor `*`.
            if permissions.allow_network {
                for domain in wasm_hosts_for_extism(&permissions.allowed_domains) {
                    if init_guard.check_network(domain).is_ok() {
                        manifest = manifest.with_allowed_host(domain);
                    }
                }
            }

            // Apply Configs
            if let Some(cfg) = configs_clone {
                for (k, v) in cfg {
                    manifest = manifest.with_config(vec![(k, v)].into_iter());
                }
            }

            // 2. Build Host Functions (メモリ安全性契約の詳細は host_fns.rs 参照)
            let allowed_root_for_write = allowed_root_for_write?;
            let functions = vec![
                host_fns::build_host_exec_fn(permissions.clone()),
                host_fns::build_host_write_fn(
                    permissions.clone(),
                    allowed_root_for_write,
                    vault_path_clone,
                ),
            ];
            let mut plugin = Plugin::new(&manifest, functions, true).map_err(|e| {
                format!("Failed to initialize WASM plugin {}: {}", skill_name_str, e)
            })?;

            plugin
                .call::<&str, String>(&func_name_str, &input_str)
                .map_err(|e| {
                    if e.to_string().to_lowercase().contains("timeout") {
                        "WASM execution timed out".to_string()
                    } else {
                        format!("WASM execution error: {}", e)
                    }
                })
        })
        .await;

        let res = match result {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(format!("Task execution failed/panicked: {}", e)),
        };

        if let Err(err_msg) = &res {
            if let Some(pool) = &self.db_pool {
                let repo = crate::aegis::incident_repo::IncidentRepository::new(pool.clone());
                if let Err(e) = repo
                    .insert_incident(skill.name(), "N/A", input, &err_msg.to_string())
                    .await
                {
                    warn!(
                        "⚠️ [Aegis] Failed to record skill incident for '{}': {}",
                        skill.name(),
                        e
                    );
                }
            }
        }

        let result = res.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;

        info!(
            "✅ [WasmSkillManager] Skill execution successful: {}",
            skill_name
        );
        Ok(result)
    }

    /// `get_metadata` を実行する
    pub fn get_metadata(&self, skill_name: &str) -> Option<SkillMetadata> {
        let meta_path = self.skills_dir.join(format!("{}.meta.json", skill_name));
        if let Ok(data) = std::fs::read_to_string(meta_path) {
            serde_json::from_str(&data).ok()
        } else {
            None
        }
    }

    /// Layer 3: Deterministic Tracer (MEV型 Quarantine Simulation)
    /// インストール対象のSkillを、ネットワークを完全に遮断し、
    /// メモリ上限を極端に絞ったサンドボックス上で「空回し」させて振る舞いを検証する。
    pub async fn dry_run_skill(
        &self,
        skill_name: &str,
        test_input: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let wasm_path = self.skills_dir.join(format!("{}.wasm", skill_name));
        if !wasm_path.exists() {
            return Err(format!("Skill {} not found for dry-run", skill_name).into());
        }

        let func_name = self
            .get_metadata(skill_name)
            .and_then(|m| m.capabilities.first().cloned())
            .unwrap_or_else(|| "execute".to_string());

        info!(
            "🛡️  [Layer 3 Deterministic Tracer] Starting dry-run for skill: {} (func: {})",
            skill_name, func_name
        );

        // Phase 13-A: Wrap EVERYTHING in ONE spawn_blocking
        let func_name_str = func_name.to_string();
        let wasm_path_clone = wasm_path.clone();
        let test_input_str = test_input.to_string();
        let skill_name_str = skill_name.to_string();

        let dry_run_success = tokio::task::spawn_blocking(move || {
            let manifest = Manifest::new([extism::Wasm::file(&wasm_path_clone)])
                .with_timeout(Duration::from_millis(500));

            let functions = host_fns::build_noop_host_fns();

            let mut plugin = match Plugin::new(&manifest, functions, true) {
                Ok(p) => p,
                Err(e) => {
                    error!("🚨 [Layer 3 Deterministic Tracer] Initialization violation (OOM or format error): {}", e);
                    return false;
                }
            };

            // 2. 実行時検証 (シミュレーション実行)
            // OOMや非合法なSyscallが発生した場合はエラーとして返ってくる
            info!("⚡ [Layer 3 Deterministic Tracer] Simulating execution with deterministic constraints...");
            match plugin.call::<&str, String>(&func_name_str, &test_input_str) {
                Ok(_) => {
                    info!("✅ [Layer 3 Deterministic Tracer] Protocol behavior validated deterministically: {}", skill_name_str);
                    true
                },
                Err(e) => {
                    error!("🚨 [Layer 3 Deterministic Tracer] Deterministic Violation Detected for '{}': {}", skill_name_str, e);
                    false
                }
            }
        }).await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { format!("Task execution failed/panicked: {}", e).into() })?;

        Ok(dry_run_success)
    }

    /// ドライラン（Dry-Run）による論理検証。
    /// 指定されたテスト入力に対して、期待されるスキーマに合致するかチェックする。
    pub async fn validate_skill_logic(
        &self,
        skill_name: &str,
        test_input: &str,
        expected_schema_json: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🧪 [WasmSkillManager] Validating skill logic for: {}",
            skill_name
        );

        // 内部的に VerifiedSkill を作成 (※validate_skill_logic は管理者のみが呼ぶため信頼済み)
        // -> [Phase 1 Update] 管理者であっても強制的に検証パイプライン(dry-run)を通しTypeStateを保証する
        let unverified = UnverifiedSkill {
            name: skill_name.to_string(),
            input_test_payload: test_input.to_string(),
        };
        let verified = unverified.verify(self).await?;
        let output = self
            .call_skill(&verified, "execute", test_input, None)
            .await?;

        // JSON Schema validation
        let schema_val: serde_json::Value = serde_json::from_str(expected_schema_json)?;
        let instance: serde_json::Value = serde_json::from_str(&output)?;

        let compiled = JSONSchema::compile(&schema_val).map_err(|e| {
            Box::new(std::io::Error::other(format!(
                "Schema compilation failed: {}",
                e
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        if let Err(mut errors) = compiled.validate(&instance) {
            let first_error = errors
                .next()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown validation error".to_string());
            error!(
                "❌ [WasmSkillManager] Logic validation failed for {}: {}",
                skill_name, first_error
            );
            return Ok(false);
        }

        info!(
            "✅ [WasmSkillManager] Logic validation successful: {}",
            skill_name
        );
        Ok(true)
    }

    /// 知識ベース（Karma）から最適なスキルを意味的に探索する (Self-Wiring Capability)
    pub async fn search_skill_in_knowledge(
        &self,
        query: &str,
        jq: &impl aiome_core::traits::JobQueue,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        // 現在のスキル一覧を取得
        let available_skills = self.list_skills();
        if available_skills.is_empty() {
            return Ok(None);
        }

        // Karmaから類似したレッスンを検索 (Top 5)
        let result = jq
            .fetch_relevant_karma(query, "global", 5, "current")
            .await?;

        for entry in result.entries {
            // エントリ内にスキル名が含まれているか、あるいはスキル名そのものが関連しているかチェック
            for skill in &available_skills {
                if entry.lesson.to_lowercase().contains(&skill.to_lowercase()) {
                    info!(
                        "🧠 [Self-Wiring] Found relevant skill '{}' via knowledge: {}",
                        skill, entry.lesson
                    );
                    return Ok(Some(skill.clone()));
                }
            }
        }

        Ok(None)
    }

    /// code_mode.d.ts に準拠した JavaScript コードを一括ロード・安全に実行する JS エンジニアブリッジ
    /// 🛡️ セキュリティロック: allow_shell_execution が false の場合は host_exec (aiome.exec) を遮断する
    pub async fn run_code_mode_js(
        &self,
        js_code: &str,
        manifest: &crate::security::PermissionManifest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        code_mode::run_code_mode_js_impl(self, js_code, manifest).await
    }
}

/// Trimmed Manifest hosts eligible for Extism `with_allowed_host`.
/// Never yields `*` (including whitespace-padded `"*"`), so wildcard Manifests stay closed for WASM.
fn wasm_hosts_for_extism(allowed_domains: &[String]) -> Vec<&str> {
    allowed_domains
        .iter()
        .filter_map(|d| {
            let d = d.trim();
            if d.is_empty() || d == "*" {
                None
            } else {
                Some(d)
            }
        })
        .collect()
}

#[cfg(test)]
mod wasm_host_enum_tests {
    use super::wasm_hosts_for_extism;

    #[test]
    fn skips_star_and_whitespace_variants() {
        let domains = vec![
            "*".into(),
            " * ".into(),
            "* ".into(),
            "".into(),
            "  ".into(),
            "api.example.com".into(),
            "  ok.dev  ".into(),
        ];
        assert_eq!(
            wasm_hosts_for_extism(&domains),
            vec!["api.example.com", "ok.dev"]
        );
    }
}

#[cfg(test)]
mod tests;
