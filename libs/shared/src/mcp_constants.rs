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
];

/// Forbidden argument flags for MCP endpoints to prevent command injection.
pub const FORBIDDEN_MCP_ARG_FLAGS: &[&str] = &[
    "-c",
    "--eval",
    "-e",
    "--exec",
    "--shell-cmd",
    "--pre",
    "--post",
];
