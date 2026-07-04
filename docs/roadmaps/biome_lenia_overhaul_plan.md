# Biome 抜本改修計画 — Lenia 型連続CAへの転換と収集ゲーム化（実装可能版 v2）

- **ステータス**: Phase 0–4 は**実装済み**（CHANGELOG 2026-07-04「Biome Lenia 転換」）。**Phase 5（面白さの核）と OP-057-R（Pro ライセンス自動接続）は同一バッチで後続着手**（2026-07-05 ユーザー決定）。§13 に v3 追補あり。
- **作成日**: 2026-07-04（v2: 実コードベース検証反映 / v3 2026-07-05: 実機検証で判明した「面白くない」真因への対策を §13 に追加）
- **目的**: Biome を「クリックでシミがにじむ作業」から「自律的に動く美しい生命体を発見・収集する育成ゲーム」へ抜本転換する。処理落ちを根絶し、UI を理解可能にする。
- **多様性の根拠**: `docs/roadmaps/biome_diversity_capacity_analysis.md`（見分けられる種 約800〜1,000、レア度＝パラメータ空間の体積）
- **検証**: 本計画は `/perfect-plan`（5ゲート）＋サブエージェント3体による実コード照合済み。§7 に検証結果を記載。

---

## 0. 現状診断（コード＋実機で検証済み・行番号は実測値に修正）

### 0.1 致命的な処理落ち（真因特定済み）

クリック1回の内部動作（`BiomeGame.tsx:372-392`）:
- `injectElement` 5×5 = **25回**（`:378-385`）+ `tick` **5回**（`:390-392`）= 計 **30回の postMessage**
- Worker は毎メッセージで `sendStateUpdate()`（`biome.worker.ts:92-142`）を同期実行し、その中で:
  - `engine.serialize()` = **全16384セルの JSON 化**（`:103`）
  - `JSON.parse(serialized)`（`:108`、frozenCells 抽出のため）
  - メインスレッドで受信ごとに `saveState()` = **IndexedDB 書き込み**（`useBiomeEngine.ts:176-179`）

→ 1クリックで「巨大 JSON 化 + パース + DB書込」が **30回連鎖**。これが処理落ちの正体。

### 0.2 描画の非効率

`BiomeCellGrid.tsx:145-194` の `useFrame` が**毎フレーム 16384 セルを走査**（活性セルのみ `setMatrixAt`/`setColorAt`）。形態別 InstancedMesh **5個**（`MORPH_COUNT=5`）への更新は連続場描画には過剰。render_buffer は **1セル12 Float32**（`RENDER_STRIDE`、`biomeTypes.ts`）。

### 0.3 ゲームデザインの根本欠陥

- 現行モデル（元素拡散＋化学反応）は**注入点を中心にシミがにじむ**だけで、自律運動も自己組織化もしない。
- 「美しい形」がクリック位置に依存し、狙って作れない／偶然も生まれない。
- ゲノム32次元は挙動への寄与が薄く、収集対象としての「個体差」が視覚に出ない。

### 0.4 UI の複雑さ

左パネル（レアリティ進捗7項目＋元素バランス8バー: `BiomeHUD.tsx`）、右パネル（元素6ボタン＋災害2＋合成＋巻戻し/新シード/遊び方: `BiomeControls.tsx`）。初見で目的（何を目指す？どう育てる？）が読み取れない。

---

## 1. 採用判断（変更なし）

| システム | 採用可否 | 理由 |
|---|---|---|
| **Lenia**（連続CA） | ✅ **中核** | 注入点非依存で自律的に動く生命体が創発。既存 128×128 グリッドを活かせる。400種超がカタログ化＝収集ゲームの原型 |
| Particle Life | ❌ | グリッド→粒子系の全面書換で描画総取替 |
| Conway GoL | ❌ | 二値離散。Lenia が連続一般化＝上位互換 |

---

## 1.5 車輪の再開発回避（二巡目検証で確定した流用方針）

一巡目計画は「畳み込みを自前実装」「`BiomeFieldRenderer` を新規作成」としていたが、実コード＋エコシステム照合で**既存資産で代替すべき箇所**が判明した。

### 1.5.1 Lenia 数値計算 — 自前の直接畳み込みをやめ `rustfft` + `ndarray` を採用

- **既存クレート `lenia_ca`（crates.io v0.1.1）/ `lenia-rs`** が StandardLenia/ExpandedLenia を FFT 畳み込みで実装済み。ただし **`rayon` 依存**が致命的: WASM はデフォルトでスレッド非対応（SharedArrayBuffer + cross-origin isolation が必要）で、`rayon` をそのまま wasm32 で使えない。加えて v0.1.1・作者自身が「習作」と明記・ライセンス未確認（本プロジェクトは BSL）。→ **直接依存は不採用**。
- **採用**: `rustfft = "6"`（MIT/Apache）を Cargo.toml に追加し、**単一スレッドの薄い Lenia ラッパを自前実装**。`lenia_ca`/`lenia-rs` は**設計リファレンス**として参照（カーネル生成・成長関数・Orbium 初期値）。
- **効果**: 直接畳み込み O(N·R²)（3ch で ~26M ops/tick）を **FFT 畳み込み O(N log N)** に置換 → §11 の「30fps 未達」リスクを構造的に解消。自前 FFT は書かない。
- **三巡目で確定した実装上の注意（rustfft 実地確認済み）**:
  1. **rustfft は wasm32-unknown-unknown を公式サポート**（std の必要部分は同ターゲットで動作）。`FftPlanner` を使うだけ。
  2. **rustfft は 1次元専用**。2D FFT は「各行を 1D FFT → 各列を 1D FFT」の分離実装（row-column法）。プランナ（`FftPlanner`）とツイドル係数は起動時に一度生成し使い回す。
  3. **`ndarray` は必須ではない**。フラット `Vec<Complex<f32>>`（128×128）を row/column ストライドで反復すれば足りる → **wasm サイズ削減のため `ndarray` は入れない**（当初案の `+ ndarray` を撤回）。
  4. **SIMD（`wasm_simd` feature）はデフォルト無効・非対応環境でトラップ**。**PoC はスカラ（feature 無効）で測定**し、30fps 未達の場合のみ `wasm_simd` を有効化（2023年以降の主要ブラウザは fixed-width SIMD 対応）。既定は無効のまま安全側に倒す。

