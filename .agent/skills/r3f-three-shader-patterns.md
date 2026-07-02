---
name: r3f-three-shader-patterns
description: Biome UI（R3F v9 / Three.js シェーダー）を編集するときに読む。Canvas 透過・DPR・instanceColor・格子更新の落とし穴。CSS レイアウトのみなら css-architecture を参照。
---

# R3F / Three.js シェーダー開発パターン

Biome UI（`src/lib/biome/BiomeCanvas.tsx`, `shaders/biomeCell.ts`, `libs/biome-engine/src/grid.rs`）向けの落とし穴集です。Canvas 介入時は `.agent/skills/docs-ui-ux-golden-rules.md` の **U-005**（`useFrame` 内 `setState` 禁止）も併読してください。

## 発動条件

- `BiomeCanvas.tsx` / カスタムシェーダー / `BiomeCellGrid` の変更
- Canvas 背景のグレー化・高 DPR での投影ズレ・シェーダ compile エラー
- セルオートマトン更新で「芋づる式拡散」が発生

## 手順

1. **Canvas alpha**: `gl={{ alpha: false }}` を明示（R3F v9 デフォルト `alpha: true` は CSS 背景と不要合成でグレー化）。
2. **DPR + orthographic**: `dpr={[1, 2]}` を指定し、カメラは Canvas の `orthographic` + `camera={{ left, right, top, bottom }}` を使う。`manual: true` は projection 更新がスキップされ高 DPR でズレる。
3. **instanceColor**: 頂点シェーダーで `#ifndef USE_INSTANCING_COLOR` ガード付きで `attribute vec3 instanceColor` を宣言。
4. **ダブルバッファリング**: `grid.rs` の `tick()` と同様、読み取りは `current_cells`、書き込みは `next_cells`。同一バッファへの in-place 拡散は禁止。
5. **検証**: `BiomeCanvas.test.tsx` と `npm test -- --run src/lib/biome/` を実行。

## 良い例 / 悪い例

### 良い例 — BiomeCanvas 現行パターン

```tsx
<Canvas gl={{ antialias: true, alpha: false }} dpr={[1, 2]} orthographic camera={{ left: 0, right: 128, top: 128, bottom: 0 }}>
```

```glsl
#ifndef USE_INSTANCING_COLOR
  attribute vec3 instanceColor;
#endif
```

### 悪い例 — マクロガード欠如（redefinition）

```glsl
// ❌ Three.js が USE_INSTANCING_COLOR 時に自動注入 → 重複定義
attribute vec3 instanceColor;
```

> 出典: memory/2026-06-30.md Lessons「R3F v9 alpha:true デフォルトでグレー合成」「manual:true で projection 更新スキップ」、`.context/RIPPLE_MAP.md` 2026-06-30 節「instanceColor redefinition / ダブルバッファリング拡散」

## 完了条件

- `cd apps/management-console && npm test -- --run src/lib/biome/BiomeCanvas.test.tsx` が GREEN
- ライト/ダーク切替で Biome Canvas の背景グレー化がないこと（目視）
- **Negative Test**: `biomeCell.ts` の `#ifndef USE_INSTANCING_COLOR` ブロックを一時削除し、`BiomeCanvas.test.tsx` が RED になることを確認してから復元
- **Revert**: 復元後に再実行し、再び GREEN であること
