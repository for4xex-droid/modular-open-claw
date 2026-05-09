---
title: '[Good First Issue] Add E2E Error Handling for MCP Discovery JSON Parsing'
labels: ['good first issue', 'api-server', 'error-handling']
assignees: ''
---

## Description
When the Aiome Management Console boots up, it reads `mcp_servers.json`. If this file is corrupted, the system currently handles it, but the error message propagated to the UI could be more descriptive. We want to improve the specific `AppError` mapping when `serde_json::from_str` fails in `discovery.rs`.

## Task Requirements
1. Locate `apps/api-server/src/mcp/discovery.rs` and find the `load_mcp_config` function.
2. Enhance the error wrapping when JSON parsing fails to include the file path or specific parse error context in the `AppError::internal` or `AppError::bad_request` string.
3. Update the corresponding unit test `test_load_mcp_config_invalid_json` in the same file to assert the new error message format.

## Why this is a Good First Issue
It involves learning how Aiome maps Rust errors to API responses via `AppError`, which is a fundamental pattern used throughout the codebase.

## TDD Acceptance Criteria
- [ ] The unit test `test_load_mcp_config_invalid_json` is updated and passes (`cargo test -p api-server`).
- [ ] The error message explicitly mentions "Invalid JSON format in mcp_servers.json".

## Getting Started
Please comment "I'd like to work on this!" below to get assigned. Review `CONTRIBUTING.md` for our workspace setup instructions.