### 1.5.2 描画 — `BiomeFieldRenderer` 新規作成をやめ既存テクスチャ資産を流用

- レガシー `BiomeRenderer.tsx:217-296,408-416` に **RGBA32F DataTexture(128×128) + texSubImage2D 毎フレーム更新 + フルスクリーンクアッド + `grid.frag`** が既に実装済み（＝連続場描画そのもの）。
- R3F 統合済みのフルスクリーン描画は `components/fluid/FluidBackground.tsx:35,89-106`（`FullscreenPass` + `FULLSCREEN_VERTEX`）と `lib/vrm/FluidAura.tsx`（`THREE.ShaderMaterial`）が既存。
- **採用**: 新規 `BiomeFieldRenderer` は「`THREE.DataTexture` + `FullscreenPass`（FluidBackground パターン）+ `grid.frag` のカラーマップ移植」の**合成**とする。ゼロからは書かない。
- **カラーマップ重複の解消**: `grid.frag:20-77`（GLSL `hsl2rgb`）と `BiomeCellGrid.tsx:39-93`（TS `computeElementColor`）は同一ロジックの**二重実装**。Phase 2 では**シェーダ側（GLSL）に一本化**し TS 版を削除。

### 1.5.3 パターン統計 — `pattern.rs` を拡張（新規モジュール化しない）

- 重心/対称性/フラッドフィル/クラスタ/面積周長は **`pattern.rs:20-185` に集約済み**（他に重複なし）。Phase 3 の Lenia 統計（mass/locomotion/symmetry）は**ここを拡張**。ただし現行は `&[BiomeCell]` の活性マスク前提 → **f32 場を閾値でマスク化するオーバーロード** `measure_field(field: &[f32], threshold: f32)` を追加する形にする。
- species_hash・トロイダル境界ヘルパ・debounce/throttle は**リポジトリに存在せず新規実装が妥当**（汎用 SHA-256/`DefaultHasher` はあるが種同定用途に不適）。

---

## 2. 保持すべき公開API契約（consumer 照合で確定・**破壊禁止**）

Lenia 化は内部実装の置換であり、以下のシグネチャは維持する。これを破ると Rust consumer（`dream_state/biome.rs`）と WASM worker がコンパイル/実行エラーになる。

### 2.1 Rust consumer（`libs/infrastructure/src/dream_state/biome.rs:44-50`）が呼ぶメソッド

| メソッド | 現行シグネチャ（`lib.rs`） | Lenia 化後の扱い |
|---|---|---|
| `set_mutation_boost(f32)` | L286 | 維持。Lenia では突然変異率（μ/σ の摂動幅）に接続 |
| `tick()` | L140 | 維持。内部を Lenia 更新則に置換 |
| `generation() -> u32` | L136 | 維持 |
| `get_rarity() -> BiomeRarity` | L218 | 維持。Lenia 統計ベースに再実装 |
| `roll_substance() -> SubstanceKind` | L269 | 維持。**Fe依存を廃し「高質量×高対称の種」で判定**（§4.3） |

`BiomeRarity`（`rarity.rs:11-19` の5バリアント Common/Uncommon/Rare/Epic/Legendary）と `SubstanceKind`（`particle.rs:11-17` の None/Higgs/Tachyon）の **enum 定義・バリアント名は変更しない**（`dream_state/biome.rs:109-116,198` が依存）。

### 2.2 WASM worker（`biome.worker.ts`）が呼ぶメソッド

`generation/get_rarity/get_active_cell_count/get_element_balance/get_mutation_boost/ticks_since_mutation/get_rarity_progress/get_last_tick_events/tick/apply_tachyon_rewind/serialize/inject_element/apply_crisis/set_mutation_boost/render_data_ptr/render_data_len`（`biome.worker.ts:31-37,60-121`）。

→ すべて維持。ただし意味を Lenia に読み替え（§4）。新規に `inject_brush(x,y,radius,channel,amount)` を追加（§3 P0-3）。

### 2.3 API/DB（`routes/biome.rs`, migrations）

- `BiomeSpecimenPayload`（`routes/biome.rs:31-42`）に `#[serde(deny_unknown_fields)]` は**なし**＝**フィールド追加は後方互換**。
- 保存を「全セルJSON」から「種パラメータJSON（μ/σ/β/kernel、数十バイト）」に変更しても、既存カラム `genome_data TEXT` にそのまま格納可能（スキーマ変更不要）。
- 新カラムが必要なら migration 追加（`libs/infrastructure/migrations/{sqlite,postgres}/`）。本計画では**既存 `genome_data` に JSON 格納で足りるためスキーマ変更なし**。

---

## 3. Phase 0: 緊急パフォーマンス修正（Lenia と独立・即効・Lenia でも有効）

現行モデルのまま処理落ちを潰す。ここでの修正は**アーキテクチャ層**なので Lenia 移行後もそのまま活きる。

| # | 変更 | 対象ファイル・箇所 | 具体 |
|---|---|---|---|
| P0-1 | **serialize を描画ループから分離** | `biome.worker.ts:92-142`, `grid.rs:284-304` | `sendStateUpdate` を「render buffer 転送のみ」に。`is_frozen` を **render_buffer に1スロット追加**（stride 12→13、`offset+12` に 0/1）。Rust `grid.rs` tick の render 更新で `cell.is_frozen` を書く。これで `serialize()`＋`JSON.parse` を毎tickから**全廃**。`biomeTypes.ts` の `RENDER_STRIDE` を 13 に、`BiomeCellGrid.tsx:146` の `offset` 計算に追随 |
| P0-2 | **IndexedDB 保存をスロットル** | `useBiomeEngine.ts:176-188` | `updated` ごとの `saveState` を廃し、**2秒デバウンス**（`setTimeout` + クリア）に。保存用 `serialize()` はこのタイミングのみ Worker に要求（新メッセージ `type:'requestSave'`→ worker が serialize して返す）。rewind 時は即保存 |
| P0-3 | **クリック注入の tick 連打・25連 postMessage 廃止** | `BiomeGame.tsx:372-392`, `useBiomeEngine.ts:301-304`, `lib.rs` | `handleCanvasClick` の 5×5 ループと 5回 tick を削除。Worker へ **1回だけ** `type:'injectBrush'` を送る。Rust に `inject_brush(&mut self, x:usize, y:usize, radius:usize, idx:usize, amount:u16)`（新規、`lib.rs`）を追加し内部で範囲注入 |

