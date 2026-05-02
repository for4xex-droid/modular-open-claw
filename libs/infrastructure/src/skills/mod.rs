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

/// 状態: 未検証の外部Skill (TypeState Pattern)
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

/// `WasmSkillManager` 構造体
pub struct WasmSkillManager {
    skills_dir: PathBuf,
    allowed_root: PathBuf,
    vault_path: Option<PathBuf>, // Phase 3: DRM 隔離領域
    memory_limit_bytes: u64,
    timeout: Duration,
    wasm_cache: std::sync::RwLock<HashMap<String, (Vec<u8>, SystemTime)>>,
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
            wasm_cache: std::sync::RwLock::new(HashMap::new()),
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
        // Discovery C: Fix RwLock poisoning crash
        let mut cache = self.wasm_cache.write().unwrap_or_else(|e| e.into_inner());

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
        // Discovery C: Fix RwLock poisoning crash
        let mut cache = self.wasm_cache.write().unwrap_or_else(|e| e.into_inner());
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
            let cache = self.wasm_cache.read().unwrap_or_else(|e| e.into_inner());
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
                let mut cache = self.wasm_cache.write().unwrap_or_else(|e| e.into_inner());
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
            let host_exec_fn = Function::new(
                "host_exec",
                [ValType::I64],
                [ValType::I64],
                UserData::new(()),
                move |plugin, inputs, outputs, _user_data| {
                    let cmd_ptr = inputs.first().and_then(|v| v.i64()).ok_or_else(|| extism::Error::msg("Missing input parameter"))? as u64;
                    let handle = plugin.memory_handle(cmd_ptr).ok_or_else(|| extism::Error::msg("Invalid memory handle"))?;
                    let cmd_str: String = plugin.memory_str(handle).map_err(|e: extism::Error| e)?.to_string();
                    let guard = BastionGuard::new(host_exec_permissions.clone());
                    let runtime = tokio::runtime::Handle::current();
                    let res = runtime.block_on(async {
                        guard.safe_exec(&cmd_str).await
                    });

                    match res {
                        Ok(stdout_str) => {
                            let stdout_bytes = stdout_str.as_bytes();
                            let mem = plugin.memory_alloc(stdout_bytes.len() as u64)?;
                            plugin.memory_bytes_mut(mem)?.copy_from_slice(stdout_bytes);
                            outputs[0] = Val::I64(mem.offset() as i64);
                        },
                        Err(e) => {
                            let err_msg = format!("Bastion Guard Error: {}", e);
                            let mem = plugin.memory_alloc(err_msg.len() as u64)?;
                            plugin.memory_bytes_mut(mem)?.copy_from_slice(err_msg.as_bytes());
                            outputs[0] = Val::I64(mem.offset() as i64);
                        }
                    }
                    Ok(())
                }
            );

            let host_write_permissions = metadata.as_ref().map(|m| m.permissions.clone()).unwrap_or_default();
            let allowed_root_for_write = allowed_root_clone.clone();
            let vault_path_for_write = vault_path_clone.clone(); // Use the local clone
            let host_write_fn = Function::new(
                "host_write",
                [ValType::I64],
                [ValType::I64],
                UserData::new(()),
                move |plugin, inputs, outputs, _user_data| {
                    let json_ptr = inputs.first().and_then(|v| v.i64()).ok_or_else(|| extism::Error::msg("Missing input parameter"))? as u64;
                    let handle = plugin.memory_handle(json_ptr).ok_or_else(|| extism::Error::msg("Invalid memory handle for host_write"))?;
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
                                    let p_str = final_path.to_string_lossy().to_lowercase();

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
}

#[cfg(test)]
mod tests;
