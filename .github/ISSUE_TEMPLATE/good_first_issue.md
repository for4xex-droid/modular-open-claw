---
name: Good First Issue
about: Beginner-friendly issue for new contributors (MCP / Webhook integrations)
title: '[GOOD FIRST ISSUE] '
labels: 'good first issue, help wanted'
assignees: ''
---

**Description (概要)**
A clear and concise description of the task. For new contributors, we highly recommend tasks that involve extending the Aiome ecosystem:
- **MCP (Model Context Protocol) Server Integration**: Building a new server to give agents new capabilities.
- **Webhook Integration**: Integrating a new external service via webhooks into the Commerce or Activity engine.
(タスクの明確かつ簡潔な説明を記載してください。Aiomeエコシステムの拡張（MCPサーバー追加やWebhook連携など）を特に歓迎します。)

**Acceptance Criteria (完了条件)**
- [ ] 
- [ ] 

**Checklist before PR (PR提出前の必須チェックリスト)**
- [ ] **Zero-Panic Policy**: パブリックAPIや業務ロジック内で `unwrap()` や `expect()` を使用していませんか？
- [ ] **Negative Test**: 単なる正常系のテストだけでなく、不正入力などの異常系テスト（Negative Test）を含めていますか？
- [ ] **TDD**: テストを先に記述し、`cargo test --workspace` が 100% GREEN になることを確認しましたか？
- [ ] **Documentation**: 新規ファイルを追加した場合、`.context/RIPPLE_MAP.md` などを更新しましたか？

**Context & Resources (追加のコンテキストとリソース)**
Add any other context or pointers (e.g., related files) here.
- **MCP Dev Guide**: [Model Context Protocol](https://modelcontextprotocol.io)
- **Webhook Example**: See `apps/api-server/src/routes/commerce_webhook/stripe.rs` for event broadcasting patterns.
(関連するファイルや参考資料があれば記載してください)