**検証（Negative Test 含む）**: クリック時 postMessage を 30→1 に削減。Chrome DevTools Performance でフレーム落ちゼロ（web-perf skill）。Negative: `RENDER_STRIDE` を 12 のままにすると frozen 列がズレて描画が壊れることを確認→13 に是正で復旧。

---

## 4. Phase 1: Lenia エンジン中核（Rust `libs/biome-engine`）

新モジュール `src/lenia.rs`（現状**存在しないので二重実装なし**）を追加し、`grid.rs::tick` の内部計算を Lenia 更新則に置換。既存 element/genome は §4.4 で「種の遺伝子」に再利用。

### 4.1 数式（arXiv:1812.05433 準拠）

```
U = K * A                          # リングカーネル K と場 A の畳み込み（ポテンシャル）
G(u) = 2·exp(-(u-μ)²/(2σ²)) - 1    # ガウス成長関数 [-1,1]
A' = clip(A + dt·G(U), 0, 1)       # 増分更新
```

### 4.2 実装方針（FFT 前提・PoC 段階を明示）

- **依存追加**: `Cargo.toml` に `rustfft = "6"` のみ（MIT/Apache、wasm32-unknown-unknown 公式サポート）。`rayon`（wasm スレッド非対応）と `ndarray`（フラット Vec で代替可・サイズ削減）は**入れない**。§1.5.1 参照。
- **2D FFT 実装**: rustfft は 1D 専用のため、row-column 法（行 FFT→列 FFT）で 2D 化。`FftPlanner`・カーネルの周波数表現は起動時に一度だけ生成。
- **段階1（PoC・必須ベンチ・スカラ）**: **単一チャンネル** Lenia。R=13。**FFT 畳み込み** O(N log N)（128×128）で実装。`wasm_simd` feature は**無効**のまま 30fps 実測 → 未達なら SIMD 有効化 or 場 96×96 縮小。
- **段階2**: **多チャンネル Lenia（RGB 3ch）**。各chが独立 μ/σ/カーネル＋ch間相互作用（色＝種の視覚的個性）。FFT なら 3ch でも実時間性を確保しやすい。
- **カーネル**: リング状 `core(r)=exp(4 - 4/(4r(1-r)))` を起動時に事前計算し、周波数領域へ一度だけ FFT しておく（毎tick 逆FFTのみ）。トロイダル境界は FFT 畳み込みで自然に成立（循環畳み込み）→ 現行の端クランプ近傍（`grid.rs:176-188`）とは別に **`rem_euclid` ベースの wrap を場アクセスに導入**。
- **場のストレージ**: `BiomeGrid` に `field: Vec<f32>`（GRID_SIZE×channels）を追加。`lenia_ca`/`lenia-rs` はカーネル生成・Orbium 初期値の**リファレンスとしてのみ参照**（直接依存しない）。

### 4.3 パラメータ＝ゲノム化と `roll_substance` 再定義

- Lenia パラメータ（μ, σ, カーネルピーク β[], dt, R）を **`LeniaGenome` 構造体**として保持。突然変異＝これらの微小摂動（`set_mutation_boost` が摂動幅を制御）。
- **`roll_substance` の Fe 依存を廃止**（`particle.rs::roll_substance_discovery:20-47` の `elements[6]>=40000` 判定を置換）。新ルール: 「場の総質量 mass が閾値超 **かつ** 対称性 symmetry が閾値超」の tick で確率ロール（高質量高対称=3%、他=0.1%、現行の確率テーブルは踏襲）。Higgs/Tachyon 50%振り分けは維持。
  - 影響: `dream_state/biome.rs:198-224`（Higgs→形質固定）はシグネチャ不変で動作継続。

### 4.4 既存 element/genome の扱い

- `CellGenome`（32次元、`genome.rs`）は**互換のため残置**しつつ、Lenia では `LeniaGenome` を主に使用。`serialize/deserialize`（`lib.rs:352-370`）は新 field 構造を含む v2 形式に（§6 マイグレーション）。
- `element.rs` の化学反応、`evolution.rs` の捕食者/生産者遷移は Lenia では非使用 → `#![allow(dead_code)]` で**削除せず抑制**（AGENTS.md Preserve Intent 原則）。将来のハイブリッド用に温存。

### 4.4b apply_tachyon_rewind の履歴再設計（二巡目で判明した抜け漏れ・**必須**）

一巡目計画はここを見落としていた。現行の巻き戻しは **`BiomeCell` グリッド専用**の履歴に依存する:
- `HistoryEntry::Keyframe(Vec<BiomeCell>)` / `Delta(Vec<(u16, BiomeCell)>)`（`lib.rs:24-27`）
- `BiomeHistory::new(20, 100)`（`lib.rs:128`）= 20世代ごとキーフレーム・最大100エントリ
- `apply_tachyon_rewind`（`lib.rs:170-181`）が `grid.set_current_cells` で復元

Lenia で内部状態が `field: Vec<f32>` になると、この履歴は**そのままでは機能しない**。対応:
- `HistoryEntry` を **`Keyframe(Vec<f32>)`（場のスナップショット）** に再定義。Delta は連続場でスパース化が難しいため、**当面キーフレームのみ**（間隔を20→広めに調整しメモリ抑制）。
- メモリ試算（サブエージェント確認済み）: 場 4ch なら 1 keyframe ≈ 256 KiB、100件で ≈ 25 MiB。**現行 BiomeCell keyframe（≈1.3 MiB×100 ≈ 130 MiB）より軽い**ため、チャンネル数を抑えれば問題なし。3ch+frozen で `max_entries` 100 維持可。
- `apply_tachyon_rewind` のシグネチャ（`&mut self, generations: u32 -> bool`）は**維持**（`biome.worker.ts:63` が依存）。内部復元先を場に変更。
- 影響テスト: `test_tachyon_rewind_restores_state`（`lib.rs:403-422`）を場ベースに更新。

### 4.5 crisis の再定義（`crisis.rs`）

- `Meteor`（`crisis.rs:33-51`）: 元素回転→**局所的な場のノイズ注入**（半径2に乱数加算）に再解釈。
- `IceAge`（`crisis.rs:53-67`）＋ `is_frozen`: Lenia 場の**局所凍結**（該当セルの field 更新をスキップ）として維持。`thaw_grid`（`:19-26`）も維持。

