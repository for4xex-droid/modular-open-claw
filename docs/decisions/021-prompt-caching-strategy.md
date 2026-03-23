# ADR-021: Prompt Caching 最適化戦略

> **Status**: Accepted  
> **Date**: 2026-03-24  
> **Origin**: Claude Code Team「Prompt Caching Is Everything」設計教訓  
> **Impact**: Phase 29 に集約（LlmProvider 層のリファクタ）

## Context

Aiome の全 LLM 呼び出しは `LlmProvider::complete(prompt, system)` を通過する。
現在の実装にはプロンプトキャッシュ最適化が一切なく、以下のコスト・レイテンシ問題が存在する。

1. **ContextEngine**: `maintain_context` の Compaction が独立プロンプトで実行され、キャッシュ共有不可
2. **FallbackRouter**: モデル切替時にキャッシュが全破壊される
3. **IntentGenerator / TaxonomyClassifier**: 静的な system_prompt を毎回送信しているが、プレフィックスキャッシュ未活用

## Decision

### Phase 29 に LLM 基盤リファクタとして集約

Phase 29 は PostgreSQL 移行と同時に、LLM API 層のインフラ強化を行う最適なタイミングである。

#### D-1: `LlmProvider` トレイトの拡張

```rust
// 現在
async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse>;

// 拡張案: キャッシュ制御パラメータを追加
async fn complete_with_cache(
    &self,
    request: &LlmRequest,  // system, messages, cache_breakpoints を含む
) -> Result<LlmResponse>;
```

デフォルト実装で `complete` を `complete_with_cache` に委譲し、後方互換を維持する。

#### D-2: プロンプト構成順序の強制

```
1. [Static] System Prompt + Tool Definitions  → cache_control: ephemeral
2. [Project] Soul Constitution / CLAUDE.md 相当 → cache_control: ephemeral
3. [Session] 会話コンテキスト要約             → セッション内キャッシュ
4. [Dynamic] 最新メッセージ                    → キャッシュなし
```

#### D-3: Cache-Safe Compaction

`ContextEngine::maintain_context` を、親セッションと同一プレフィックスを共有する形に修正。

#### D-4: Soul State をメッセージで伝達

Soul の動的情報（Level, Fatigue, Karma）をシステムプロンプトではなく `<soul-update>` タグとしてメッセージに挿入。

## Phase 統合: P29 に 2 タスク追加

| # | タスク | 工数 | 効果 |
|---|---|---|---|
| 8 | `LlmRequest` 構造体 + `complete_with_cache` の追加 | M | 全後続 Phase でキャッシュ活用可能に |
| 9 | `ContextEngine` Compaction をキャッシュセーフ化 + Soul State メッセージ化 | S | コスト 30-60% 削減 |

## Consequences

- **コスト削減**: プロンプトキャッシュヒットにより LLM API コスト 30-60% 削減見込み
- **後方互換**: `complete()` はデフォルト実装で `complete_with_cache()` に委譲
- **トレイト数**: 新トレイト追加なし（既存 `LlmProvider` の拡張のみ）
- **工数**: +2 日（Phase 29 に統合）
