# ADR-036: MCP Dispatch and Security Centralization

## Status
Accepted

## Context
As we integrate high-performance CLI tools like `fff.nvim` running as MCP servers, we discovered critical architectural gaps in how `api-server` handled MCP tool execution (detailed in perfect_plan_fff_integration verification):
1. **Unreachable MCP Tools**: `tool_call_router.rs` did not have a dispatch loop for MCP. All unknown `skill_name` requests automatically fell back to executing Wasm skills, rendering MCP servers unreachable over the standard chat tool loop.
2. **Scattered Security Whitelists**: The security whitelist enforcing safe MCP commands (`npx`, `node`, `python3`) was duplicated across `mcp/client.rs` and `routes/skill.rs`.
3. **Missing Timeout Safeguards**: `McpClient::call()` and tool dispatch lacked explicit timeouts, allowing slow MCP requests (e.g., massive file system queries via `fff-mcp`) to indefinitely block Aiome's core chat loop processing threads.

## Decision
1. **Centralize Whitelists in shared::mcp_constants**: Create `libs/shared/src/mcp_constants.rs` containing `ALLOWED_MCP_COMMANDS` and `ALLOWED_MCP_PREFIXES` to be the single source of truth.
2. **Add Polling-based MCP Dispatch in Router**: Before defaulting to Wasm skill execution, the `DefaultToolCallRouter` sequentially queries all active MCP servers via `McpProcessManager.active_client_ids()`. If a client has the matching tool, it intercepts the invocation.
3. **Enforce Strict Timeouts**: `client.list_tools()` is limited to 2 seconds, and `client.call_tool()` is limited to 30 seconds (`tokio::time::timeout`) to preserve agent responsiveness.
4. **Binary Command Whitelist Exception**: Binary commands like `fff-mcp` added strictly to `ALLOWED_MCP_COMMANDS` successfully bypass the NPM `@` package-name restriction check while still guaranteeing security at the root level. 

## Consequences
- **Positive**: LLMs can autonomously utilize tools from loaded MCP servers without hardcoded handling paths.
- **Positive**: Risk of SSRF/Zombie processes locking up the Aiome core is mitigated via explicit process lifecycle management and tokio timeouts.
- **Negative / Constraint**: Iterating through all active MCP clients invoking IPC `list_tools()` upon tool dispatch incurs a slight O(N) performance hit. Given the `MAX_MCP_PROCESSES` limit is 10, this is negligible (<5ms), but should be upgraded to caching or `ToolDiscoveryEngine` lookup if active client limits explicitly grow.