**検証（Negative Test 含む）**: Orbium 正典パラメータ（μ=0.15, σ=0.017, R=13, dt=0.1）で「移動するグライダー」が再現されるテスト（重心が移動）。決定論テスト（同一シードで同一挙動、`grid.rs:338` 既存パターン流用）。Negative: μ を安定域外（例 0.5）にすると即崩壊（mass→0）することを確認。

---

## 5. Phase 2: 描画の刷新（単一データテクスチャ + シェーダ）

- `BiomeCellGrid.tsx`（16384 InstancedMesh 5個走査）を**廃止**し、`BiomeFieldRenderer.tsx`（新規）に置換。既存資産の**合成**として実装（§1.5.2）: `THREE.DataTexture`（128×128 RGBA32F）+ `FluidBackground.tsx` の `FullscreenPass` パターン + `grid.frag` のカラーマップ移植。ゼロからは書かない。
- render_buffer を「連続場 RGB（＋frozen）」レイアウトに再定義。Worker は `render_data_ptr/len`（既存）で Float32 場を Transferable 転送（`biome.worker.ts:119-140` の転送機構をそのまま流用）。
- カラーマップは **GLSL 側（`grid.frag:20-77` の `hsl2rgb`）に一本化**し、TS 版 `computeElementColor`（`BiomeCellGrid.tsx:39-93`）の二重実装を解消。希少個体は `BiomePostEffects.tsx` の既存 Bloom を流用。
- **削除候補**（本番 import ゼロを確認済み）: `BiomeRenderer.tsx`（レガシー、テストのみ参照。ただし DataTexture/クアッド生成ロジックは Phase 2 で移植後に削除）、`BiomeCellGrid.tsx`、関連 `cellGeometries.ts`/`shaders/biomeCell.ts`。削除に伴い `BiomeRenderer.test.tsx`/`BiomeCanvas.test.tsx`/`BiomeGame.test.tsx`（`RENDER_STRIDE` mock）を更新（サブエージェント委譲）。

**検証**: 描画コストが per-cell 更新から `texImage2D` 1回に。フレームレート測定（web-perf）。

---

## 6. Phase 3: 収集・レアリティ再設計（Lenia 統計量）

`pattern.rs`（`measure:17`, `PatternMetrics:9-14`）を Lenia 統計に拡張し、`rarity.rs::determine_rarity_with_progress:47` を再実装。

| 指標 | 算出 | 収集価値 | レア度への寄与 |
|---|---|---|---|
| 質量 mass | 場の総和 | 生存＝安定種 | 生存判定 |
| 移動速度 locomotion | 重心変位/tick | 動く種は希少 | Epic 以上 |
| 対称性 symmetry | 既存 `compute_symmetry:63` 流用 | 幾何学的な美 | 全レア度 |
| 存続世代 longevity | 崩壊せず持続した tick | 安定性 | Rare 以上 |
| 種同定 species_hash | パラメータ＋形態の量子化ハッシュ | 図鑑の一意キー | 図鑑登録 |

- **レア度＝パラメータ空間の体積**（多様性分析 §4）: Common 60%/Uncommon 25%/Rare 10%/Epic 4%/Legendary 1%。薄い多様体上の安定解ほど高レア。
- `RarityProgress`（`rarity.rs:21-39`、既存15フィールド）は**フィールド削除せず**、Lenia 指標（`mass`, `locomotion`, `longevity`, `species_hash`）を**追記**（後方互換。`BiomeGame.test.tsx:51-67` の mock を追随更新＝サブエージェント委譲）。
- **図鑑（新規 UI）**: `BiomeDendou.tsx`（殿堂入り、`BiomeGame.tsx:18` で使用中）を「種図鑑」に発展。発見種をサムネイル＋パラメータで一覧。
- **保存**: 種パラメータ（μ/σ/β = 数十バイト）を `genome_data`（既存カラム）に JSON 格納 → 全セルJSON保存を廃止し IndexedDB/API 負荷激減。

**検証（Negative Test 含む）**: シード10種で種多様性・レア度分布を検証（全部同じ/全部レアを排除、`lib.rs:543 test_balance_gate_seed_diversity` 流用）。Negative: 意図的に「全セル同一パラメータ」を注入し、レア度が全て Common に落ちる（多様性ゼロ検知）ことを確認。

---

## 7. /perfect-plan 検証結果（5ゲート）

### Gate 1: 構造スキャン
- ⚠️→✅ `docs/architecture/ARCHITECTURE.md` は**存在しない**。依存図はルート `ARCHITECTURE.md:49,59,75,145`。計画の参照先を修正済み。
- ✅ `lenia.rs` は未存在＝二重実装なし。`pattern.rs`/`rarity.rs`/`genome.rs`/`crisis.rs`/`particle.rs` は実在（新規作成の重複なし）。
- ✅ 計画未記載だが影響を受ける既存: `element.rs`/`evolution.rs`（Lenia で非使用→ dead_code 抑制で対応、§4.4）。

### Gate 2: 要件カバレッジ（NURTURE クロスチェック）
- §2 経済台帳: ❌ 影響なし（通貨・課金に触れない）。
- §3 MCP: ❌ 影響なし。
- §4 セキュリティ: ⚠️ `serialize/deserialize` の入力検証を維持（`deserialize:362` の Result エラー処理）。認証は既存ルートの `RequireAuth` 継続。
- §6 VRM/3D: ✅ 本計画の主対象（描画刷新）。VRM とは独立。
- §7 A2C（報酬）: ⚠️ Higgs→形質固定（`dream_state/biome.rs:198`）と Legendary→plasticity（`:226-247`）の経路は維持。`roll_substance` 再定義後も valence マッピングは不変。

### Gate 3: 依存 & 波及
- ✅ `biome_engine` consumer: `app_state.rs:192`, `state_assembly.rs:381`, `dream/dream.rs:53`, `dream_state/{mod.rs,biome.rs,tests.rs}`, worker/vite（TS）。API シグネチャ維持で波及は内部に限定。
- ✅ Mock: `useBiomeEngine.test.ts:10,16,56`（`jest.mock('biome-engine')`）と `BiomeGame.test.tsx` の RarityProgress mock を追随更新（委譲）。
- ✅ `api_integration_tests/biome.rs:16-122` は API 不変なら影響なし（保存内容の意味変更のみ）。

