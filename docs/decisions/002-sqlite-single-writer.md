# ADR-002: SQLite Single-Writer 制約とデッドロック回避

**Status**: Accepted  
**Date**: 2026-03-18  
**Deciders**: motivationstudio

## Context

SQLite は軽量で組み込みに最適だが、**同一接続内で同時に1つのトランザクションしか保持できない**。async Rust との組み合わせで、再帰的な async 関数が2つ目のトランザクションを開こうとしてデッドロックが発生した。

## Decision

1. **R-001**: `do_*` メソッドから swarm 操作を呼ぶ場合、`JobQueue` トレイト経由ではなく `SwarmOps::do_*` を直接使う
2. **R-008**: async 関数から同じ async 関数を再帰的に呼び出さない。リニアフローで解決する
3. **RUST_MIN_STACK=64MB**: debug ビルドの巨大な async state machine に対応

## Consequences

- **Good**: 8件のハングテストが解消
- **Bad**: コードの呼び出しパターンが制限される（直感的でない箇所あり）
- **Rule**: `.agent/skills/architecture-rules.md` の R-001, R-008 に文書化済み
