# Phase E — アバター配線計画（2D + 3D / Inochi 凍結 / Live2D 後付け）

**版**: v1.0.3  
**日付**: 2026-07-25  
**状態**: **E5 ✅**（UI/文書凍結 + /reflexion×2）。E1–E4 / Phase F は未実装  
**ゲート**: **E0 = Y**（2026-07-25 ユーザー決定）  
**親参照**: [`nurture_remaining_ledger_plan.md`](nurture_remaining_ledger_plan.md) Wave D / NR-02・03・11  
**品質計画の意図**: [`nurture_quality_max_plan.md`](nurture_quality_max_plan.md) Phase E 段落（本ファイルが実装親）

---

## 0. 製品方針（SSOT）

| 決定 | 内容 |
|---|---|
| **リリース当初のアバター** | **2D 画像**（PNG / lite・billboard 系）と **3D モデル**（VRM 本線 + GLB プレビュー）のみ |
| **Inochi 族** | **凍結**。既存 Inochi2D（`.inx` / `InxRenderer` / upload API）は出荷 UI から外し、新規実装禁止。**Inochi3D は着手禁止**（リポジトリに実体なし・開始しない） |
| **Live2D** | **後付け可能**にする。リリース当初は非同梱。アダプタ境界だけ先に定義し、SDK/ライセンス承認後に Phase F で実装 |
| **E0** | **Y** — Settings の `vrm` モードを「名ばかり PNG」から **実 three-vrm ロード**へ移行する |

### モード語彙（出荷）

| Settings / 内部 mode | 出荷 | 実体 |
|---|---|---|
| `lite` | ✅ | 2D PNG（`AiomeAvatar` / `getAssetPath('lite')`） |
| `vrm` | ✅ | **実 VRM**（`@pixiv/three-vrm`）。移行完了まで PNG フォールバック可 |
| `glb` | ✅（任意露出） | 静的 3D プレビュー（既存 `GlbRenderer`）。表情/viseme は任意・最小 |
| `off` | ✅ | 非表示 |
| `inx` / Inochi2D | ⏸️ 凍結 | UI 選択から除去。ルートは削除せず deprecate（破壊的削除は別承認） |
| Live2D / Cubism | ⏸️ 将来 | Phase F。`AvatarLipSyncAdapter` 実装を追加するだけで差し込める設計 |

---

## 1. 現状アンカー（2026-07-25 実測）

| 事実 | アンカー |
|---|---|
| Settings 選択は `vrm` \| `lite` \| `off` のみ | `useDisplayMode.ts` / `SettingsPage.tsx` |
| mode=`vrm` でも実行時は **PNG** `CharacterBillboard` | `VrmRenderer.tsx` → `CharacterBillboard.tsx`（`TextureLoader`） |
| `AVATAR_ASSETS.vrm` の値がすべて `.png` | `AvatarContext.tsx` |
| `@pixiv/three-vrm` は依存のみ。`useVrmExpression` **import ゼロ** | `package.json` / `useVrmExpression.ts` |
| `GlbRenderer` は `useGLTF` 実装済みだが mode 到達不能 | `GlbRenderer.tsx` / `useDisplayMode` に `glb` なし |
| `InxRenderer` は WASM 未接続プレースホルダ | `InxRenderer.tsx`（`@nicebyte/inochi2d-es` コメントアウト） |
| Inochi2D API は存在 | `POST/GET .../avatar/inochi2d/*`、`avatar-engine` loader はモックメタ |
| Inochi3D / Live2D / Cubism **実装なし**（文書に凍結・Phase F 宣言のみ） | コードランタイム grep 0。計画/OPEN に方針記載あり |
| DRM 鍵: `DrmEngine` + `x-nurture-drm-key` ✅ / `tauri://vrm/` ❌ | nurture-infra / asset.rs / src-tauri |
| viseme キューは `useVisemeSync` まで。VRM expression 未接続 | `useVisemeSync.ts` |
| サンプル `.vrm` は `public/vrm/sample/sample.vrm` 等にあるが UI 未参照 | NR-11 |
| BoneChecker は `VrmAvatar` + GLTF fail-closed。コーパス調整は NR-06 Human | `bone_check.rs` |
| `VramArbiter` はサイドカー MB 用。UI fps 予算には流用禁止 | ledger NR-02 |