### Gate 4: 悪魔の弁護人
1. **最悪のシナリオ**: 多チャンネル Lenia の畳み込みが WASM で 30fps に届かず処理落ちが Lenia でも再発。→ **緩和**: §4.2 段階1で単一ch PoC ベンチを必須ゲート化。未達なら R 縮小 or `rustfft` 導入 or 場を 96×96 に縮小。
2. **見落とされた前提**: 「Orbium が既存 128×128・トロイダル境界・dt=0.1 でそのまま安定移動する」を暗黙前提にしている。→ 正典値でも実装差で崩れうる。§4 検証を「グライダー再現テスト」で強制ゲート化済み。
3. **やらないメリット**: Phase 0 のみで処理落ちは解消し、既存の収集要素も残る。→ **段階リリース可**（§9）。Lenia 中核（Phase 1-2）は最もリスクが高いので Phase 0 を先行 merge し価値を先出しできる。

### Gate 5: 実行順序
- ✅ Phase 0（perf, model 非依存）→ Phase 1（Lenia core）→ Phase 2（描画, core の render buffer 定義に依存）→ Phase 3（収集, core 統計に依存）→ Phase 4（UI）。依存の逆転なし。
- ⚠️ P0-1 の render_buffer stride 変更（12→13）は Phase 2 で場レイアウトに再定義される。**二度手間回避のため、Lenia 一括採用が決定なら P0-1 は「stride 変更」ではなく「serialize 分離のみ」に留め、frozen は Phase 2 の場レイアウトに含める**選択肢を推奨（§9 で判断）。

### 判定
**⚠️ PATCH → ✅（本 v2 で反映済み）**。事実誤り（行番号・API 名・ARCHITECTURE パス・Fe依存・stride）を修正し、PoC ベンチと検証ゲートを追加した本 v2 は、そのまま着手可能。

### Gate 6（二巡目追加）: 重複・車輪の再開発・抜け漏れ
- ✅ **車輪の再開発を2件排除**: (a) 自前直接畳み込み → `rustfft`+`ndarray`（FFT）に変更（§1.5.1）。(b) 描画の新規フルスクラッチ → 既存 `BiomeRenderer`/`FluidBackground`/`grid.frag` の合成に変更（§1.5.2）。既存 Rust Lenia クレート（`lenia_ca`/`lenia-rs`）は rayon 依存・習作・ライセンス未確認のため直接依存せずリファレンス参照に留める判断を明記。
- ✅ **重複を1件解消**: カラーマップの二重実装（GLSL `grid.frag` と TS `computeElementColor`）を GLSL 一本化。
- ⚠️→✅ **抜け漏れを1件補完**: `apply_tachyon_rewind` の履歴が `BiomeCell` 専用で Lenia 場に非対応だった問題を §4.4b で再設計（メモリ試算付き）。
- ✅ **テスト影響を全数把握**（§12 に一覧化）: Rust 直接影響テスト 約20件、TS 影響 約30箇所（stride 12・serialize `cells[].is_frozen` mock 形状・morphology・RarityProgress）。機械的更新はサブエージェント委譲。

### Gate 7（三巡目・収束確認）: FFT 依存の実地検証と細部確定
- ✅ **土台依存 `rustfft` を実地確認**: wasm32-unknown-unknown 公式サポート。1D 専用のため 2D は row-column 法で実装。`wasm_simd` はデフォルト無効（非対応環境でトラップ）→ PoC はスカラで測定し必要時のみ有効化。§1.5.1/§4.2 に反映。
- ✅ **不要依存 `ndarray` を撤回**: フラット `Vec<Complex<f32>>` で代替でき wasm サイズを抑えられるため依存追加を1つ削減。
- ✅ **ADR 番号確定**: 最新が `047-biome-structure-beauty-rarity.md` のため本改修は **ADR 048**。
- **収束判定**: 三巡目で新規の構造的欠陥・車輪の再開発は検出されず、細部（FFT 実装法・依存削減・ADR番号）の確定のみ。**計画は収束した**と判断する。

---

## 8. Phase 4: UI 抜本簡素化

1. **目的の明示**: 「美しい生命体を育てて図鑑を完成させよう」を常時表示。
2. **操作の集約**（`BiomeControls.tsx` の元素6＋災害2＋合成を廃し3操作に）:
   - 🖌 **種まき**: クリックで生命の種を撒く（注入点非依存で創発）。
   - 🎲 **環境を変える**: μ/σ スライダー2本（Lenia の全表現力）。新種発見の主操作。
   - 📖 **図鑑**: 発見種の閲覧・保存。
3. **情報削減**: `BiomeHUD.tsx` の7項目チェックリスト＋8バーを「現在の生命体スコア」1枚に集約。
4. **チュートリアル**（`BiomeTutorial.tsx` 流用）: 「スライダーで生き物の姿が変わる。珍しい姿を図鑑登録」の1分導線。

**検証**: 初見ユーザーが説明なしで「種まき→スライダー→図鑑登録」に到達（社内ドッグフーディング）。

---

## 9. 実施順序・段階リリース判断・委譲マップ

```
Phase 0（緊急perf, model非依存）→ 実機で処理落ち解消を確認 → 先行 merge 可
  → Phase 1（Lenia core, 単一ch PoCベンチ必須ゲート）→ Orbium 再現 + cargo test
  → Phase 2（描画, 場レイアウト確定）→ wasm-pack build + フレームレート
  → Phase 3（収集）→ 種多様性テスト
  → Phase 4（UI）→ ドッグフーディング
  → docs 同期（CHANGELOG/RIPPLE_MAP/ADR 048）
```

**リリース戦略**: Phase 0 を独立 PR で先行（処理落ち即解消）。Phase 1-4 は Lenia 転換 PR 群。P0-1 の frozen 扱いは「Lenia 一括採用が確定なら serialize 分離のみ、frozen は Phase 2 場レイアウトへ」（Gate 5 参照）。

