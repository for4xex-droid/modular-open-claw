# ADR-006: Box::pin による JobQueue Delegation（R-002）

**Status**: Accepted  
**Date**: 2026-03-18  
**Deciders**: motivationstudio

## Context

`impl JobQueue for SqliteJobQueue` は **60以上のメソッド** を持ち、各メソッドが `async fn` として定義されている。Rust の async/await は各 async fn をステートマシンに変換するため、全メソッドの合計 state machine サイズが数百 KB に達し、**debug ビルドでスタックオーバーフロー**が発生した。

## Decision

全ての委譲メソッドで `Box::pin()` を使い、async state machine をヒープに配置:

```rust
async fn store_karma(&self, ...) -> Result<(), AiomeError> {
    Box::pin(self.do_store_karma(...)).await
}
```

補助策として `.cargo/config.toml` に `RUST_MIN_STACK=64MB` を設定。

## Consequences

- **Good**: スタックオーバーフロー完全解消
- **Bad**: 全メソッドに Box::pin のボイラープレートが必要
- **Bad**: ヒープ割り当てによる微小なオーバーヘッド（実測影響なし）
- **Rule**: `.agent/skills/architecture-rules.md` の R-002 に文書化済み
