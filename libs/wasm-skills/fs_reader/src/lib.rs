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

fn is_safe_file_path(path: &Path) -> bool {
    // 0. パス全体のコンポーネント検査: .git/, .aws/, .kube/ などの隠しディレクトリを遮断
    for component in path.components() {
        if let std::path::Component::Normal(seg) = component {
            let s = seg.to_string_lossy();
            if s.starts_with('.') {
                // .git, .env, .aws, .kube 等の隠しコンポーネントを一律拒否
                return false;
            }
        }
    }

    // 1. ファイル名（ステム）のチェック
    if let Some(stem) = path.file_stem() {
        if stem.is_empty() {
            return false;
        }
    } else {
        return false;
    }

    // 2. ホワイトリスト拡張子のチェック (Default Deny)
    if let Some(ext_os) = path.extension() {
        if let Some(ext) = ext_os.to_str() {
            let ext_lower = ext.to_lowercase();
            let allowed = [
                "txt", "md", "json", "yaml", "yml", "csv", "tsv", "toml",
                "rs", "py", "ts", "js", "html", "css", "xml", "log",
            ];
            return allowed.contains(&ext_lower.as_str());
        }
    }

    // 拡張子がない、または未定義の場合は デフォルト拒否 (Default Deny)
    false
}

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
    let full_path = Path::new("/mnt/workspace").join(&req.path);

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

    if !canonical_path.starts_with(Path::new("/mnt/workspace")) {
        let res = ReadResponse {
            content: String::new(),
            error: Some("Security Violation: Path traversal blocked.".into()),
        };
        return Ok(serde_json::to_string(&res)?);
    } else if !is_safe_file_path(&canonical_path) {
        let res = ReadResponse {
            content: String::new(),
            error: Some(
                "Security Violation: Access to this file type is forbidden by allowlist policy."
                    .into(),
            ),
        };
        return Ok(serde_json::to_string(&res)?);
    }

    // SEC: File size limit (10MB) to prevent OOM in WASM sandbox
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
    match fs::metadata(&canonical_path) {
        Ok(meta) if meta.len() > MAX_FILE_SIZE => {
            let res = ReadResponse {
                content: String::new(),
                error: Some(format!(
                    "File too large ({} bytes). Maximum allowed: {} bytes.",
                    meta.len(),
                    MAX_FILE_SIZE
                )),
            };
            return Ok(serde_json::to_string(&res)?);
        }
        Err(e) => {
            let res = ReadResponse {
                content: String::new(),
                error: Some(format!("Cannot stat file: {}", e)),
            };
            return Ok(serde_json::to_string(&res)?);
        }
        _ => {} // Size OK, proceed
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_allowed_extensions() {
        assert!(is_safe_file_path(Path::new("/mnt/workspace/test.txt")));
        assert!(is_safe_file_path(Path::new("/mnt/workspace/data.json")));
        assert!(is_safe_file_path(Path::new("/mnt/workspace/README.md")));
        assert!(is_safe_file_path(Path::new("/mnt/workspace/src/main.rs")));
        assert!(is_safe_file_path(Path::new("/mnt/workspace/config.yaml")));
        // Case insensitive
        assert!(is_safe_file_path(Path::new("/mnt/workspace/DOC.TXT")));
    }

    #[test]
    fn test_blocked_extensions() {
        // Hidden files
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.env")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.kube/config")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.aws/credentials")));

        // Crypto / Secret files
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/secret.pem")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/key.p12")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/id_rsa")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/passwd")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/shadow")));

        // Executables / Binaries / DB
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/app.exe")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/script.sh")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/data.sqlite")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/data.sql")));
    }

    #[test]
    fn test_hidden_dir_bypass() {
        // Critical: files with allowed extensions inside hidden directories must be BLOCKED
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.git/config.json")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.git/HEAD")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.aws/config.yaml")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.ssh/known_hosts.txt")));
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/.env.local")));
    }

    #[test]
    fn test_no_extension() {
        // Files without extensions should be blocked (Default Deny)
        assert!(!is_safe_file_path(Path::new("/mnt/workspace/unknown_binary")));
    }
}