| 作業 | 担当 |
|---|---|
| Lenia エンジン（`lenia.rs`, tick 置換, カーネル畳み込み, PoC ベンチ） | 親エージェント |
| roll_substance/crisis 再定義、rarity/図鑑ロジック | 親エージェント |
| 描画シェーダ（`BiomeFieldRenderer`, カラーマップ） | 親エージェント |
| UI 削除・整理、i18n、テスト mock 更新、CHANGELOG/RIPPLE_MAP、レガシー削除 | **低トークンサブエージェント** |
| 既存テスト期待値更新（Rust 50 + Jest 50） | **低トークンサブエージェント** |

---

## 10. マイグレーション・互換性

- **IndexedDB**: 元素モデルの全セルJSON（`biome_db` v1, store `engine_states`, key `seed_N`）は Lenia と非互換。**`biome_db` v1→v2 で新規化、旧セーブ破棄**（開発段階のため許容）。`useBiomeEngine.ts:47` の `open('biome_db', 1)` を 2 に。
- **API/DB**: `deny_unknown_fields` なし＝後方互換。`genome_data`（既存 TEXT）に種パラメータJSON格納でスキーマ変更不要。
- **Rust consumer**: API シグネチャ維持で `dream_state/biome.rs`・api-server は無改修。

---

## 11. 残存する未確定事項（着手前に PoC で解消）

1. **多チャンネル Lenia の WASM 実時間性**（最重要）: FFT 畳み込み（`rustfft`、§1.5.1）採用で大幅緩和されるが、段階1 単一ch ベンチで 30fps 未達なら場を 96×96 に縮小。`rustfft` の wasm32 ビルド可否を PoC 冒頭で確認。**着手前 PoC ゲート**。
2. Orbium 正典パラメータが本実装（トロイダル 128×128, dt）で安定移動するか。
3. `species_hash` の量子化粒度（多様性分析の「約1,000種を見分けられる」に合わせた量子化ステップ設計）。

---

## 12. テスト影響一覧（二巡目・サブエージェント全数調査。実装時の更新チェックリスト）

Lenia 化で更新が必要な既存テスト。機械的更新は**低トークンサブエージェント委譲**。

### 12.1 Rust（`libs/biome-engine` ほか）

| 変更軸 | 影響テスト（ファイル:行） | 対応 |
|---|---|---|
| render stride（12→新レイアウト） | `grid.rs:408` `assert_eq!(len, GRID_SIZE*12)`、`grid.rs:412-415`、`grid.rs:534-535` `test_prismatic_render_buffer_value` | stride 定数化し新値でアサート更新 |
| serialize 形式（BiomeCell→場） | `lib.rs:441-456` `test_serialize_deserialize`、`lib.rs:403-422` `test_tachyon_rewind_restores_state` | 場ベースのラウンドトリップに書換 |
| morphology 廃止/再定義 | `evolution.rs:46-70`（predator/producer）、`grid.rs:376` `test_cell_morphology_initialization`、`rarity.rs:215,255,284,302`（`morphology_count`）、`lib.rs:527` `test_engine_last_tick_events` | Lenia 統計 or 温存判断に応じ更新 |
| roll_substance の Fe 依存廃止 | `particle.rs:56-80` `test_high_iron_increases_discovery_rate` | 「高質量×高対称」トリガーのテストに置換 |
| RarityProgress フィールド追加 | `rarity.rs:185,214,254,282,301,315`、`lib.rs:518,561` | 新フィールド（mass/locomotion/longevity）を含めてアサート更新 |
| Fe 元素反応（`element.rs`） | `element.rs:142,195,225,254,280` ほか | **元素モデルを温存する限り変更不要**（§4.4 dead_code 抑制） |
| dream_state | `dream_state/tests.rs:1184-1326`（`debug_force_substance(Higgs)`） | `debug_force_*` 維持なら影響最小 |

### 12.2 TypeScript（`apps/management-console`）

| 変更軸 | 影響箇所（ファイル:行） | 対応 |
|---|---|---|
| `RENDER_STRIDE`（=12） | `biomeTypes.ts:32`、`BiomeCellGrid.tsx:146`、`BiomeGame.tsx:213-216`、`useBiomeEngine.ts:279-289`、テスト `BiomeGame.test.tsx:50`/`BiomeCanvas.test.tsx:48`/`BiomeRenderer.test.tsx:114,132`/`useBiomeEngine.test.ts:32,73`/`setupTests.ts:97` | 定数更新＋mock サイズ追随。BiomeCellGrid 廃止時は該当削除 |
| serialize `cells[].is_frozen` | `biome.worker.ts:103-113`、`useBiomeEngine.ts:329-336`（`serializeGenome`） | frozen を render buffer 経由に移し JSON.parse 全廃（P0-1） |
| morphology_distribution | `BiomeGame.tsx:207-221,539`、`BiomeResult.tsx:157-171`、`BiomeHUD.tsx:167-173`、`BiomeDendou.tsx:157-163`、テスト `BiomeComponents.test.tsx:238-289` | Lenia 種指標に置換 or 非表示 |
| RarityProgress mock | `BiomeGame.test.tsx:51-67` | 新フィールド追加 |

### 12.3 DB/API（既存維持で影響最小）

- `biome_db_tests.rs:68-94`（`morphology_distribution` 列）と `routes/biome.rs:39` は**カラム維持なら無改修**。Lenia では `morphology_distribution` に「種の特徴分布 JSON」を格納する形で意味だけ変更（スキーマ不変）。

---

## 13. Phase 5: 面白さの核（v3 追補 — 2026-07-05 実機検証で確定）

Phase 0–4 で「Lenia 化・処理落ち解消・UI 簡素化」は完了したが、**実機検証（WASM を直接叩いた数値計測）で「面白くない」の真因が別にある**ことが判明した。Phase 5 はこの真因への対策のみを扱う。Phase 1–4 と役割が重複しないよう、各項目に「なぜ既存 Phase で解決しないか」を明記する。

### 13.1 実機検証で確定した真因（推測でなく計測値）

