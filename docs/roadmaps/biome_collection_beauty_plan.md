# Biome 収集体験改善計画 v2 — 「珍しい＝視覚的に楽しい＝アルゴリズム的に美しい」

- **ステータス**: 実装済み（2026-07-04）
- **作成日**: 2026-07-04（v2: 同日 /perfect-plan 5ゲート検証を反映）
- **目的**: Biome の収集要素を、セル多様性と形状的アルゴリズムのバランス改善によって面白くする。

---

## 0. /perfect-plan 検証で確定した前提（実コード検証済み）

v1 からの主な修正点。以下は grep/read で確認済みの事実。

| # | 検証結果 | 計画への反映 |
|---|---|---|
| V-1 | 🔴 **ゲノム座 8 は `crisis.rs` L58 が IceAge 耐性として既に使用** | 異方性座を **12–15** に変更（v1 の 8–11 は衝突） |
| V-2 | `RarityProgress` の Rust 外部参照ゼロ（infrastructure / api-server になし）。TS は `useBiomeEngine.ts` L10–21 の interface と `BiomeHUD.tsx` L154–177 のみ | フィールド追加は安全。更新対象はこの2ファイル＋mock |
| V-3 | `BiomeComponents.test.tsx` L34–55 が `condition_active_500` mock と UI 文言「活性セル 500+」「特殊形態 3種類+」をアサート。`BiomeGame.test.tsx` L69–79 が RarityProgress を mock | 閾値変更時にこの2テストの更新が必須（B-6 に明記） |
| V-4 | `BiomeEvent` に TS 側 exhaustive check なし。分岐は `BiomeGame.tsx` L185–194 の if チェーンのみ（`MorphologyChanged` だけ処理）。`MassExtinction` / `NewReactionDiscovered` は Rust 側でも push されていない死にコード | `PrismaticBorn` 追加は if チェーン＋union 型（`useBiomeEngine.ts` L24–27）の2箇所追加で安全 |
| V-5 | ゲノム座 31 の参照はリポジトリ全体でゼロ。凍結セルは変異ブロック全体スキップ（`grid.rs` L217–232、テスト L401–419） | Prismatic マーカー座 31 は安全。IceAge 凍結で輝き保存が可能（仕様として成立） |
| V-6 | API `rarity` は `String`（`biome.rs` L37）で enum 検証なし。specimens struct に `deny_unknown_fields` なし。`morphology_distribution` / `discovered_reactions` は `Option<String>` で受付済み（L39–40） | API/DB 変更は**一切不要**。FE から送るだけ |
| V-7 | `dream_state/biome.rs` L46–49 は `engine.tick()` / `get_rarity()` 経由のみ。`dream_state/tests.rs` L1050 は `debug_force_rarity` 使用 | レアリティ再設計の波及は dream_state に**表示文字列のみ**（L109–115）。テストは force なので不変 |
| V-8 | `BiomeRenderer.tsx`（legacy 754行）は本番 import ゼロ（テストのみ） | **改修対象外**と確定。テストも active=1.0 直書きで影響なし |
| V-9 | `particle.rs` はゲノム・レアリティ未使用（Fe 濃度のみ） | 変更不要。スコープ外 |
| V-10 | active スロット（offset+2）の読取は `useBiomeEngine.ts` L276（`!== 0`）と `BiomeCellGrid.tsx` L149（`< 0.5`）のみ | Prismatic=2.0 は両方で後方互換 ✅ |

---

## 1. 現状診断（v1 から変更なし・コード検証済み）

