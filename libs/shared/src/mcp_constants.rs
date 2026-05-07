/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/// Allowed binary commands for MCP endpoints.
/// NOTE: Both `python` and `python3` are needed because some systems
/// (e.g. Debian/Docker) only have `python3`, while others (e.g. macOS
/// with pyenv, or user-configured mcp_servers.json) may specify `python`.
pub const ALLOWED_MCP_COMMANDS: &[&str] = &[
    "npx", "node", "python3", "python", "uvx",
    "fff-mcp", // Binary command — skips package-prefix validation
    "geo-mcp",
];

/// Allowed npm/uvx package prefixes for MCP endpoints.
pub const ALLOWED_MCP_PREFIXES: &[&str] = &[
    "@modelcontextprotocol/",
    "@stripe/",
    "@appsyogi/",
    "@secops/",
    // Phase 1: MCP Ecosystem Expansion
    "@brightdata/",
    "@upstash/",
    "@playwright/",
    "@canva/",
    "@crewai/",
    "@autogen/",
    "@iflow-mcp/", // X(Twitter) MCP Server
];

/// Allowed unscoped npm packages for MCP (exact match).
/// These packages don't have an @scope/ prefix, so prefix matching won't work.
pub const ALLOWED_MCP_PACKAGES: &[&str] = &[
    "firecrawl-mcp",
    "exa-mcp-server",
    "chrome-devtools-mcp",
    "freee-mcp",
    "mcp-remote",
    "discord-mcp-server",
    "notion-mcp-server",
];

/// Forbidden argument flags for MCP endpoints to prevent command injection.
///
/// Note: `-y` / `--yes` are intentionally NOT included here.
/// They only skip the interactive prompt in npx/uvx; the actual package
/// safety is enforced by `validate_mcp_package` (whitelist gate).
pub const FORBIDDEN_MCP_ARG_FLAGS: &[&str] = &[
    "-c",
    "--eval",
    "-e",
    "--exec",
    "--shell-cmd",
    "--pre",
    "--post",
];

/// Validates that the package name in a `npx`/`uvx` command is whitelisted.
///
/// Returns `Ok(())` if the package is allowed, or `Err(reason)` if not.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The first positional argument is not in `ALLOWED_MCP_PREFIXES` or `ALLOWED_MCP_PACKAGES`.
/// - No positional argument (non-flag) is found in `args`.
pub fn validate_mcp_package(command: &str, args: &[String]) -> Result<(), String> {
    if command != "npx" && command != "uvx" {
        return Ok(());
    }

    let pkg = args.iter().find(|a| !a.starts_with('-'));
    if let Some(p) = pkg {
        if !ALLOWED_MCP_PREFIXES
            .iter()
            .any(|prefix| p.starts_with(prefix))
            && !ALLOWED_MCP_PACKAGES.iter().any(|pkg_name| {
                p == *pkg_name
                    || p.strip_prefix(pkg_name)
                        .is_some_and(|rest| rest.starts_with('@'))
            })
        {
            return Err(format!("MCP package '{}' is not whitelisted", p));
        }
    } else {
        return Err(format!("Missing package name for command '{}'", command));
    }
    Ok(())
}

/// Validates that no forbidden argument flags are present in the args.
///
/// Returns `Ok(())` if clean, or `Err(reason)` with the offending flag.
///
/// # Errors
///
/// Returns `Err(String)` if any argument matches a pattern in `FORBIDDEN_MCP_ARG_FLAGS`.
/// Matching handles both long flags (`--eval`, `--eval=code`) and
/// short flags (`-c`, `-cVALUE`), while excluding false positives from
/// long flags starting with `--` (e.g. `--env-file` does not match `-e`).
pub fn validate_mcp_arg_flags(args: &[String]) -> Result<(), String> {
    for arg in args {
        let lower = arg.to_lowercase();

        // Note: -y / --yes (for skipping npx/uvx prompts) are NOT blocked.
        // See FORBIDDEN_MCP_ARG_FLAGS doc comment for rationale.

        for flag in FORBIDDEN_MCP_ARG_FLAGS {
            let matched = if flag.starts_with("--") {
                // Long flags: match exact or --flag=value
                lower == *flag
                    || lower
                        .strip_prefix(flag)
                        .is_some_and(|rest| rest.starts_with('='))
            } else {
                // Short flags (-c, -e): match exact or -cVALUE
                // Guard: exclude long flags (--env-file must NOT match -e)
                !lower.starts_with("--")
                    && (lower == *flag || (lower.starts_with(flag) && lower.len() > flag.len()))
            };
            if matched {
                return Err(format!(
                    "Forbidden argument flag '{}' in MCP command (matched: {})",
                    arg, flag
                ));
            }
        }
    }
    Ok(())
}
