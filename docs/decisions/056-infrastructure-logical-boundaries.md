# ADR-056: Infrastructure 論理境界（セクション可視化・物理分割しない）

**Status**: Accepted  
**Date**: 2026-07-22  
**Deciders**: motivationstudio  
**Related**: OP-091, [`evolutionary_architecture_plan.md`](../roadmaps/evolutionary_architecture_plan.md) §7, ADR-003, ADR-031（ISP 延期・本 ADR で再オープンしない）

## Context

`libs/infrastructure/src/lib.rs` は `pub mod` が約 97 あり、個別 doc コメントはあるが Bounded Context の見出しがない。F-4（本番 `.rs` 行数）で巨大モジュールが可視化されても、物理クレート分割の ROI は低い（brain `refactoring_value_analysis.md`）。

## Decision

1. **`lib.rs` に固定セクション見出しのみを追加**し、モジュールを論理グループで並べる。公開 API・クレート境界は変えない。
2. **物理分割（クレート分割一括）は行わない**。skills 再分割（OP-050 済）と JobQueue ISP（ADR-031）も本 ADR の対象外。
3. **新規 `pub mod` を追加する PR は、いずれかのセクション配下に置く**ことをレビュー規約とする（セクション外の orphan 追加を禁止）。
4. F-4 の分割**候補**を記録するが、実装しない。

### セクション定義

| セクション | 意図 |
|---|---|
| Security | 認可隣接・免疫・境界検証・出力フィルタ |
| Economy | Gig / 課金隣接ゲートウェイ・生成課金周辺（Commerce 本体は別クレート） |
| Soul-adapters | Soul トレイトの具象・永続・転生 |
| Skills / Tools | WASM スキル・Capability・Arena |
| Observability | 診断・監査・SLO・アラート |
| Cortex / Knowledge | 文書投影・索引・コンテキスト |
| Channels | 外部チャット・投稿・トレンド |
| Workflow / JobQueue | タスク・ジョブ・ワークフロー・監督 |
| Platform | DB・LLM・LoRA・共通 I/O |

### F-4 分割候補（実装しない・2026-07-22 実測ベース）

| 行数帯 | 例 | メモ |
|---|---|---|
| ≥1500 | `workflow/mod.rs` | 将来の論理サブモジュール検討のみ |
| ≥1000 | `society_of_thought.rs`, `lora_marketplace.rs`, `lora_training.rs` | ドメイン単位の読みやすさ改善候補 |
| ≥900 | `api-server` `core_services.rs` / `tool_call_router.rs` | infra 外だが F-4 レポートに出る |

## Consequences

- **Good**: 新規モジュール配置の指針が明確。大規模リファクタなしでナビゲート性向上。
- **Good**: ADR-031 / OP-050 と衝突しない。
- **Bad**: セクションは文書的契約でありコンパイラは強制しない → PR レビューで担保。
- **Mitigation**: OP-090 `architecture_fitness.py` の F-4 でサイズ回帰を観測。