| # | 課題 | 根拠 |
|---|---|---|
| P-1 | **形が生まれない**: 全8元素一律 10% 等方拡散 → 円形ブロブに収束 | `grid.rs` L160–204 |
| P-2 | **ゲノムが張りぼて**: 32次元が拡散・反応・描画に一切影響しない | `genome.rs` 全体、tick 内 genome 参照ゼロ |
| P-3 | **レアリティが作業ゲー**: 「セル数 ≥ N」「形態 ≥ M」のみ | `rarity.rs` L102–119 |
| P-4 | **個体レベルの珍しさ不在**: グリッド全体集計のみ | render buffer 仕様 `grid.rs` L273–286 |
| P-5 | **収集物が薄い**: 標本は (64,64) 固定1セルのゲノム＋元素比率のみ。`morphology_distribution` は API 対応済みなのに未送信 | `BiomeGame.tsx` L514–526 |

**テーゼ**: 差動拡散（チューリングパターンの数学的必要条件）で「形」を創発させ、その構造美（対称性・複雑度）を計測してレアリティに直結させる。珍しい＝美しい＝アルゴリズム的に正しい、が定義レベルで一致する。

---

## 2. 実装項目（このまま実行可能なレベル）

### B-1: 差動・異方性拡散 — `grid.rs` / `genome.rs`

**`genome.rs` に追加:**

```rust
// ゲノム座の意味付け（既存: 0–7 元素適応, 8 IceAge耐性 [crisis.rs L58 使用中]）
pub const LOCUS_ANISO_N: usize = 12;   // 北方向拡散重み
pub const LOCUS_ANISO_E: usize = 13;
pub const LOCUS_ANISO_S: usize = 14;
pub const LOCUS_ANISO_W: usize = 15;
pub const LOCUS_PRISMATIC: usize = 31; // B-4 マーカー

impl CellGenome {
    /// 元素 e の実効拡散率変調 (0.5–1.5, 10000 が中立)
    pub fn retention_factor(&self, e: usize) -> f32 {
        1.5 - (self.values[e] as f32 / 65535.0)  // 座 0–7 流用: 高適応=保持
    }
    /// 方向重み [N, E, S, W]（正規化済み、各 0.1 下限）
    pub fn anisotropy(&self) -> [f32; 4] { /* 座 12–15 を正規化 */ }
    pub fn is_prismatic(&self) -> bool { self.values[LOCUS_PRISMATIC] >= 60000 }
    pub fn set_prismatic(&mut self) { self.values[LOCUS_PRISMATIC] = 65535; }
}
```

**`grid.rs` tick の拡散部（L160–204）を変更:**

1. 元素別基本拡散率 `const DIFFUSION_RATES: [u16; 8] = [6, 12, 5, 14, 10, 7, 3, 4];`（% 単位。活性系 C/P 遅、抑制系 N/H 速 = チューリング条件）
2. `spread_amount = amount * rate / 100` に `retention_factor` を乗算
3. 近傍配分を均等割りから `anisotropy()` の重み配分に変更（既存の rand_factor 80–120% 揺らぎは温存 → 決定論テスト互換）

- **互換性**: `BiomeCell` serde スキーマ不変。初期ゲノム全座 10000 → anisotropy 均等・retention 中立 ≈ 従来挙動に近い連続性
- **テスト**: 既存 `test_all_elements_diffusion` / `test_deterministic_behavior` / `test_frozen_cells_*` PASS 維持。新規: 「座 12 を 60000 にしたセルは北への拡散量が最大」

### B-2: パターン美計測 — `pattern.rs`（新規モジュール、`lib.rs` L9–15 に `pub mod pattern;` 追加）

```rust
pub struct PatternMetrics {
    pub symmetry_score: f32,    // 0–1: 重心軸の H/V 鏡映一致率の max
    pub complexity_score: f32,  // 0–1: 1 − 4πA/P²（等周不等式。円盤=0）
    pub cluster_count: u16,     // 4近傍 flood fill 連結成分数
}
pub fn measure(cells: &[BiomeCell]) -> PatternMetrics
```

- 呼び出しは `determine_rarity_with_progress()` 内のみ（BiomeGame が10世代ごと取得 L178–181 → 追加負荷実質ゼロ）
- **テスト**: 十字型 → symmetry ≈ 1.0 / 円盤 → complexity < 0.2 / 市松 → cluster 多数、を固定値検証

