# ADR-020: Agent Design Patterns (Progressive Disclosure & Action Space 最適化)

> **Status**: Accepted  
> **Date**: 2026-03-24  
> **Origin**: Claude Code Team「Seeing like an Agent」設計教訓の分析  
> **Impact**: Phase 29〜34 に分散統合

## Context

Claude Code チームの設計教訓から、エージェント設計における 5 つのパターンを特定した。
これらは Aiome の自律 AI エージェントアーキテクチャに直接適用可能である。

## Decision

### 1. Progressive Disclosure（段階的コンテキスト発見）
- `DreamState::explorative_dream` のシード選択を固定リストから **自律検索ループ** に置換
- Zone API に `summary` → `detail` の 2 段階エンドポイントを導入
- 全エンジンの能力要約を返す `CapabilityRegistry` を新設

### 2. Action Space 最適化（トレイト数制約）
- `aiome-contracts` の trait 数を **20 以下に維持**
- `SubscriptionEngine` は独立トレイトではなく `CommerceEngine` の拡張メソッドとして実装
- 新機能は既存トレイトの拡張か Progressive Disclosure で対応

### 3. Elicitation パターン（構造化質問）
- `IntentFirewall` を `Allow/Deny` の 2 値から `Allow/Deny/AskUser` の **3 値判定** に拡張
- `AskUser` 時は構造化された選択肢をフロントエンドに返却

### 4. 自力コンテキスト構築
- エージェントにコンテキストを渡すのではなく、自力で構築させる設計を優先
- `DreamState` に `max_depth: 3`, `max_coins: 100` の探索ループを追加

## Phase 統合マップ

| Phase | タスク | 工数 |
|---|---|---|
| P29 | `CapabilityRegistry` 新設 | S |
| P30 | `CommerceEngine` に Subscription 拡張（独立トレイト回避） | S |
| P31 | `DreamState` 自律検索ループ | M |
| P32 | `IntentFirewall` 3 値判定化 | M |
| P34 | Zone Progressive Disclosure API | S |

## Consequences

- クリティカルパスへの影響: ゼロ（各 Phase にオプショナルタスクとして追加）
- トレイト数: 20 以下を維持（`SubscriptionEngine` 統合により +0）
- 合計工数: +3 日
