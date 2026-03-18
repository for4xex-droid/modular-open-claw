# ADR-001: 3層 Soul Architecture

**Status**: Accepted  
**Date**: 2026-03-14  
**Deciders**: motivationstudio

## Context

AIエージェントの「人格」をどう設計するか。単純なプロンプトエンジニアリングだけでは、環境の変化に応じた動的な成長や、過去の経験に基づく直感的な判断ができない。

## Decision

神経科学の3層脳モデル（爬虫類脳/哺乳類脳/新皮質）に着想を得た3層アーキテクチャを採用:

| 層 | 名称 | 役割 | 対応モジュール |
|:---:|---|---|---|
| L1 | Reactive | 即座の反射防衛・感情的な刻印 | `defense.rs`, `somatic.rs` |
| L2 | Deliberative | 予測モデルの更新・可塑性 | `predictive.rs`, `attachment.rs` |
| L3 | Meta-cognitive | 本能の蒸留・転生 | `instinct.rs`, `engine.rs`, `anamnesis.rs` |

## Consequences

- **Good**: 各層が独立しており、段階的に機能を追加できる
- **Good**: `soul` クレートはドメイン非依存（infrastructure に依存しない）
- **Bad**: パイプラインが複雑になり、テストの Dummy 実装が増える
- **Mitigation**: トレイト境界でインターフェースを固定（→ ADR-003）