### B-3: レアリティ再設計 — `rarity.rs`

```text
Legendary: active ≥ 800 AND 形態 ≥ 4 AND homeostasis
           AND (symmetry ≥ 0.80 OR complexity ≥ 0.85) AND prismatic ≥ 2
Epic:      active ≥ 400 AND 形態 ≥ 3
           AND (symmetry ≥ 0.65 OR complexity ≥ 0.70 OR prismatic ≥ 1)
Rare:      active ≥ 100 AND (形態 ≥ 2 OR symmetry ≥ 0.50)
Uncommon:  active ≥ 10
```

`RarityProgress` へ**追加**（V-2 により既存フィールド削除禁止・追加は安全）:
`symmetry_score: f32`, `complexity_score: f32`, `cluster_count: u16`, `prismatic_cells: u16`, `condition_structure: bool`, `condition_prismatic: bool`

- 既存 `condition_active_500` / `condition_active_1000` は**フィールド名を維持したまま**閾値 400/800 の判定結果を格納する（HUD 互換のため。名前変更は TS 3ファイル波及するため不採用）→ HUD の分母表示のみ更新（B-6）
- dream_state への波及は表示文字列のみ（V-7）。`debug_force_rarity` テストは不変
- **テスト**: `test_legendary_criteria`（rarity.rs L158–197）を対称配置＋prismatic 2個でセットアップし直す。Negative 新規: 「非対称 1000 セルブロブ＋prismatic 0 → Epic 以下」

### B-4: Prismatic セル — `grid.rs` / `lib.rs`

1. tick の変異処理（`grid.rs` L229–234）内で、変異当選セルがさらに `rng.gen_range(0..64) == 0` なら `genome.set_prismatic()`
2. render buffer: `render_buffer[offset+2] = if prismatic { 2.0 } else { 1.0 }`（V-10 により後方互換）
3. `BiomeEvent::PrismaticBorn { x: u16, y: u16 }` を `lib.rs` L100–104 の enum に追加し、tick 比較ループ（L149–156）で prismatic 遷移を検知して push
4. 期待値: 205/65536 × 1/64 ≈ 0.005%/セル/tick → 活性500 × 200世代 ≈ **5個/ラン**（boost 2.0 で ≈10個）
5. IceAge 凍結セルは変異スキップ（V-5）→ 「凍結で輝きを保存する」戦略が仕様として成立

- **テスト**: 固定シードで出現決定論。Negative: mutation_rate 0 → 出現ゼロ。旧セーブ（座31 ≈ 10000）→ `is_prismatic() == false`

### B-5: 視覚表現 — `BiomeCellGrid.tsx`（親エージェント担当）

1. `useFrame` ループ（L145–183）: `renderView[offset + 2] > 1.5` のセルは `computeElementColor` の結果を虹彩 HSL（`(time * 0.15 + i * 0.01) % 1.0` を hue に）で上書き ＋ スケールパルス `scale * (1.0 + 0.15 * sin(time * 3 + i))`
2. `BiomePostEffects.tsx`: props に `structureBonus: boolean` を追加し bloom intensity を一段強化（`BiomeGame.tsx` から `symmetry_score >= 0.65` で渡す）
3. シェーダー（`biomeCell.ts` L102–112 の Epic/Legendary 光沢）は不変

### B-6: HUD / Result / payload / テスト同期（低トークンサブエージェント委譲）

計画確定済みのため機械的に実行可能。対象と変更内容:

