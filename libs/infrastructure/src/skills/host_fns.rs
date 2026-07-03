/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! WASM ホスト関数ビルダー（`host_exec` / `host_write`）。
//!
//! ── B-1: Memory Safety Contract (Bun Rust Rewrite Pattern) ──
//! The host_exec/host_write functions use Extism's memory pointer pipeline:
//!   1. Guest passes I64 offset → host validates via memory_handle()
//!   2. memory_handle() returns None if offset is out-of-bounds (safe)
//!   3. memory_str() validates UTF-8 encoding (safe)
//!   4. Response is allocated via memory_alloc() with exact length (no overflow)
//!
//! When WASI P2 + Component Model becomes available, these raw pointer
//! exchanges should be replaced with WIT-typed interfaces.

use super::is_sensitive_path;
use crate::security::{BastionGuard, PermissionManifest, RuntimeJail};
use extism::{Function, UserData, Val, ValType};
use std::path::PathBuf;

/// `host_exec`（シェル実行）ホスト関数を構築する
pub(super) fn build_host_exec_fn(permissions: PermissionManifest) -> Function {
    Function::new(
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
            let cmd_str: String = plugin
                .memory_str(handle)
                .map_err(|e: extism::Error| e)?
                .to_string();
            let guard = BastionGuard::new(permissions.clone());
            let runtime = tokio::runtime::Handle::current();
            let res = runtime.block_on(async { guard.safe_exec(&cmd_str).await });

            // Step 4: Response allocation with exact byte length
            match res {
                Ok(stdout_str) => {
                    let stdout_bytes = stdout_str.as_bytes();
                    let mem = plugin.memory_alloc(stdout_bytes.len() as u64)?;
                    plugin.memory_bytes_mut(mem)?.copy_from_slice(stdout_bytes);
                    outputs[0] = Val::I64(mem.offset() as i64);
                }
                Err(e) => {
                    let err_msg = format!("Bastion Guard Error: {}", e);
                    tracing::warn!("🛡️ [host_exec] BastionGuard rejected command: {}", e);
                    let mem = plugin.memory_alloc(err_msg.len() as u64)?;
                    plugin
                        .memory_bytes_mut(mem)?
                        .copy_from_slice(err_msg.as_bytes());
                    outputs[0] = Val::I64(mem.offset() as i64);
                }
            }
            Ok(())
        },
    )
}

/// `host_write`（サンドボックス内ファイル書込）ホスト関数を構築する。
/// `allowed_root` は呼び出し元で canonicalize 済みのパスを渡すこと。
pub(super) fn build_host_write_fn(
    permissions: PermissionManifest,
    allowed_root: PathBuf,
    vault_path: Option<PathBuf>,
) -> Function {
    Function::new(
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
                tracing::warn!(
                    "🛡️ [host_write] Invalid memory handle at offset {}",
                    json_ptr
                );
                extism::Error::msg("Invalid memory handle for host_write")
            })?;
            let req_str = plugin.memory_str(handle).map_err(|e: extism::Error| e)?;

            if !permissions.allow_filesystem_write {
                let res_json = serde_json::json!({ "success": false, "path": "", "error": "Security Violation: Field writing is not permitted for this skill." }).to_string();
                let mem = plugin.memory_alloc(res_json.len() as u64)?;
                plugin
                    .memory_bytes_mut(mem)?
                    .copy_from_slice(res_json.as_bytes());
                outputs[0] = Val::I64(mem.offset() as i64);
                return Ok(());
            }

            #[derive(serde::Deserialize)]
            struct WriteReq {
                path: String,
                content: String,
            }
            let res_json = match serde_json::from_str::<WriteReq>(req_str) {
                Ok(req) => {
                    let full_path = allowed_root.join(&req.path);
                    let parent_dir = full_path.parent().unwrap_or(&full_path);
                    if !parent_dir.exists() {
                        let _ = std::fs::create_dir_all(parent_dir);
                    }
                    match std::fs::canonicalize(parent_dir) {
                        Ok(canon_parent) => {
                            let Some(file_name) = full_path.file_name() else {
                                let res_json = serde_json::json!({ "success": false, "path": "", "error": "Invalid filename" }).to_string();
                                let mem = plugin.memory_alloc(res_json.len() as u64)?;
                                plugin
                                    .memory_bytes_mut(mem)?
                                    .copy_from_slice(res_json.as_bytes());
                                outputs[0] = Val::I64(mem.offset() as i64);
                                return Ok(());
                            };
                            let final_path = canon_parent.join(file_name);

                            let mut path_allowed = final_path.starts_with(&allowed_root);

                            // Check against Vault if workspace missed
                            if !path_allowed {
                                if let Some(vault_root) = &vault_path {
                                    if final_path.starts_with(vault_root) {
                                        path_allowed = true;
                                    }
                                }
                            }

                            let is_sensitive = is_sensitive_path(&final_path);

                            if !path_allowed {
                                serde_json::json!({ "success": false, "path": "", "error": "Security Violation: Path traversal blocked." }).to_string()
                            } else if is_sensitive {
                                serde_json::json!({ "success": false, "path": "", "error": "Security Violation: Access to sensitive internal file is forbidden." }).to_string()
                            } else {
                                if let Some(parent) = final_path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                match std::fs::write(&final_path, req.content) {
                                    Ok(_) => serde_json::json!({ "success": true, "path": final_path.to_string_lossy().to_string(), "error": None::<String> }).to_string(),
                                    Err(e) => serde_json::json!({ "success": false, "path": "", "error": format!("Write failed: {}", e) }).to_string(),
                                }
                            }
                        }
                        Err(e) => serde_json::json!({ "success": false, "path": "", "error": format!("Parent path canonicalization failed: {}", e) }).to_string(),
                    }
                }
                Err(e) => serde_json::json!({ "success": false, "path": "", "error": format!("Invalid JSON payload: {}", e) }).to_string(),
            };

            let mem = plugin.memory_alloc(res_json.len() as u64)?;
            plugin
                .memory_bytes_mut(mem)?
                .copy_from_slice(res_json.as_bytes());
            outputs[0] = Val::I64(mem.offset() as i64);
            Ok(())
        },
    )
}

/// dry-run 用の no-op ホスト関数（host_exec / host_write のスタブ）を構築する
pub(super) fn build_noop_host_fns() -> Vec<Function> {
    let host_exec_fn = Function::new(
        "host_exec",
        [ValType::I64],
        [ValType::I64],
        UserData::new(()),
        |plugin, _inputs, outputs, _user_data| {
            let mem = plugin.memory_alloc(0)?;
            outputs[0] = Val::I64(mem.offset() as i64);
            Ok(())
        },
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
        },
    );
    vec![host_exec_fn, host_write_fn]
}