---

## 2. やらないこと（再発明・スコープ外）

| 禁止 | 理由 |
|---|---|
| Inochi2D WASM 完成 / Inochi3D 新規 | 製品凍結 |
| Live2D SDK 同梱・Cubism ランタイム実装（本計画内） | Phase F。ライセンス未決 |
| 第2 DRM / 第2 VramArbiter | 既存再利用のみ |
| `PurchasePolicy` / buy 解禁 | Wave B'=N 済み |
| クローゼット / 一般アセットストア UI 全体 | 別計画（本計画は表示配線＋公式素体＋DRM 配信） |
| PNG のまま `tauri://vrm/` を名乗る | E0=Y と矛盾 |
| Safety-Critical（`src-tauri` CSP / protocol）の自律変更 | **明示承認必須**（E4） |
| ADR-022 Aesthetic Pressure の本実装 | 本計画外（フォーマット凍結の注記のみ） |

---

## 3. アーキテクチャ（アダプタ境界）

```text
                    ┌─────────────────────────────┐
 DisplayMode        │  AvatarSurface (単一入口)     │
 lite|vrm|glb|off   │  - 2D: AiomeAvatar / Billboard│
 (+ live2d 将来)    │  - 3D: VrmRuntime / GlbPreview │
                    │  - frozen: Inx* (到達不能)     │
                    └───────────┬─────────────────┘
                                │
              AvatarLipSyncAdapter.applyViseme / reset
                                │
         ┌──────────────────────┼──────────────────────┐
         ▼                      ▼                      ▼
   VrmLipSyncAdapter      Glb(no-op/min)        Live2dLipSyncAdapter
   (Phase E)              (Phase E)             (Phase F・未実装)
```

- 既存 `AvatarLipSyncAdapter`（`types/avatar.ts`）を **正の契約**とする。Inochi 向け実装は凍結（削除必須ではない）。
- Live2D は同契約の新実装 + `DisplayMode` 値追加のみで差し込む（レンダラ本体の if 地獄禁止）。

---

## 4. Waves

### E0 — 製品ゲート（✅ 2026-07-25）

- [x] three-vrm を実行時にする = **Y**
- [x] 出荷フォーマット = **2D + 3D**
- [x] Inochi 族凍結 / Live2D は後付け枠

**DoD**: 本計画 v1.0 が OPEN から参照され、偽オープン「親計画未作成」が消える。

---

### E1 — 公式素体・アセット正本（NR-11 / Human 協調）

1. 公式 `.vrm` を `apps/management-console/public/vrm/`（および api-server static 同期ルール）に配置。既存 `sample/sample.vrm` を検証用に残すか、公式素体に置換するかを Human 決定。
2. `AVATAR_ASSETS` の `vrm` エントリを **実 `.vrm` URL** に更新（`.png` 誤用を解消）。2D 用は `lite` に集約。
3. BoneChecker / NR-06: 公式素体は閾値パス必須。コーパス調整は Human（本 Wave と並列可）。
4. LP / MESSAGING の「Inochi2D 感情表現」表記は **E5 文書 Wave** で「2D / VRM」に同期（実装前に偽訴求しない）。

**DoD**: UI が参照する公式 `.vrm` がリポジトリに存在し、`AVATAR_ASSETS.vrm` が `.vrm` を指す。Negative: 欠落 URL でレンダラがクラッシュせずフォールバック。

**検証**: Human 目視 + Jest（アセットパス）+ BoneChecker 対公式素体。

---

### E2 — 実 VRM ロード（NR-02 の前半）

1. `VrmRuntime`（仮名）コンポーネントを新設: `GLTFLoader` + `@pixiv/three-vrm` で ArrayBuffer/URL ロード。
2. `VrmRenderer` / `AvatarViewerModal` / `DioramaView` の mode=`vrm` を `CharacterBillboard` → `VrmRuntime` に切替。
3. 失敗時フォールバック: PNG billboard（`lite` アセット）または明示エラー UI（クラッシュ禁止）。
4. `useVrmExpression` を **初めて import** し、`avatarState` → expression を接続。
5. `useVisemeSync` → `VrmLipSyncAdapter` → `expressionManager`（または同等）へ接続。`ExpressionPipeline`（感情文 UI）とは混線禁止。

