/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]

// WASM ゲストはホスト関数を unsafe で呼ぶ必要がある。
// Wasmtime サンドボックスにより安全性は保証される。
//! サポートドキュメント
#![allow(unsafe_code)]
#![allow(missing_docs)]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[host_fn]
extern "ExtismHost" {
    fn host_exec(cmd: String) -> String;
}

#[derive(Deserialize)]
struct ExecRequest {
    pub cmd: String,
}

#[derive(Serialize)]
struct ExecResponse {
    pub stdout: String,
    pub stderr: Option<String>,
}

/// サポートドキュメント
#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: ExecRequest = serde_json::from_str(&input)?;

    let cmd = req.cmd.trim();
    if cmd.is_empty() {
        return Err(extism_pdk::Error::msg("Command cannot be empty.").into());
    }

    let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();
    let base_cmd = cmd_parts[0];

    // Minimal allowlist of safe base commands
    let allowlist = [
        "ls", "pwd", "echo", "cat", "grep", "git", "cargo", "npm", "node", "python", "python3",
        "rustc", "tsc", "npx",
    ];

    if !allowlist.contains(&base_cmd.to_lowercase().as_str()) {
        let res = ExecResponse {
            stdout: String::new(),
            stderr: Some(format!(
                "Error: Sentinel Sandbox blocked command '{}'. Not in allowlist.",
                base_cmd
            )),
        };
        return Ok(serde_json::to_string(&res)?);
    }

    // Strict ban on shell chaining and redirection
    let dangerous_syntax = [">", ">>", "|", "&", ";", "`", "$(", "\n"];
    for syntax in &dangerous_syntax {
        if cmd.contains(syntax) {
            let res = ExecResponse {
                stdout: String::new(),
                stderr: Some(format!("Error: Sentinel Sandbox blocked command. Forbidden shell syntax '{}' detected.", syntax)),
            };
            return Ok(serde_json::to_string(&res)?);
        }
    }

    // Call the host function (Aiome OS Sentinel)
    let result = unsafe { host_exec(cmd.to_string())? };

    let res = ExecResponse {
        stdout: result,
        stderr: None,
    };
    Ok(serde_json::to_string(&res)?)
}
