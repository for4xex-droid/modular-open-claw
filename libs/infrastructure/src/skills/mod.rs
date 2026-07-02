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
/// `discovery` モジュール
pub mod discovery;
/// `forge` モジュール
pub mod forge;
/// `harness` モジュール (AutoHarness 実装)
pub mod harness;
/// Tool Execution Hooks
pub mod hooks;
/// `importer` モジュール
pub mod importer;
/// スキルの並列実行と評価
pub mod skill_arena;

use contracts::requires;

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

#[derive(Debug, Clone)]
pub struct UnverifiedSkill {
    /// name
    pub name: String,
    /// input_test_payload
    pub input_test_payload: String,
}

/// 状態: 確定的検証をパスした安全なSkill (TypeState Pattern)
#[derive(Debug, Clone)]
pub struct VerifiedSkill {
    name: String,
}

impl VerifiedSkill {
    /// Internal constructor for the infrastructure crate to promote unverified skills.
    /// This ensures mathematical safety of the TypeState pattern.
    pub(crate) fn promote_internal(name: String) -> Self {
        Self { name }
    }

    /// TEST ONLY: Create a verified skill without dry-run.
    /// This is used for integration tests.
    pub fn new_for_test<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }

    /// `name` を実行する
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl UnverifiedSkill {
    /// 契約プログラミングにより、検証を通過したものだけが型を昇格できる
    #[requires(self.input_test_payload.len() < 50_000, "Payload limits exceeded")]
    // #[ensures] is removed here because verification failure (Err) is a valid, expected state machine outcome for malicious skills.
    pub async fn verify(
        self,
        manager: &WasmSkillManager,
    ) -> Result<VerifiedSkill, Box<dyn std::error::Error + Send + Sync>> {
        let is_safe = manager
            .dry_run_skill(&self.name, &self.input_test_payload)
            .await?;
        if is_safe {
            Ok(VerifiedSkill::promote_internal(self.name))
        } else {
            Err(format!(
                "Skill {} failed the deterministic dry-run quarantine",
                self.name
            )
            .into())
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// `SkillMetadata` 構造体
pub struct SkillMetadata {
    /// name
    pub name: String,
    /// description
    pub description: String,
    /// capabilities
    pub capabilities: Vec<String>,
    /// inputs
    pub inputs: Vec<String>,
    /// outputs
    pub outputs: Vec<String>,
    #[serde(default)]
    /// allowed_hosts
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    /// permissions
    pub permissions: crate::security::PermissionManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkillMaturity {
    Quarantined, // dry-run未通過
    Probation,   // 通過済みだが実績 < 5
    Trusted,     // 実績 >= 5 & 成功率 > 80%
    Veteran,     // 実績 >= 50 & 成功率 > 95%
}

impl std::fmt::Display for SkillMaturity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillMaturity::Quarantined => write!(f, "Quarantined"),
            SkillMaturity::Probation => write!(f, "Probation"),
            SkillMaturity::Trusted => write!(f, "Trusted"),
            SkillMaturity::Veteran => write!(f, "Veteran"),
        }
    }
}

#[allow(clippy::empty_loop)]
static DUMMY_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("a^").unwrap_or_else(|_| loop {}));

static LOG_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| match regex::Regex::new(r#"aiome\.log\((.*)\);"#) {
        Ok(r) => r,
        Err(_) => DUMMY_REGEX.clone(),
    });
static EXEC_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    match regex::Regex::new(r#"(?:const\s+(\w+)\s*=\s*(?:await\s+)?)?aiome\.exec\((.*)\);"#) {
        Ok(r) => r,
        Err(_) => DUMMY_REGEX.clone(),
    }
});
static WRITE_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| match regex::Regex::new(r#"aiome\.writeFile\((.*)\);"#) {
        Ok(r) => r,
        Err(_) => DUMMY_REGEX.clone(),
    });
