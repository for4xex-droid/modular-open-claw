---
title: '[Good First Issue] Add Google Drive MCP to Whitelist'
labels: ['good first issue', 'mcp', 'ecosystem']
assignees: ''
---

## Description
Aiome OS utilizes the Model Context Protocol (MCP) to federate external skills. Currently, we officially support `@github/`, `@notion/`, and `@tavily/` prefixes in our security whitelist. We want to expand this to include Google Drive for automated document management.

## Task Requirements
1. Update `libs/shared/src/mcp_constants.rs` to include `@google/` or `@modelcontextprotocol/server-gdrive` in the whitelist.
2. Add a basic unit test in the same file to verify that the new prefix is accepted by `is_whitelisted_package`.

## Why this is a Good First Issue
This task is highly isolated, has a clear existing pattern (see Notion and GitHub implementations in the same file), and immediately adds value to the Aiome ecosystem.

## TDD Acceptance Criteria
- [ ] `cargo test -p shared` passes perfectly with the new test case included.
- [ ] No `unwrap()` or `expect()` is used in the implementation (Zero-Panic policy).

## Getting Started
Please comment "I'd like to work on this!" below to get assigned. Review `CONTRIBUTING.md` for our workspace setup instructions.