**DoD**: Settings=`vrm` で実 `.vrm` が描画され、speaking で口/表情が動く。Positive + Negative（壊れた URL / 非 VRM バイト）。

**禁止**: Inx 経路の「ついで修正」。Glb は E2b で任意。

#### E2b — GLB モード露出（任意・同時可）

- `useDisplayMode` に `glb` を追加し Settings に露出するかは製品確認（既定: **任意**）。
- 既存 `GlbRenderer` 再利用。viseme は no-op でよい。

---

### E3 — UI フレーム予算（NR-02）

1. MC 側に **UI 専用**のフレーム/負荷ガード（例: 背面タブで `frameloop="demand"`、可視時のみ animate、解像度キャップ）。
2. **`VramArbiter` 流用禁止**（サイドカー LLM VRAM と別問題）。
3. 低スペックフォールバック: 自動で `lite`（2D）へ落とすオプションを検討（既定オフでも可）。

**DoD**: ドキュメント化された予算（目標 fps / 同時 Canvas 数）と、超過時の挙動テスト。

---

### E4 — On-memory DRM 配信（NR-03）🔐 Safety-Critical

1. 既存 `DrmEngine` / License / `x-nurture-drm-key` を再利用（再実装禁止）。
2. Desktop: Tauri custom protocol（構想名 `tauri://vrm/` または現行 Tauri 2 の asset protocol 方針に合わせた名称）で復号バイトを WebView へ。  
   - **`src-tauri` / `tauri.conf.json` 変更は明示承認必須**（AGENTS Safety-Critical / T-003）。
3. Web: 平文 CDN 直読みを既定禁止にするかは製品確認（ベータは local/sample のみでも可）。
4. `VrmRuntime` は URL 文字列だけでなく ArrayBuffer ロード経路を持つ。

**DoD**: Desktop で鍵付きアセットがディスク平文永続なし（または文書化した例外）で表示。Negative: 鍵なし拒否。

**依存**: E2 完了後。OP-088 鍵注入と非重複（再利用）。

---

### E5 — Inochi 凍結の文書・UI 閉鎖 + 訴求同期 ✅ 2026-07-25

1. ✅ Settings / 型 / `HomePage` / Diorama / CharacterPanel / AvatarViewer の `inx` 分岐を出荷パスから除去。`InxRenderer` はファイル残置 + `@deprecated`。
2. ✅ inochi upload/serve に **deprecate** 注記 + upload warn（API 即削除は別承認）。
3. ✅ ADR-022 / LP i18n（`free_f4`）/ PRIVACY / SYSTEM_PANORAMA。Live2D は現行機能として未記載。
4. ✅ `avatar-engine` description を「2D + VRM; Inochi2D frozen」へ。

**DoD**: ユーザーが Inochi を選べない。公開面に Inochi を現行機能として書かない。

---

### E6 / Phase F — Live2D 差し込み枠（本計画では実装しない）

**解除条件（すべて）**:
1. Cubism / Live2D ライセンスと配布形態の Human 承認
2. `AvatarLipSyncAdapter` + DisplayMode 拡張の設計レビュー
3. E2 の VRM 経路が安定（回帰ピンあり）

**最小設計メモ（実装時）**:
- 新レンダラ 1 ファイル + adapter 1 つ
- Settings に `live2d` 追加
- Inochi コードを Live2D に転用しない（別スタック）

---

### E7 — マーケット seed（任意・E1 後）

公式素体をマーケットに載せる場合のみ。一般ストア UI 全体は別計画。

---

## 5. 実行順序

```text
E0 ✅
  → E1（公式素体 / AVATAR_ASSETS）∥ NR-06 Human
       → E2（three-vrm 実ロード + viseme）
            → E2b（glb 露出・任意）∥ E3（フレーム予算）
                 → E4（DRM protocol・承認後）
       → E5（Inochi 凍結 UI/文書）… E2 と並列可、公開訴求は E2 前に偽機能を書かない
Phase F (Live2D) … E2 安定 + ライセンス後
```

