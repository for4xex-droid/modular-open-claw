# ADR-047: Biome 構造美ベースのレアリティと Prismatic セル

- **Status**: Accepted
- **Date**: 2026-07-04

## Context

Biome のレアリティ判定が「活性セル数 × 形態種数」のみであり、等方拡散により視覚的に単調なブロブに収束していた。32次元ゲノムは突然変異するが挙動に影響せず、収集体験としての「珍しさ」が欠如していた。

## Decision

1. **差動拡散**: 元素別拡散率 + ゲノム座 0–7（保持力）/ 12–15（異方性）でチューリング型パターンを創発させる。座 8 は IceAge 耐性（`crisis.rs`）のため使用しない。
2. **構造美指標**: `pattern.rs` で symmetry / complexity / cluster_count を計算し、Epic 以上の到達に構造条件を必須化する。
3. **Prismatic マーカー**: 新 struct フィールドではなくゲノム座 31 ≥ 60000 をマーカーとし、IndexedDB 既存セーブとの serde 互換を維持する。render buffer の active スロットは 0/1/2（Prismatic）で後方互換。
4. **API 変更なし**: `morphology_distribution` は既存 API カラムを FE から送信するのみ。

## Consequences

- 珍しい形＝高 symmetry/complexity＝視覚的に映える、が定義レベルで一致する。
- レアリティ閾値変更に伴い HUD 表示分母（400+/800+）と Jest mock を更新済み。
- `cargo test -p biome-engine` 50件 + Jest biome 50件 PASS。
