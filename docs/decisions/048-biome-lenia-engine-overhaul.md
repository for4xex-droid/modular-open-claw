# ADR-048: Biome Lenia エンジン全面転換

- **Status**: Accepted
- **Date**: 2026-07-04

## Context

Biome は元素拡散モデルから Lenia 連続場（arXiv:1812.05433）へ転換した。Phase 0–4 完了後も、畳み込みが直接法のまま、3 チャンネルが ch0 複製、巻き戻し履歴が `BiomeCell` ベース、IndexedDB/serialize が v1 元素セーブのまま残っていた。

## Decision

1. **FFT 畳み込み**: `rustfft` row-column 2D FFT（`convolve2d_fft`）を tick パスに接続。直接法はテスト用に温存。
2. **3ch 独立更新**: 各 RGB チャンネルが独立した μ/σ で Lenia 成長。描画は 3ch 色差をそのまま利用。
3. **履歴再設計**: `BiomeHistory` を `LeniaSnapshot`（`Vec<f32>` 場 + longevity + 重心）キーフレームのみに変更。最大 100 世代。`apply_tachyon_rewind` は場を復元。
4. **serialize v2**: `{ version: 2, field, genome, longevity, ... }`。v1 `{ cells }` は世代・boost のみ復元し Lenia 場は新規シード維持（非互換明示）。
5. **IndexedDB v2**: `biome_db` version 2。アップグレード時に `engine_states` を再作成し v1 セーブを破棄。

## Consequences

- 巻き戻しは最大 100 世代、場スナップショット ≈ 256 KiB/entry。
- 旧 IndexedDB / v1 JSON セーブは自動的に無効化（開発段階の許容範囲）。
- `dream_state/biome.rs` の公開 API（`tick`, `get_rarity`, `roll_substance`）は不変。
- Rust 63 テスト + Jest biome テスト PASS。
