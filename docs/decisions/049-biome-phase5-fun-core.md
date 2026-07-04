# ADR-049: Biome Phase 5 — 面白さの核

- **Status**: Accepted
- **Date**: 2026-07-05

## Context

Biome Lenia 転換（Phase 0–4、ADR-048）完了後も実機計測で「面白くない」ことが判明。真因は4点:

1. **R1 異シード収束**: 全シードが同一リングスタンプに収束し、種の多様性がゼロ（異シード8個→同一パターン）。
2. **R2 動く生物不在**: リングスタンプは Orbium 等のソリトンではなく、自律移動する「生き物」が存在しない。
3. **R3 クリック無意味**: 種まき以外の操作が結果に影響せず、プレイヤー介入が報われない。
4. **R4 レアリティ自動最大化**: 放置で場全体が広がるテクスチャが Epic 判定される（Orbium 200世代放置→Epic）。

## Decision

1. **正典ソリトン種ライブラリ**: Chakazul/Lenia `animals.json` 由来の5種 RLE を `species_library.rs` に静的登録。`decode_rle`（LeniaND.py 準拠）+ `select_species(seed)` でシードごとに決定的に異なる種を配置（R1 対策）。
2. **マルチ種相互作用**: `LeniaGenome.interaction [[f32;3];3]` を追加し `seed_ecosystem(a,b,competition)` で2種を別 ch に配置。tick に ch 間相互抑制項を追加（R2 対策）。
3. **環境ペン**: `env_mask`（壁/養分/毒）+ `paint_env`/`clear_env` を tick に接続。FE に4ボタン環境ペン UI（R3 対策）。
4. **レアリティ局在性条件**: `bbox_ratio`（活性外接矩形占有率）を導入し Epic 以上に `bbox_ratio < 0.5` を必須化（R4 対策）。
5. **shake 除去**: クリック時の `transform:translate` ジッターを廃止し box-shadow パルスに置換。

## Rationale

| 対策 | PoC / 実測 |
|---|---|
| 種ライブラリ | 異シード8個→一意種5種。RLE デコード・不正入力フォールバック・species fit テスト PASS |
| マルチ種 | エコシステム active=1011（2種共存）。強競合全滅・相互作用ゼロ時従来同一性テスト PASS |
| 環境ペン | 壁描画で active 208→0。空マスク不変テスト PASS |
| bbox_ratio | Orbium 200世代放置: Epic→Uncommon。局在テクスチャ Rare 上限テスト PASS |
| shake 除去 | 枠ジッター解消（視覚的確認） |

Rust 75 + Jest 52 + wasm-pack build 全 PASS。

## Consequences

- `LeniaGenome.interaction` は `#[serde(default)]` で後方互換。既定ゼロ行列は Phase 0–4 と同一挙動。
- `dream_state/biome.rs` の公開 API（`tick`, `get_rarity`, `set_mutation_boost`, `roll_substance`）シグネチャ不変。
- serialize v2 / IndexedDB v2 スキーマ不変。新 WASM メソッド（`seed_ecosystem`, `paint_env`, `clear_env`）は FE から opt-in 利用。
- `species/*.rle` は Chakazul 正典データの静的同梱。外部 CDN 依存なし。

## Alternatives Considered

- **Particle Life への再転換**: Phase 0 で Lenia 選定済み（ADR-048）。連続場の収集・レアリティ基盤を破棄するコストが大きく却下。
- **プロシージャル種生成**: RLE 正典パターンの視認品質が PoC で劣る。5種ライブラリ + シード決定選択で十分な多様性を確認し却下。
- **レアリティ閾値のみ調整**: bbox_ratio なしでは放置テクスチャの Epic 化を構造的に防げない（R4 実測）ため却下。
