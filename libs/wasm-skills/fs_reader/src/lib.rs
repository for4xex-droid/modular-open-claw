/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]

//! # クレート固有のインデックス
//!
#![forbid(unsafe_code)]
#![allow(missing_docs)]
use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct ReadRequest {
    pub path: String,
}

#[derive(Serialize)]
struct ReadResponse {
    pub content: String,
    pub error: Option<String>,
}

/// サポートドキュメント
#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: ReadRequest = serde_json::from_str(&input)?;

    // In Aiome OS, skills are typically jail-rooted to /mnt/workspace
    // The request 'path' is relative to that root.
    let full_path = Path::new("/mnt").join(&req.path);

    // Validate path to prevent directory traversal
    let canonical_path = match std::fs::canonicalize(&full_path) {
        Ok(p) => p,
        Err(e) => {
            let res = ReadResponse {
                content: String::new(),
                error: Some(format!("Invalid path: {}", e)),
            };
            return Ok(serde_json::to_string(&res)?);
        }
    };

    let p_str = canonical_path.to_string_lossy().to_lowercase();
    if !canonical_path.starts_with(Path::new("/mnt")) {
        let res = ReadResponse {
            content: String::new(),
            error: Some("Security Violation: Path traversal blocked.".into()),
        };
        return Ok(serde_json::to_string(&res)?);
    } else if p_str.contains(".env")
        || p_str.contains(".git")
        || p_str.contains(".db")
        || p_str.contains(".sqlite")
        || p_str.contains("credentials")
        || p_str.contains("id_rsa")
        || p_str.contains("history")
        || p_str.contains("config/security.json")
    {
        let res = ReadResponse {
            content: String::new(),
            error: Some(
                "Security Violation: Access to sensitive system or database file is forbidden."
                    .into(),
            ),
        };
        return Ok(serde_json::to_string(&res)?);
    }

    match fs::read_to_string(&canonical_path) {
        Ok(content) => {
            let res = ReadResponse {
                content,
                error: None,
            };
            Ok(serde_json::to_string(&res)?)
        }
        Err(e) => {
            let res = ReadResponse {
                content: String::new(),
                error: Some(format!("Could not read file: {}", e)),
            };
            Ok(serde_json::to_string(&res)?)
        }
    }
}
