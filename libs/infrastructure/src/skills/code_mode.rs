/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Code Mode JS ブリッジ。
//!
//! `code_mode.d.ts` に準拠した疑似 JavaScript（`aiome.log/exec/writeFile/readFile/fetch`
//! の5命令）を正規表現で行単位に解釈するミニインタープリタ。本物の JS エンジンではない。

use super::{is_sensitive_path, WasmSkillManager};
use crate::security::{BastionGuard, RuntimeJail};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::{info, warn};

static LOG_REGEX: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r#"aiome\.log\((.*)\);"#).ok());
static EXEC_REGEX: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(r#"(?:const\s+(\w+)\s*=\s*(?:await\s+)?)?aiome\.exec\((.*)\);"#).ok()
});
static WRITE_REGEX: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r#"aiome\.writeFile\((.*)\);"#).ok());
static READ_REGEX: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(r#"(?:const\s+(\w+)\s*=\s*(?:await\s+)?)?aiome\.readFile\((.*)\);"#).ok()
});
static FETCH_REGEX: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(r#"(?:const\s+(\w+)\s*=\s*(?:await\s+)?)?aiome\.fetch\((.*)\);"#).ok()
});

/// `${var}` 形式の変数を展開する
fn expand_vars(s: &str, vars: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    for (k, v) in vars {
        let pattern_curly = format!("${{{}}}", k);
        result = result.replace(&pattern_curly, v);
    }
    result
}

/// 前後のクォート（" ' `）を1組だけ剥がす
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

/// トークンを解決する: クォート文字列なら変数展開、そうでなければ変数参照
fn resolve_token(token: &str, vars: &HashMap<String, String>) -> String {
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
}

/// code_mode.d.ts に準拠した JavaScript コードを一括ロード・安全に実行する JS エンジニアブリッジ
/// 🛡️ セキュリティロック: allow_shell_execution が false の場合は host_exec (aiome.exec) を遮断する
pub(super) async fn run_code_mode_js_impl(
    manager: &WasmSkillManager,
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
    let canon_root = std::fs::canonicalize(&manager.allowed_root)
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

        // 1. aiome.log
        if let Some(caps) = LOG_REGEX.as_ref().and_then(|r| r.captures(line)) {
            let inner = caps[1].trim();
            let msg = resolve_token(inner, &variables);
            info!("📝 [JS Log] {}", msg);
            last_output = msg;
            continue;
        }

        // 2. aiome.exec
        if let Some(caps) = EXEC_REGEX.as_ref().and_then(|r| r.captures(line)) {
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
        if let Some(caps) = WRITE_REGEX.as_ref().and_then(|r| r.captures(line)) {
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
                        "Security Violation: Access to sensitive internal file is forbidden".into(),
                    );
                }

                std::fs::write(&final_path, content)?;
                last_output = format!("Wrote to {}", relative_path);
            }
            continue;
        }

        // 4. aiome.readFile
        if let Some(caps) = READ_REGEX.as_ref().and_then(|r| r.captures(line)) {
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
        if let Some(caps) = FETCH_REGEX.as_ref().and_then(|r| r.captures(line)) {
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
                    if domain == "*" || domain == host || host.ends_with(&format!(".{}", domain)) {
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
                    if let Ok(json_args) = serde_json::from_str::<serde_json::Value>(&extra_args) {
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