| ファイル | 変更 |
|---|---|
| `useBiomeEngine.ts` L11–21 | `RarityProgress` interface に B-3 の6フィールド追加。`BiomeEvent` union（L24–27）に `PrismaticBorn` 追加 |
| `BiomeHUD.tsx` L142–180 | チェックリストに `condition_structure` / `condition_prismatic` バッジ、symmetry/complexity 数値表示。既存分母 500→400, 1000→800 に文言更新 |
| `BiomeGame.tsx` L185–194 | if チェーンに `PrismaticBorn` → Toast（✨ アイコン）追加 |
| `BiomeGame.tsx` L518–526 | specimenPayload に `morphology_distribution` を追加送信（API 対応済み V-6、FE 集計は renderView から morph カウント） |
| `BiomeResult.tsx` | props に metrics 追加、L136–151 の形態分布表示を有効化 |
| `BiomeGame.test.tsx` L69–79 | mock RarityProgress に新フィールド追加 |
| `BiomeComponents.test.tsx` L34–55 | mock＋文言アサート（「活性セル 500+」→「400+」等）更新 |
| `i18n/{ja,en}.json` | 新キー約8個（prismatic Toast、HUD ラベル） |

**スコープ外と確定**: `BiomeRenderer.tsx`（legacy、本番 import ゼロ V-8）、`particle.rs`（V-9）、API/DB（変更不要 V-6）

### B-7: ビルド・検証

```bash
cargo test -p biome-engine                                      # 1. Rust 単体
cd libs/biome-engine && wasm-pack build --target web --out-dir pkg  # 2. WASM（CI と同一: ci.yml L127–129）
cd apps/management-console && npm test -- --testPathPattern=biome   # 3. Jest（vitest ではない）
```

**バランスゲート（ネイティブテストとして実装）**: シード10種 × 200 tick で
- symmetry / complexity がシード間で分散（全シード同値 = 指標死亡）
- Prismatic 出現 0–15 個
- Epic+ 到達がシードの半数以下

### B-8: ドキュメント同期

CHANGELOG [Unreleased] / RIPPLE_MAP（pattern.rs 追加）/ ADR（レアリティ再設計と Prismatic マーカー方式の判断記録）

---

## 3. 検証プロトコル（3段階・AGENTS.md 準拠）

1. **Positive**: B-7 全 PASS ＋ ブラウザで縞/樹状/Prismatic の目視確認
2. **Negative**: 変異率0→Prismatic ゼロ / 非対称ブロブ→Epic 止まり / 旧セーブ→誤判定なし / テスト意図破壊（閾値を故意に壊してテストが落ちること）
3. **Revert & Report**: 注入異常を復元し3段階を報告

---

## 4. トークン運用（委譲マップ）

| 作業 | 担当 |
|---|---|
| B-1〜B-4（Rust コア）、B-5、B-7 検証、B-8 | 親エージェント |
| B-6 全体（表の8ファイル、変更内容確定済み） | **composer-fast サブエージェント** |

---

## 5. 実行順序

```
B-1 → B-2 → B-3 → B-4 → cargo test（Rust 一括）
  → wasm-pack build
  → B-5（親）∥ B-6（サブエージェント並行）
  → B-7 Jest + バランスゲート → B-8
```

依存: B-3 は B-2/B-4 の指標に依存 → ただし同一クレート内なので一括実装・一括テストで問題なし。B-6 は B-3 のフィールド名確定に依存（本計画で確定済みのため並行可能）。

---

## 6. リスクと対策（v2 更新）

| リスク | 対策 |
|---|---|
| ~~ゲノム座 8–11 衝突~~ | **v2 で解消**: crisis.rs の座 8 使用を確認し 12–15 に変更（V-1） |
| IndexedDB 既存セーブ破壊 | struct 無変更・マーカー方式（V-5 で座 31 未使用確認済み） |
| FE テスト暗黙依存 | V-3 で対象を特定済み（BiomeComponents / BiomeGame の2テスト） |
| バランス崩壊 | B-7 シード分散ゲートで数値判定 |
| render buffer 退行 | V-10 で全読取箇所を確認、0/1/2 後方互換 |
| wasm-pack ビルド忘れ | B-7 手順化＋ pkg/ 差分をコミットに含める |
