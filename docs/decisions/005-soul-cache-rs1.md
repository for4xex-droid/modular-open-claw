# ADR-005: Soul Memory Cache（RS-1）

**Status**: Accepted  
**Date**: 2026-03-19  
**Deciders**: motivationstudio

## Context

チャットの LLM コールごとに `SqliteSoulStore::load_soul()` を呼ぶと:
- SQLite I/O が発生（ディスクアクセス）
- JSON デシリアライズコスト
- 数十万の experience_buffer を持つ Soul では顕著な遅延

しかし、チャットプロンプトに必要なのは Soul 全体ではなく**ごく一部のサマリ情報**のみ。

## Decision

`SqliteSoulStore` にインメモリ `SoulSnapshot` キャッシュを導入:

```rust
struct SoulSnapshot {
    attachment_style: AttachmentStyle,
    narrative_self: Option<String>,
    prompt_fragment: String,
    generation: u32,
}
```

- `save_soul()` 呼び出し時にキャッシュを自動更新
- `get_snapshot()` は `RwLock::read()` のみ（DB I/O ゼロ）
- `load_soul()` 時もキャッシュ未設定なら自動セット

## Consequences

- **Good**: チャットパスのDB I/O を完全排除
- **Good**: SoulSnapshot は Clone 可能な軽量構造体
- **Risk**: キャッシュと DB の不整合（save_soul を経由せずにDBが更新された場合）
- **Mitigation**: save_soul が唯一の書き込みパスであることをアーキテクチャルールで保証