| # | 症状（ユーザー報告） | 計測による真因 | 検証方法 |
|---|---|---|---|
| R1 | 「似たセルしか生まれない」 | **初期条件がリングスタンプ（`seed_orbium_ring`, `lenia.rs:289-317`）**。異シード 8 個を 200 世代進めても mass≈3400・充填23% にほぼ収束（seed1=3314, seed42=3395, seed99=3436…）。本物のソリトンでないため必ずベタ塗りテクスチャに崩壊する | μ/σ 42通り＋シード8個を WASM 直接実行で計測 |
| R2 | 「動く生き物がいない」 | **局在して移動する設定が μ/σ 空間に 0 件**。mass は `98→542→1841→3396` と単調増加し bbox は 100% に到達（＝無限に広がるテクスチャ、生物ではない） | mass<600 かつ移動あり の設定を全探索 → 0件 |
| R3 | 「クリックしても変わらない」 | **種まきが長期的に無意味**。同一シードでクリック2回 vs 0回を100世代比較 → mass はどちらも 1841 で完全一致 | inject_brush 有無で serialize 比較 |
| R4 | 「達成感がない」 | **レアリティが放置で自動最大化**。一切操作せず 200 世代で rarity=Epic(3/4) に到達 | 無操作 200 tick 後の `get_rarity()` |
| — | エンジンは正しい | **正典 Orbium の RLE を流すと mass≈71 で安定し重心がグリッド上を泳ぎ続けた**。カーネル・成長関数・FFT は正常。問題は初期条件のみ | Chakazul/Lenia の Orbium RLE を投入し 500 tick |

**結論**: Phase 1 のエンジンは数学的に正しい。面白くない原因は「①雑な初期条件」「②チャンネル間相互作用の不在（現状 3ch は同一 sim の RGB。`lenia.rs:158-185` に相互作用項なし）」「③操作が結果に影響しない設計」「④レアリティ設計の破綻」の4点。

### 13.2 P5-1: ソリトン種ライブラリ（最優先・最小コスト最大効果）

- **やること**: 手続き的リングスタンプに代え、**正典 Lenia 生物の実パターン（RLE）を種データとして埋め込む**。Chakazul/Lenia `animals.json`（`cells` 保有 **548 種**）から挙動が多様な代表を選定（Orbium 系・放射/左右対称・移動・振動・回転・大型）。キュレーション表は [種データ抽出サブエージェント](33c82246-1379-473d-81da-13361a9440a6) が作成済み（20 種選定済み）。
- **実装**:
  1. `libs/biome-engine/src/species_library.rs`（新規）に `SpeciesSeed { name, mu, sigma, r, kernel_peaks, rle: &'static str }` の静的テーブルを持たせる。RLE デコーダ（255段階・`pqrstuvwxy` プレフィックス仕様、正典 `LeniaND.py:108-112` 準拠）を実装。**PoC で検証済みのデコードロジックを移植**（自前で書き直さない）。
  2. `LeniaSimulator::seed_species(&mut self, seed_id: usize, cx, cy)` を追加し、パターンを中央配置＋その種の μ/σ/R をゲノムに設定。
  3. `seed_from_rng`（`lenia.rs:127-136`）を「乱数で種ライブラリから 1 種を抽選して配置」に変更。異シード＝異なる生物になる（R1 解消）。
- **なぜ既存 Phase で解決しないか**: Phase 1 は「リングスタンプで PoC」を明示的に採用した（§4.2）。当時は正典 RLE 資産をリポジトリに持たない前提だったが、検証で `animals.json` が流用可能と判明したため、初期条件を差し替える新規作業。
- **車輪の再開発回避**: RLE デコードは PoC で動作確認済みのロジックを移植。カーネル半径ごとの FFT プランは既存 `LeniaSimulator`（`lenia.rs:78-100`）が R を持つので、種ごとに R が異なる場合のみ再プラン（R 種類は数個に限定）。
- **検証（Negative 必須）**: Positive=Orbium 種で重心移動を確認、異シード10個で species_hash が全て異なることを確認（R1）。Negative=不正 RLE（範囲外文字）を投入し panic せず空パターンにフォールバックすることを確認。

### 13.3 P5-2: マルチ種相互作用（捕食・対戦）— 検証で成立を確認済み

- **やること**: 現状「同一 sim の RGB 3ch」を、**相互作用する 2〜3 種**に変える。64×64 プロトタイプで以下を実測し成立を確認済み:
  - **捕食（被食者A・捕食者B）**: 適正パラメータ（pred係数 0.8〜1.2）で両種が安定共存（A≈840, B≈850）。強すぎ（1.5）ると捕食者が餓死し全滅 → **緊張感のある動的平衡が生まれる**。
  - **縄張り対戦（相互抑制）**: 2 陣営が競合し一方が領域制圧（857 vs 924）→ **対戦メカニクスとして機能**。
- **実装**: `lenia.rs::tick`（`:148-194`）のチャンネル更新に相互作用項を追加。ch1 の成長を `growth(potential) - k_pred * ch_prey[i]`、捕食者は餌依存 `growth * min(1, prey_potential * c) - starve` の形（PoC で成立したモデル）。相互作用係数を `LeniaGenome` に追加。
- **なぜ既存 Phase で解決しないか**: Phase 1 §4.2 段階2 は「ch 間相互作用（色＝個性）」を予定に挙げていたが**実装されていない**（コード確認済み: `lenia.rs:158-185` に他 ch を参照する項なし）。計画と実装の乖離を埋める作業＝計画の未完部分の実装。
- **検証（Negative 必須）**: Positive=捕食共存パラメータで 500 tick 後に両種 mass>0（振動または共存）。Negative=pred 係数を過大（1.5）にして捕食者 mass→0（餌を食い尽くして自滅）を確認＝相互作用が実際に効いている証明。

### 13.4 P5-3: 環境ペン（プレイヤーの因果を回復）— 検証で成立を確認済み

- **やること**: 種まきに加え、**壁/養分/毒を「描く」操作**を追加。64×64 プロトタイプで壁を描くと場が壁を回り込んで反対側へ到達（右側 mass 0→411）することを確認済み＝プレイヤー操作が結果を変える（R3 解消）。
- **実装**: `BiomeGrid` に `env_mask: Vec<u8>`（0=通常, 1=壁/成長禁止, 2=養分増強, 3=毒/減衰）を追加。`tick` の更新でマスクを参照（壁セルは field=0 固定、養分は growth 正方向を増幅、毒は減衰）。既存 `inject_brush`（`lib.rs:208`）と同じ座標系で `paint_env(x,y,radius,kind)` を追加。
- **なぜ既存 Phase で解決しないか**: Phase 4 UI は「種まき / μσ / 図鑑」の3操作に**削った**が、検証で「クリックが無意味（R3）」が判明。環境ペンは削減された操作性を、意味のある形で1つ戻す新規要素。UI は Phase 4 の3操作を4操作に増やす最小変更。
- **検証（Negative 必須）**: Positive=壁を描くと壁の反対側到達が遅延/迂回する。Negative=`env_mask` を全0（ペン未使用）にすると挙動が現状と一致（新機能が既存を壊さない）。

