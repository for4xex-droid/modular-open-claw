# ADR-003: Soul クレートと Infrastructure のトレイト境界分離

**Status**: Accepted  
**Date**: 2026-03-14  
**Deciders**: motivationstudio

## Context

`soul` クレートの3層ロジックを、特定のDB・LLM・APIに依存させたくない。テストで LLM や SQLite を使わずにドメインロジックを検証したい。

## Decision

`soul` クレートは **トレイト定義のみ** を持ち、具体的な実装は `infrastructure` クレートに配置する:

```
soul/adapter.rs    → trait SoulDomainAdapter（定義）
soul/engine.rs     → trait SamsaraEngine（定義）

infrastructure/soul_adapter.rs    → struct CoreDomainAdapter（実装）
infrastructure/samsara_engine.rs  → struct DefaultSamsaraEngine（実装）
```

`SoulPipeline<A: SoulDomainAdapter, E: SamsaraEngine>` のジェネリクスにより、テスト時は DummyAdapter/DummyEngine で差し替え可能。

## Consequences

- **Good**: soul クレートの単体テストに LLM/DB 不要
- **Good**: 将来的に別の LLM バックエンドへの差し替えが容易
- **Bad**: トレイトシグネチャ変更時に soul と infrastructure の両方を同期する必要がある
- **Mitigation**: RIPPLE_MAP.md に影響範囲を明記（→ `.context/RIPPLE_MAP.md`）