---

## 6. OPEN / 台帳への写像

| ID | Phase E 内 | 状態（計画 v1.0 時点） |
|---|---|---|
| NR-11 | E1 | OPEN（Human 素体） |
| NR-02 | E2 + E3 | OPEN（親計画成立後に着手可） |
| NR-03 | E4 | OPEN（Tauri 承認ゲート） |
| NR-06 | E1 並列 | HUMAN |
| Inochi2D/3D | E5 + 凍結 | ✅ E5 出荷閉鎖（API 物理削除は別承認） |
| Live2D | Phase F | ⏸️ 後付け枠 |

---

## 7. /perfect-plan 検証結果（v1.0）

### Gate 1: 構造スキャン
- ✅ 二重実装回避: 新規は `VrmRuntime` + adapter。`DrmEngine` / `GlbRenderer` / `useVisemeSync` / `useVrmExpression` は再利用。
- ✅ Inochi3D は存在しない → 「凍結」= 開始禁止 + Inochi2D 出荷停止で十分。
- ✅ `ExpressionPipeline` 混線を明示禁止。

### Gate 2: 要件カバレッジ
- ✅ §6 VRM/3D: E0–E4 でカバー。
- ✅ §5 法的: BoneChecker / NR-06 を E1 に接続。Live2D ライセンスを Phase F ゲートに。
- ✅ §2 経済: 本計画は DRM 鍵再利用のみ。buy 非対象。
- ✅ §4: E4 は Tauri Safety-Critical。

### Gate 3: 依存・波及
- FE: `VrmRenderer` / `CharacterBillboard` / `AvatarContext` / `useDisplayMode` / `DioramaView` / `AvatarViewerModal` / `CharacterPanel`
- BE: E4 のみ nurture asset + Desktop tauri（承認後）
- テスト: Jest avatar / HomePage mocks（`.vrm` URL 前提に更新）
- LP: E5 で Inochi 表記削除

### Gate 4: 悪魔の弁護人
1. **最悪**: three-vrm がメインスレッドを食い UI 不可 → E3 予算と lite フォールバックを必須化。
2. **誤前提**: 「mode=vrm は既に 3D」→ 実は PNG。E1 でアセット拡張子を直さないと E2 が即死。
3. **やらないメリット**: PNG+lite のまま出荷は可能だが、ユーザー決定 E0=Y と矛盾。Inochi 完成より 2D+VRM の方がリリース初期と整合。

### Gate 5: 順序
- ✅ E1（実 `.vrm` URL）→ E2（ローダ）。逆順禁止。
- ✅ E4 は E2 の ArrayBuffer 経路後。
- ✅ E5 文書を E2 前に「VRM 対応済み」と偽らない（凍結告知は先行可）。

### 判定
- **✅ PASS（v1.0）** — 実装は別承認。次アクションは E1 アセット決定 or E2 実装承認。

---

## 8. 成功基準

| 基準 | 意味 |
|---|---|
| 出荷フォーマット | ユーザーが選べるのは 2D と 3D（+off）のみ |
| Inochi | UI 非露出・新規着手なし（2D/3D とも） |
| Live2D | 未実装だが adapter 契約で差し込み可能と文書化 |
| E0 | Settings=`vrm` で実 VRM が動く（E2 DoD） |
| 再発明ゼロ | DRM/VramArbiter/PurchasePolicy を新設しない |
| Safety | E4 は人間承認なしに `src-tauri` を触らない |

---

## 9. 次アクション（ユーザー向け）

**E5 ✅**。残り実装は明示承認が必要:

1. **E1**: 公式素体 `.vrm` の選定・配置（Human）→ 続けて `AVATAR_ASSETS` 更新  
2. **E2**: 「Phase E の VRM 実ロードを実装して」  
3. **E4**: Desktop DRM protocol は別途明示承認  
4. **Phase F**: Live2D はライセンス後に別計画  
5. （任意）Inochi API 物理削除 — 破壊的・別承認