### 13.5 P5-4: レアリティ再設計（自動最大化の是正）

- **やること**: R4（放置で Epic 到達）を是正。現行 `rarity.rs:62-91` の tier 判定は mass/longevity 閾値が「放置テクスチャ」で満たされてしまう。**「局在性（bbox が小さい＝散らばらず生物的）」と「移動性（locomotion）」を必須条件に加える**。ベタ塗りテクスチャ（bbox≈100%）は Common 止まりにする。
- **実装**: `rarity.rs::lenia_rarity_tier` に bbox_ratio（`pattern::measure_field` で算出可能、既存 `pattern.rs` を拡張）を渡し、Epic 以上は `bbox_ratio < 0.5 && locomotion > 閾値` を AND 条件化。
- **なぜ既存 Phase で解決しないか**: Phase 3 でレアリティを Lenia 統計化したが、**局在性を条件に入れなかった**ため放置テクスチャが高レア化する穴が残った。Phase 3 の判定式の欠陥修正。
- **検証（Negative 必須）**: Positive=移動する局在ソリトン（Orbium 種）が Epic 以上。Negative=無操作 200 tick の広がったテクスチャが Common/Uncommon 止まりになることを確認（R4 解消の直接証明）。

### 13.6 P5-5: shake 演出の除去（UI ジッター）

- **やること**: クリックごとに枠が上下左右へ微動する問題（`BiomeGame.tsx:345-346` の `setShakeOffset({ x: (Math.random()-0.5)*8, ... })` を 512px コンテナの `transform: translate`（`:554`）に適用）を除去。演出を残すなら座標を動かさない `box-shadow` パルスに置換。
- **なぜ既存 Phase にないか**: Phase 0–4 完了後のユーザー報告で新規に判明した UI バグ。単純作業。
- **委譲**: 低トークンサブエージェント（`setShakeOffset` 呼び出しと state・`transform` 参照の削除、または box-shadow 化）。

### 13.7 実行順序・委譲マップ（Phase 5）

```
P5-5（shake除去, 独立・即効）── サブエージェント
P5-1（ソリトン種ライブラリ）→ これ単体で R1/R2 の大半を解消 ← 最優先・親
  → P5-4（レアリティ是正, 種ライブラリの多様性に依存）← 親
  → P5-2（マルチ種相互作用, 種ライブラリ上で捕食/対戦）← 親
  → P5-3（環境ペン, 操作の因果回復）← 親（UI 変更含む）
  → docs 同期（CHANGELOG / RIPPLE_MAP / ADR-049）← サブエージェント
```

| 作業 | 担当 | 理由 |
|---|---|---|
| RLE デコーダ・種ライブラリ・相互作用項・環境マスク・レアリティ式（本番 Rust ロジック） | **親エージェント** | Safety-Critical 隣接＋設計判断を伴う |
| shake 除去（UI 単純削除） | サブエージェント | 機械的 |
| テスト期待値更新（species_hash・rarity 閾値・stride 不変確認） | サブエージェント | 機械的 |
| CHANGELOG / RIPPLE_MAP / ADR-049 記載 | サブエージェント | 定型 |
| `animals.json` からの RLE 抽出・キュレーション | **完了**（[サブエージェント](33c82246-1379-473d-81da-13361a9440a6)） | — |

### 13.8 /perfect-plan 5ゲート検証（Phase 5）

- **Gate 1 構造スキャン**: `species_library.rs` は未存在＝二重実装なし。`lenia.rs`/`rarity.rs`/`pattern.rs`/`grid.rs` は実在（拡張であり新規重複なし）。`animals.json` はリポジトリ未収録→ RLE 文字列を `species_library.rs` に埋込（外部依存を持ち込まない）。
- **Gate 2 要件カバレッジ**: §2 経済/§3 MCP/§4 セキュリティ/§8 P2P いずれも影響なし（Biome 内部ロジックのみ）。§4 は RLE デコードの入力検証（範囲外文字フォールバック）を Negative Test で担保。
- **Gate 3 依存・波及**: `LeniaGenome` にフィールド追加（相互作用係数・種 R）→ `serialize/deserialize`（`lib.rs:397-413`）と `orbium_default`（`lenia.rs:28`）に追随。consumer（`dream_state/biome.rs`）が呼ぶ API シグネチャ（tick/get_rarity/set_mutation_boost/roll_substance）は**不変**＝波及は biome-engine 内に限定。IndexedDB は v2→v3 で旧セーブ破棄（開発段階で許容、§10 準拠）。
- **Gate 4 悪魔の弁護人**:
  1. **最悪シナリオ**: 種ごとに R が異なると FFT プランを種数分持つ必要があり wasm が肥大化。→ 緩和: R を数値種（13/20/27 等）に丸め、プランを R 種類ぶんだけ生成・共有。
  2. **見落とし前提**: 「正典パターンが 128×128・トロイダル・dt=0.1 でそのまま安定」を仮定。→ PoC で Orbium は安定移動を確認済みだが、大型種（Pentadecactenium 等 93×8）は 128 グリッドで自己衝突しうる。P5-1 で「グリッドに収まる中小型種を優先選定」をゲート化。
  3. **やらないメリット**: P5-1 だけでも R1/R2 は大幅改善。P5-2/P5-3 はリスクが高いので P5-1 を先行 merge して価値を先出しできる（段階リリース）。
- **Gate 5 実行順序**: P5-5 独立 → P5-1（種）→ P5-4（レアリティは種の多様性前提）→ P5-2（相互作用は種の上で動く）→ P5-3（環境ペン）。依存の逆転なし。

### 13.9 判定

**⚠️ PATCH → ✅**。真因は推測でなく WASM 直接計測で確定済み。対策は既存 Phase の未完部分の完成（相互作用）＋検証で判明した穴の修正（初期条件・レアリティ・因果）であり、車輪の再開発・二重実装なし。**P5-1 を最優先ゲートとして着手可能**。