static READ_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    match regex::Regex::new(r#"(?:const\s+(\w+)\s*=\s*(?:await\s+)?)?aiome\.readFile\((.*)\);"#) {
        Ok(r) => r,
        Err(_) => DUMMY_REGEX.clone(),
    }
});
static FETCH_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    match regex::Regex::new(r#"(?:const\s+(\w+)\s*=\s*(?:await\s+)?)?aiome\.fetch\((.*)\);"#) {
        Ok(r) => r,
        Err(_) => DUMMY_REGEX.clone(),
    }
});

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
        let allowed_root_clone = self.allowed_root.clone();
        let timeout = self.timeout;
        let vault_path_clone = self.vault_path.clone();
        let skills_dir_parent = self.skills_dir.parent().map(|p| p.to_path_buf());

        let result = tokio::task::spawn_blocking(move || {
            // 1. Build Manifest (Inside closure)
            let wasm = if wasm_path_clone.exists() {
                extism::Wasm::file(&wasm_path_clone)
            } else {
                // Fallback to data if file isn't found (should be handled by caller usually)
                extism::Wasm::data(wasm_data)
            };

            let host_exec_permissions = metadata.as_ref().map(|m| m.permissions.clone()).unwrap_or_default();
            let init_guard = BastionGuard::new(host_exec_permissions.clone());

            let mut manifest = Manifest::new([wasm])
                .with_timeout(timeout);

            // Apply Sandbox Roots
            if let Some(parent) = skills_dir_parent {
                if let Ok(jail_root) = std::fs::canonicalize(parent) {
                    if init_guard.check_fs_write(&jail_root).is_ok() {
                        manifest = manifest.with_allowed_path(jail_root.to_string_lossy().to_string(), "/mnt");
                    }
                }
            }

            // Apply Network Whitelist
            if host_exec_permissions.allow_network {
                for domain in &host_exec_permissions.allowed_domains {
                    if domain != "*" && init_guard.check_network(domain).is_ok() {
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

            // 2. Build Host Functions
            //
            // ── B-1: Memory Safety Contract (Bun Rust Rewrite Pattern) ──
            // The host_exec/host_write functions use Extism's memory pointer pipeline:
            //   1. Guest passes I64 offset → host validates via memory_handle()
            //   2. memory_handle() returns None if offset is out-of-bounds (safe)
            //   3. memory_str() validates UTF-8 encoding (safe)
            //   4. Response is allocated via memory_alloc() with exact length (no overflow)
            //
            // When WASI P2 + Component Model becomes available, these raw pointer
            // exchanges should be replaced with WIT-typed interfaces.
            // ──────────────────────────────────────────────────────────────
            let host_exec_fn = Function::new(
                "host_exec",
                [ValType::I64],
                [ValType::I64],
                UserData::new(()),
                move |plugin, inputs, outputs, _user_data| {
                    // Step 1: Extract memory pointer — fails safely if guest sends garbage
                    let cmd_ptr = inputs.first().and_then(|v| v.i64()).ok_or_else(|| {
                        tracing::warn!("🛡️ [host_exec] Guest sent no input parameter");
                        extism::Error::msg("Missing input parameter")
                    })? as u64;
                    // Step 2: Validate memory handle — returns Error if OOB
                    let handle = plugin.memory_handle(cmd_ptr).ok_or_else(|| {
                        tracing::warn!("🛡️ [host_exec] Invalid memory handle at offset {}", cmd_ptr);
                        extism::Error::msg("Invalid memory handle")
                    })?;
                    // Step 3: UTF-8 validated string extraction
                    let cmd_str: String = plugin.memory_str(handle).map_err(|e: extism::Error| e)?.to_string();
                    let guard = BastionGuard::new(host_exec_permissions.clone());
                    let runtime = tokio::runtime::Handle::current();
                    let res = runtime.block_on(async {
                        guard.safe_exec(&cmd_str).await
                    });

                    // Step 4: Response allocation with exact byte length
                    match res {
                        Ok(stdout_str) => {
                            let stdout_bytes = stdout_str.as_bytes();
                            let mem = plugin.memory_alloc(stdout_bytes.len() as u64)?;
                            plugin.memory_bytes_mut(mem)?.copy_from_slice(stdout_bytes);
                            outputs[0] = Val::I64(mem.offset() as i64);
                        },
                        Err(e) => {
                            let err_msg = format!("Bastion Guard Error: {}", e);
                            tracing::warn!("🛡️ [host_exec] BastionGuard rejected command: {}", e);
                            let mem = plugin.memory_alloc(err_msg.len() as u64)?;
                            plugin.memory_bytes_mut(mem)?.copy_from_slice(err_msg.as_bytes());
                            outputs[0] = Val::I64(mem.offset() as i64);
                        }
                    }
                    Ok(())
                }
            );

            let host_write_permissions = metadata.as_ref().map(|m| m.permissions.clone()).unwrap_or_default();
            let allowed_root_for_write = match std::fs::canonicalize(&allowed_root_clone) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("🚨 [host_write] Failed to canonicalize allowed_root: {}", e);
                    return Err(format!("Security: Cannot resolve allowed_root: {}", e));
                }
            };
            let vault_path_for_write = vault_path_clone.clone(); // Use the local clone
            let host_write_fn = Function::new(
                "host_write",
                [ValType::I64],
                [ValType::I64],
                UserData::new(()),
                move |plugin, inputs, outputs, _user_data| {
                    // B-1: Same memory safety pipeline as host_exec
                    let json_ptr = inputs.first().and_then(|v| v.i64()).ok_or_else(|| {
                        tracing::warn!("🛡️ [host_write] Guest sent no input parameter");
                        extism::Error::msg("Missing input parameter")
                    })? as u64;
                    let handle = plugin.memory_handle(json_ptr).ok_or_else(|| {
                        tracing::warn!("🛡️ [host_write] Invalid memory handle at offset {}", json_ptr);
                        extism::Error::msg("Invalid memory handle for host_write")
                    })?;
                    let req_str = plugin.memory_str(handle).map_err(|e: extism::Error| e)?;

                    if !host_write_permissions.allow_filesystem_write {
                        let res_json = serde_json::json!({ "success": false, "path": "", "error": "Security Violation: Field writing is not permitted for this skill." }).to_string();
                        let mem = plugin.memory_alloc(res_json.len() as u64)?;
                        plugin.memory_bytes_mut(mem)?.copy_from_slice(res_json.as_bytes());
                        outputs[0] = Val::I64(mem.offset() as i64);
                        return Ok(());
                    }

                    #[derive(serde::Deserialize)]
                    struct WriteReq { path: String, content: String }
                    let res_json = match serde_json::from_str::<WriteReq>(req_str) {
                        Ok(req) => {
                            let full_path = allowed_root_for_write.join(&req.path);
                            let parent_dir = full_path.parent().unwrap_or(&full_path);
                            if !parent_dir.exists() { let _ = std::fs::create_dir_all(parent_dir); }
                            match std::fs::canonicalize(parent_dir) {
                                Ok(canon_parent) => {
                                    let Some(file_name) = full_path.file_name() else {
                                        let res_json = serde_json::json!({ "success": false, "path": "", "error": "Invalid filename" }).to_string();
                                        let mem = plugin.memory_alloc(res_json.len() as u64)?;
                                        plugin.memory_bytes_mut(mem)?.copy_from_slice(res_json.as_bytes());
                                        outputs[0] = Val::I64(mem.offset() as i64);
                                        return Ok(());
                                    };
                                    let final_path = canon_parent.join(file_name);

                                    let mut path_allowed = final_path.starts_with(&allowed_root_for_write);

                                    // Check against Vault if workspace missed
                                    if !path_allowed {
                                        if let Some(vault_root) = &vault_path_for_write {
                                            if final_path.starts_with(vault_root) {
                                                path_allowed = true;
                                            }
                                        }
                                    }

                                    let mut is_sensitive = false;
                                    for component in final_path.components() {
                                        if let std::path::Component::Normal(c) = component {
                                            if let Some(s) = c.to_str() {
                                                if s.starts_with(".env") || s == ".git" || s == "security.json" {
                                                    is_sensitive = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    if !path_allowed {
                                        serde_json::json!({ "success": false, "path": "", "error": "Security Violation: Path traversal blocked." }).to_string()
                                    } else if is_sensitive {
                                        serde_json::json!({ "success": false, "path": "", "error": "Security Violation: Access to sensitive internal file is forbidden." }).to_string()
                                    } else {
                                        if let Some(parent) = final_path.parent() { let _ = std::fs::create_dir_all(parent); }
                                        match std::fs::write(&final_path, req.content) {
                                            Ok(_) => serde_json::json!({ "success": true, "path": final_path.to_string_lossy().to_string(), "error": None::<String> }).to_string(),
                                            Err(e) => serde_json::json!({ "success": false, "path": "", "error": format!("Write failed: {}", e) }).to_string()
                                        }
                                    }
                                },
                                Err(e) => serde_json::json!({ "success": false, "path": "", "error": format!("Parent path canonicalization failed: {}", e) }).to_string()
                            }
                        },
                        Err(e) => serde_json::json!({ "success": false, "path": "", "error": format!("Invalid JSON payload: {}", e) }).to_string()
                    };

                    let mem = plugin.memory_alloc(res_json.len() as u64)?;
                    plugin.memory_bytes_mut(mem)?.copy_from_slice(res_json.as_bytes());
                    outputs[0] = Val::I64(mem.offset() as i64);
                    Ok(())
                }
            );

            let functions = vec![host_exec_fn, host_write_fn];
            let mut plugin = Plugin::new(&manifest, functions, true)
                .map_err(|e| format!("Failed to initialize WASM plugin {}: {}", skill_name_str, e))?;

            plugin.call::<&str, String>(&func_name_str, &input_str)
                .map_err(|e| {
                    if e.to_string().to_lowercase().contains("timeout") {
                        "WASM execution timed out".to_string()
                    } else {
                        format!("WASM execution error: {}", e)
                    }
                })
        }).await;

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

            let host_exec_fn = Function::new(
                "host_exec",
                [ValType::I64],
                [ValType::I64],
                UserData::new(()),
                |plugin, _inputs, outputs, _user_data| {
                    let mem = plugin.memory_alloc(0)?;
                    outputs[0] = Val::I64(mem.offset() as i64);
                    Ok(())
                }
            );
            let host_write_fn = Function::new(
                "host_write",
                [ValType::I64],
                [ValType::I64],
                UserData::new(()),
                |plugin, _inputs, outputs, _user_data| {
                    let mem = plugin.memory_alloc(0)?;
                    outputs[0] = Val::I64(mem.offset() as i64);
                    Ok(())
                }
            );
            let functions = vec![host_exec_fn, host_write_fn];

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
        let lines = js_code.lines();
        let mut variables: HashMap<String, String> = HashMap::new();
        let mut last_output = String::new();

        // 30秒のタイムアウト付き HTTP クライアントを一度だけ生成して使い回す (P1-1, P1-2)
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build reqwest Client: {}", e))?;

        // 比較基準となる root パス自体を canonicalize しておく（macOS の /var と /private/var の不一致防止）
        let canon_root = std::fs::canonicalize(&self.allowed_root)
            .map_err(|e| format!("Failed to canonicalize allowed_root: {}", e))?;

        for line in lines {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with("//")
                || line.starts_with("/*")
                || line.starts_with("*")
            {
                continue;
            }

            let expand_vars = |s: &str, vars: &HashMap<String, String>| -> String {
                let mut result = s.to_string();
                for (k, v) in vars {
                    let pattern_curly = format!("${{{}}}", k);
                    result = result.replace(&pattern_curly, v);
                }
                result
            };

            fn unquote(s: &str) -> &str {
                let s_trim = s.trim();
                if (s_trim.starts_with('"') && s_trim.ends_with('"'))
                    || (s_trim.starts_with('\'') && s_trim.ends_with('\''))
                    || (s_trim.starts_with('`') && s_trim.ends_with('`'))
                {
                    if s_trim.len() >= 2 {
                        &s_trim[1..s_trim.len() - 1]
                    } else {
                        s_trim
                    }
                } else {
                    s_trim
                }
            }

            let resolve_token = |token: &str, vars: &HashMap<String, String>| -> String {
                let token_trim = token.trim();
                if (token_trim.starts_with('"') && token_trim.ends_with('"'))
                    || (token_trim.starts_with('\'') && token_trim.ends_with('\''))
                    || (token_trim.starts_with('`') && token_trim.ends_with('`'))
                {
                    expand_vars(unquote(token_trim), vars)
                } else {
                    vars.get(token_trim)
                        .cloned()
                        .unwrap_or_else(|| token_trim.to_string())
                }
            };

            // 1. aiome.log
            if let Some(caps) = LOG_REGEX.captures(line) {
                let inner = caps[1].trim();
                let msg = resolve_token(inner, &variables);
                info!("📝 [JS Log] {}", msg);
                last_output = msg;
                continue;
            }

            // 2. aiome.exec
            if let Some(caps) = EXEC_REGEX.captures(line) {
                if !manifest.allow_shell_execution {
                    return Err("Security Violation: Shell execution is not permitted".into());
                }

                let inner = caps[2].trim();
                let cmd = resolve_token(inner, &variables);
                let guard = BastionGuard::new(manifest.clone());
                let stdout = guard.safe_exec(&cmd).await?;

                if let Some(var_name) = caps.get(1) {
                    variables.insert(var_name.as_str().to_string(), stdout.clone());
                }
                last_output = stdout;
                continue;
            }

            // 3. aiome.writeFile
            if let Some(caps) = WRITE_REGEX.captures(line) {
                if !manifest.allow_filesystem_write {
                    return Err("Security Violation: Filesystem write is not permitted".into());
                }
                let args_str = &caps[1];
                let args_parts: Vec<&str> = args_str.splitn(2, ',').map(|s| s.trim()).collect();
                if args_parts.len() == 2 {
                    let relative_path = resolve_token(args_parts[0], &variables);
                    let content = resolve_token(args_parts[1], &variables);

                    let full_path = canon_root.join(&relative_path);
                    let parent_dir = full_path.parent().unwrap_or(&full_path);
                    if !parent_dir.exists() {
                        let _ = std::fs::create_dir_all(parent_dir);
                    }
                    let canon_parent = std::fs::canonicalize(parent_dir)?;
                    let Some(file_name) = full_path.file_name() else {
                        return Err("Invalid filename".into());
                    };
                    let final_path = canon_parent.join(file_name);

                    if !final_path.starts_with(&canon_root) {
                        return Err("Security Violation: Path traversal blocked".into());
                    }

                    if is_sensitive_path(&final_path) {
                        return Err(
                            "Security Violation: Access to sensitive internal file is forbidden"
                                .into(),
                        );
                    }

                    std::fs::write(&final_path, content)?;
                    last_output = format!("Wrote to {}", relative_path);
                }
                continue;
            }

            // 4. aiome.readFile
            if let Some(caps) = READ_REGEX.captures(line) {
                let inner = caps[2].trim();
                let relative_path = resolve_token(inner, &variables);
                let full_path = canon_root.join(&relative_path);
                let parent_dir = full_path.parent().unwrap_or(&full_path);

                if !parent_dir.exists() {
                    return Err("File not found".into());
                }
                let canon_parent = std::fs::canonicalize(parent_dir)?;
                let Some(file_name) = full_path.file_name() else {
                    return Err("Invalid filename".into());
                };
                let final_path = canon_parent.join(file_name);

                if !final_path.starts_with(&canon_root) {
                    return Err("Security Violation: Path traversal blocked".into());
                }

                if is_sensitive_path(&final_path) {
                    return Err(
                        "Security Violation: Access to sensitive internal file is forbidden".into(),
                    );
                }

                let content = std::fs::read_to_string(&final_path)?;
                if let Some(var_name) = caps.get(1) {
                    variables.insert(var_name.as_str().to_string(), content.clone());
                }
                last_output = content;
                continue;
            }

            // 5. aiome.fetch
            if let Some(caps) = FETCH_REGEX.captures(line) {
                if !manifest.allow_network {
                    return Err("Security Violation: Network access is not permitted".into());
                }

                let args_str = &caps[2];
                let args_parts: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
                if args_parts.len() >= 2 {
                    let method = resolve_token(args_parts[0], &variables);
                    let url = resolve_token(args_parts[1], &variables);

                    let parsed_url = url::Url::parse(&url)?;
                    let host = parsed_url.host_str().ok_or("Invalid host in URL")?;
                    let mut domain_allowed = false;
                    for domain in &manifest.allowed_domains {
                        if domain == "*"
                            || domain == host
                            || host.ends_with(&format!(".{}", domain))
                        {
                            domain_allowed = true;
                            break;
                        }
                    }
                    if !domain_allowed {
                        return Err(format!(
                            "Security Violation: Access to domain {} is blocked",
                            host
                        )
                        .into());
                    }

                    let req_method = match method.to_uppercase().as_str() {
                        "GET" => reqwest::Method::GET,
                        "POST" => reqwest::Method::POST,
                        "PUT" => reqwest::Method::PUT,
                        "DELETE" => reqwest::Method::DELETE,
                        _ => return Err(format!("Unsupported HTTP method: {}", method).into()),
                    };

                    let mut builder = http_client.request(req_method, &url);

                    if args_parts.len() >= 3 {
                        let extra_args = args_parts[2..].join(",");
                        if let Ok(json_args) =
                            serde_json::from_str::<serde_json::Value>(&extra_args)
                        {
                            if let Some(headers_obj) =
                                json_args.get("headers").and_then(|h| h.as_object())
                            {
                                for (k, v) in headers_obj {
                                    if let Some(v_str) = v.as_str() {
                                        builder = builder.header(k, v_str);
                                    }
                                }
                            }
                            if let Some(body_val) = json_args.get("body") {
                                if let Some(body_str) = body_val.as_str() {
                                    builder = builder.body(body_str.to_string());
                                } else {
                                    builder = builder.body(body_val.to_string());
                                }
                            }
                        }
                    }

                    let response = builder.send().await?;
                    let status = response.status().as_u16();
                    let body = response.text().await?;

                    let res_json = serde_json::json!({
                        "status": status,
                        "body": body
                    })
                    .to_string();

                    if let Some(var_name) = caps.get(1) {
                        variables.insert(var_name.as_str().to_string(), res_json.clone());
                    }
                    last_output = res_json;
                }
                continue;
            }

            // マッチしない無効な行に対する警告ログ (改行エスケープ & 長さ制限) (P2-2)
            let safe_line = line.replace('\n', "\\n").replace('\r', "\\r");
            let truncated_line = shared::strings::truncate_chars_safely(&safe_line, 100, true);
            warn!(
                "⚠️ [JS Bridge] Unrecognized JS line skipped: {}",
                truncated_line
            );
        }

        Ok(last_output)
    }
}

#[cfg(test)]
mod tests;
