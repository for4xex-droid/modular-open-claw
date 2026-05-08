---
name: Good First Issue
about: Beginner-friendly issue for new contributors
title: '[GOOD FIRST ISSUE] '
labels: 'good first issue, help wanted'
assignees: ''
---

**Description (概要)**
A clear and concise description of the task.
(タスクの明確かつ簡潔な説明を記載してください)

**Acceptance Criteria (完了条件)**
- [ ] 
- [ ] 

**Checklist before PR (PR提出前の必須チェックリスト)**
- [ ] **Zero-Panic Policy**: パブリックAPIや業務ロジック内で `unwrap()` や `expect()` を使用していませんか？
- [ ] **Negative Test**: 単なる正常系のテストだけでなく、不正入力などの異常系テスト（Negative Test）を含めていますか？
- [ ] **TDD**: テストを先に記述し、`cargo test --workspace` が 100% GREEN になることを確認しましたか？
- [ ] **Documentation**: 新規ファイルを追加した場合、`.context/RIPPLE_MAP.md` などを更新しましたか？

**Context (追加のコンテキスト)**
Add any other context or pointers (e.g., related files) here.
(関連するファイルや参考資料があれば記載してください)
