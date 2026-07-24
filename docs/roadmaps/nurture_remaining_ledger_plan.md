# Nurture 残台帳クローズアウト計画（実コードベース）

**版**: v1.3.3  
**日付**: 2026-07-25  
**実装状況**: Wave **0a / 0b / 0c / A / B'=N** ✅。**Phase E 親計画成立（E0=Y）+ E5 ✅**。残: E1–E4 / NR-06 Human / NR-07 Legal  
**正本コード**: `aiome/commercial/`（スタンドアロン `Project-Nurture/` は古いミラー。TLA も **差分あり** → CI・編集は commercial のみ）  
**台帳正本**: 完了後は `OPEN.md` へ正式移入。`REMAINING_TASKS.md` §3 は本 Disposition に合わせて更新する。  
**Phase E 正本**: [`phase_e_vrm_wiring_plan.md`](phase_e_vrm_wiring_plan.md) v1.0

### 版履歴（検証駆動）

| 版 | 実コード検証で直したこと |
|---|---|
| v1.0 | 初版 Disposition |
| v1.1 | `TaskApprovalOverlay` 購買流用を削除。`PurchasePolicy` 新 enum を禁止し MCP whitelist に収束。ADR-052 DONE 明示。他 TLA を Wave A から除外 |
| v1.2 | Wave A の TLC **cwd=`commercial/`** を必須化。NR-14 `/withdraw` sunset。whitelist 回帰ピン。Settings=monthly のみ。Phase E 親計画未作成。HTTP eKYC/Pro |
| v1.3.1 | /reflexion: Disposition・OPEN 移入案を実装後状態に同期。NR-14 前倒し・CHANGELOG 404\|405・NR-01=CI 配線を明記。計画ファイルを git 追跡対象に含める |
| v1.3.2 | Wave B'=**N** 製品判断クローズ（解禁トグル非実装）。NR-09 DONE。OPEN / REMAINING / UNCERTAINTY / synergy W-5 追記を同期 |
| v1.3.3 | E0=**Y**。Phase E 親計画作成。出荷=2D+3D。Inochi2D/3D 凍結。Live2D=Phase F。NR-02/03/11 を E1–E4 に分解 |
| v1.3 | Wave 0 を **0a 文書 / 0b ピン / 0c alias** に分割（承認粒度）。`REMAINING_TASKS` の `file://Project-Nurture/...` リンクを commercial へ直し、偽オープンの参照先誤りを禁止。NR-14 影響を実測縮小（OpenAPI/generated に withdraw なし・router + deep_scan_matrix のみ）。Wave A: DoD=CI（ローカル JRE 無しでも可）。TLA に `MODULE` ヘッダ無し→失敗時のみヘッダ追加を許可。A2C D-6 は §3 外・再計画禁止 |

---

## 0. 目的

`REMAINING_TASKS.md` §3（および `UNCERTAINTY_BREAKTHROUGH.md` 由来項目）を実コードに合わせて再分類し、偽オープン削除・委譲・凍結・最小 Wave のみを残す。

---

## 1. Disposition 表（単一の正）

| ID | 旧台帳項目 | 実コード現状（検証アンカー） | Disposition | 行き先 |
|---|---|---|---|---|
| NR-01 | TLA+ 策定＋TLC | `commercial/specs/NurtureEconomyProtocol.tla` + `.cfg`（`minted` / SurpriseBonus / `CoinsConserved`）。W-6 配線済（`buy.rs`→`SurpriseEngine`）。**aiome formal-verify は Quarantine のみ**。スタンドアロン TLA は **古い別物**（行数 64 vs 78） | **PARTIAL → Wave A**（仕様再作成禁止。commercial を CI へ） | Wave A |
| NR-02 | VRM 15fps / LLM VRAM | `VramArbiter`=サイドカー MB。15/60fps **なし**。実行時=`CharacterBillboard`（PNG）。`useVrmExpression` import ゼロ | **OPEN（E2–E3）** | [`phase_e_vrm_wiring_plan.md`](phase_e_vrm_wiring_plan.md) |
| NR-03 | On-memory DRM `tauri://vrm/` | `DrmEngine` / License / `x-nurture-drm-key` あり。Custom Protocol **なし**。OP-088=鍵注入 | **OPEN（E4）** | 同・`src-tauri` 承認必須 |
| NR-04 | Saga Compensable / CompensationLog | `Initiated…Cancelled` + `nurture_saga_logs` + `rollback()`。請求シンボルなし | **FREEZE（既定）** | Wave C（例外のみ） |
| NR-05 | ZKP / CoinQuantum / 経済 CRDT | `coin_quantum.rs` スタブ。Bellman なし。Automerge は経済外で完了 | **FREEZE** | 分散決済要求まで |
| NR-06 | BoneChecker 実機調整 | `CHILD_PROPORTION_THRESHOLD = 1/5.5`・unverifiable fail-closed・GLTF | **HUMAN** | コーパスのみ |
| NR-07 | `MAX_TOTAL_OUTSTANDING_COINS` | 未実装。`KC_LEGAL_POSITION` で無償 KC=非該当 | **LEGAL GATE** | 有償 KC 別計画 |
| NR-08 | 特商法 | LP `/tokushoho` + COMPLIANCE ✅ | **DONE** | Wave 0 |
| NR-09 | PurchasePolicy 3 モード | **二系統**: (1) MCP whitelist: `marketplace_search`/`wallet_balance` のみ true（`marketplace_buy` は `_ => false`）。(2) HTTP `execute_purchase`: **ProAuthenticated + eKYC + 所有者一致** + OP-011 + spend_guard。Settings 露出は現状 **`economy.monthly_spend_limit` のみ**。Job Overlay / Buzz は購買ではない | **DONE（Wave B'=N・2026-07-25）**。新 enum 禁止。解禁トグルは作らない。将来の解禁は明示 Y のみ | Wave B' ✅ |
| NR-10 | CP→ギフト / Tremendous | ADR-052 OUT。`/commerce/convert-points` のみ（`/withdraw` alias は NR-14 で削除済）。payout DROP 済。`commercial/gift/mod.rs` 空→復活禁止 | **OUT / DONE** | Wave 0 |
| NR-11 | 公式素体 | sample.vrm は存在するが UI 未参照。`AVATAR_ASSETS.vrm` は `.png` | **OPEN（E1 / Human）** | phase_e E1 |
| NR-12 | Biome 目視 | OP-002 ✅ | **DONE** | Wave 0 |
| NR-13 | 他 TLA の CI 未配線 | Karma/Federation/ContextEngine 等 | **OUT OF THIS PLAN** | 別 OP |
| NR-14 | `/withdraw` alias sunset | sunset 2026-08-01 **前倒し削除**（2026-07-25）。`convert-points` のみ。deep_scan 追随済 | **DONE** | Wave 0c ✅ |

---

## 2. やらないこと（再発明禁止）

| 禁止 | 理由 / 既存正本 |
|---|---|
| TLA 再策定・スタンドアロン TLA を正本化 | commercial が正本（差分確認済） |
| Wave A で全 TLA 一括配線 | NR-13 |
| ルート cwd のまま nested path で TLC（未検証） | **`working-directory: commercial`** で実行 |
| 第2 VRAM 調停 / 第2 DRM / 新 Saga FW | `VramArbiter` / `DrmEngine` / `log_saga` |
| 経済 ZKP・CoinQuantum 本実装 | NR-05 |
| BoneChecker 再実装 | 閾値・コーパスのみ |
| CP→Tremendous / `commercial/gift` 復活 | ADR-052 |
| `MAX_TOTAL_OUTSTANDING_COINS` 先行 | 無償 KC 中は誤解 |
| `Project-Nurture/` へ新機能 | — |
| **`PurchasePolicy` / 新承認画面** | MCP whitelist + HTTP eKYC |
| **`TaskApprovalOverlay` / `BuzzApproval` を購買に流用** | Job / SNS 専用 |
| PNG のまま `tauri://vrm/` | 死蔵 |
| Phase E 実装手順を本ファイルに肥大化 | 親計画 [`phase_e_vrm_wiring_plan.md`](phase_e_vrm_wiring_plan.md) へ委譲済み |
| Inochi2D 完成 / Inochi3D 着手 | 製品凍結（出荷は 2D+3D。Live2D は Phase F） |
| SurpriseEngine / W-6 / convert-points / A2C D-6 本体の再計画 | 完了済（D-6 は `should_trigger_a2c_gift` + dry-run 既存） |
| Settings に日次/単発上限を「新キー」で二重定義 | nurture `EconomyPolicy` と api-server settings の既存同期を調査してから露出のみ（Y 時） |
| `REMAINING_TASKS` からスタンドアロン Nurture 絶対パスを正本扱い | 正本ドキュメントは `commercial/docs/` |

---

## 3. 既存計画との境界

| 既存 | 関係 |
|---|---|
| `nurture_quality_max_plan.md` ✅ + Phase E 1 段落 | 意図の親。**実装親** = [`phase_e_vrm_wiring_plan.md`](phase_e_vrm_wiring_plan.md) v1.0 |
| `synergy_maximization_plan.md` W-5/W-6 | W-6 完了。W-5「buy 解禁判断」= NR-09 |
| ADR-052 | NR-10 + NR-14 |
| `KC_LEGAL_POSITION.md` | NR-07 |
| OP-011 / OP-083-B | HTTP 購買＋spend_guard |
| OP-088 | DRM 鍵。NR-03 と非重複 |
| OP-020 | CRDT 誤認防止 |
| Upstream OP-030–034 / OP-068 | スコープ外 |

---

## 4. Waves

### Wave 0 — 台帳同期＋短命後始末（承認単位で分割）

#### 0a — 文書のみ（既定・最優先・API 非破壊）

1. `OPEN.md` の Nurture 一括行を §6 案へ置換。
2. `REMAINING_TASKS.md` §3 を Disposition 付きに更新し、冒頭リンクを  
   `commercial/docs/UNCERTAINTY_BREAKTHROUGH.md` / `commercial/` 配下ガイドへ変更（`file://.../Project-Nurture/...` を正本扱いしない）。
3. `commercial/docs/UNCERTAINTY_BREAKTHROUGH.md` に superseded 注記（A1「TLA 不存在」、C2 CP→ギフト、PurchasePolicy 推奨など）。
4. CHANGELOG `[Unreleased]` 1 行。
5. （任意）synergy 計画の buy 解禁オープン → NR-09 ポインタ。

**DoD**: 偽オープン解消。コード差分なし。

#### 0b — whitelist 回帰ピン（任意同時・低リスク）

`apps/api-server/src/mcp/server.rs` テストに:

```rust
assert!(!check_whitelist("marketplace_buy"));
assert!(!check_whitelist("buy"));
```

設定トグルは作らない（**Wave B'=N クローズ**）。**DoD**: 当該 unit PASS。

#### 0c — NR-14 alias 削除（明示承認・破壊的）

1. `router.rs` から `/api/v1/commerce/withdraw` ルートと sunset コメントを削除。
2. `docs/architecture/deep_scan_matrix.md` の旧 path 行を削除または convert-points に置換。
3. OpenAPI / `generated.ts` は現状 withdraw 非掲載のため **再生成は確認のみ**（差分ゼロ想定）。
4. Negative: 旧 path が 404（統合テスト 1 本で足りる）。

**Safety-Critical**: 公開 API alias 削除 → **0a/0b と分離承認**。

---

### Wave A — NurtureEconomy TLC を aiome CI へ（既定・CI 承認後）

**これだけ**（仕様の意味変更禁止・他 TLA 禁止）:

```yaml
- name: Run TLC Model Checker (NurtureEconomyProtocol)
  working-directory: commercial
  run: |
    java -cp ../tla2tools.jar tlc2.TLC specs/NurtureEconomyProtocol.tla \
      -modelcheck -deadlock -workers auto \
      -config specs/NurtureEconomyProtocol.cfg
```

- jar は既存ステップどおりリポジトリルートへ DL → `../tla2tools.jar`。
- **DoD の正本は CI**（`setup-java` あり）。開発機に JRE が無くても Wave A をブロックしない。
- 許可される最小修正（意味不変）: (1) cwd/パス、(2) ファイル先頭への  
  `---- MODULE NurtureEconomyProtocol ----` 追加（現状ヘッダ無し。Quarantine 仕様はヘッダあり）。  
  `BuyItem` / 不変条件の書き換えは別コミット＋承認。
- ファイル欠落時は Quarantine のように silent skip **しない**（fail）。
- スタンドアロン Project-Nurture CI 同期スクリプトは作らない。

**DoD**:
- Positive: CI で TLC PASS。
- Negative: `CoinsConserved` 一時破壊 → FAIL → 復元（CI または JRE ある環境）。
- Safety-Critical: workflow 変更は **明示承認必須**。
---

### Wave B' — エージェント buy 解禁（製品ゲート）

**決定（2026-07-25）: N — クローズ。** 解禁トグルは実装しない。

| 回答 | 行動 |
|---|---|
| **N（採用）** | 文書で「実効制御=whitelist 未解禁 + HTTP は eKYC/Pro」。実装は Wave 0b の回帰ピンのみ。OPEN NR-09 ✅ |
| **Y（将来）** | 明示承認後のみ下記。現状は着手禁止 |

**Y の最小設計（将来用・今は実装しない）**:

1. whitelist を設定値参照に変更（新 enum 禁止）。
2. 金額ガード新設禁止。Settings は現状 `monthly_spend_limit` のみ → 追加露出は **既存キーの同期調査後**に限る（二重キー禁止）。
3. 承認 UI 新設禁止。Job Overlay 流用禁止。
4. HTTP `execute_purchase` は触らない（既に Pro+eKYC）。
5. TDD: OFF 拒否 / ON 許可 + spend_guard Negative。Wave 0 のピンはトグル OFF 既定と整合。

---

### Wave C — Saga 型強化（例外のみ）

着手条件: 本番 rollback 漏れ or 明示要求。それ以外 FREEZE。`Compensable` 名目のリファクタ禁止。

---

### Wave D — Phase E（親計画ゲート）

**✅ 親計画成立（2026-07-25）**: [`phase_e_vrm_wiring_plan.md`](phase_e_vrm_wiring_plan.md) v1.0。**E0=Y**。

製品方針（要約）:
- 出荷: **2D 画像 + 3D（VRM / GLB）**
- **Inochi2D / Inochi3D 凍結**（UI 非露出・3D 着手禁止）
- **Live2D**: Phase F 後付け（`AvatarLipSyncAdapter`）

```
E0 ✅  three-vrm 実行時化 = Y
E1     公式素体 .vrm（NR-11 / Human）
E2     実ロード + viseme（CharacterBillboard からの移行）
E3     UI フレーム予算（NR-02）— VramArbiter 流用禁止
E4     On-memory 配信（NR-03）— DrmEngine 再利用・src-tauri 承認
E5     Inochi 凍結の UI/訴求閉鎖
E6/F   Live2D（ライセンス後）
E7     マーケット seed（任意）
```

**本台帳ファイルでは Phase E を実装しない。** 実装承認は phase_e 計画に対して行う。

---

### Wave L — 有償 KC

`KC_LEGAL_POSITION.md` §4 完了後の別計画。本計画は LEGAL GATE 記載のみ。

---

## 5. 実行順序

```text
Wave 0a (台帳文書) ──┬──► Wave 0b (whitelist ピン)  … 任意同時
                     └──► Wave 0c (withdraw alias) … 別承認
                              │
                              └──► Wave A (TLC CI, cwd=commercial)
                                        ├── Wave B'=N ✅  … 製品判断クローズ
                                        ├── Wave C  … 既定スキップ
                                        └── Wave D  … Phase E 親計画 ✅ → E1–E4 は phase_e 計画へ
Wave L  … 有償 KC 後
```

並列可: 0a ∥ 製品ゲート議論。0b ∥ 0a。0c と A は独立承認可。

**既定推奨出荷**: **0a + 0b + A + B'=N**（文書）。0c は sunset 前倒し済。

---

## 6. OPEN.md 移入案（Wave 0a）

```markdown
## 🌱 Project-Nurture / commercial 残（正本: docs/roadmaps/nurture_remaining_ledger_plan.md v1.3.3）

- [x] NR-08 / NR-10 / NR-12 / NR-14 / NR-01 / NR-09（B'=N）
- [x] Phase E 親計画 E0=Y（phase_e_vrm_wiring_plan.md v1.0）— 2D+3D / Inochi 凍結 / Live2D 後付け
- [ ] NR-11 E1 / NR-02 E2–E3 / NR-03 E4 / NR-06 Human / NR-07 Legal
- ⏸️ Inochi 族 / Live2D Phase F / NR-04 / NR-05
- （別レーン）NR-13 他 TLA CI
```

---

## 7. /perfect-plan 検証結果（v1.3）

### Gate 1: 構造スキャン
- ✅ 再確認: formal-verify=Quarantine のみ / MCP buy 未掲載 / Surprise 配線済 / withdraw は **router のみ**（OpenAPI・generated 非掲載）/ deep_scan に旧 path / BoneChecker 1/5.5 / Settings monthly のみ / Phase E ファイルなし / REMAINING がスタンドアロン絶対パス参照 / commercial TLA に MODULE ヘッダなし。
- ✅ 購買×Overlay 誤接続は排除維持。
- ✅ ローカル JRE 欠落を確認 → Wave A DoD を CI 正本に変更。

### Gate 2: 要件カバレッジ
- ✅ §3 全項目を Disposition。D-6 A2C を誤って再オープンしていない。
- ✅ 偽オープンの参照先（REMAINING→Project-Nurture）を 0a で修正対象化。

### Gate 3: 依存・波及
- 0c: router + deep_scan（types は確認のみ）。
- 0b: mcp テストのみ。
- A: CI + 必要なら MODULE ヘッダ 1 行。

### Gate 4: 悪魔の弁護人
1. **最悪**: Quarantine 流 silent skip で TLC 未実行のまま GREEN → fail-closed を維持。
2. **誤前提**: 「withdraw 削除＝OpenAPI/types 大工事」→ **実測は router 専有**。過剰作業を削減。
3. **やらないメリット**: 0a+0b+A で台帳・意図ピン・CI を解消。PurchasePolicy/Phase E/Saga/ZKP は不要。

### Gate 5: 順序
- ✅ 0a →（0b∥）→（0c 別承認）→ A → 任意 B'/C → 別 D → L。
- ✅ 破壊的 0c を文書 Wave に混ぜない。

### 判定
- **✅ PASS（v1.3）** — 既定スコープ **0a + 0b + A**。
- 同一検証ループの追加反復は、新規コード事実が無い限り **収穫逓減**。次は実装承認待ち。

---

## 8. 成功基準

| 基準 | 意味 |
|---|---|
| 偽オープンゼロ | 特商法・ADR-052・Biome・「TLA 未策定」「CP ギフト必須」が未解決に残らない |
| 再発明ゼロ | §2 禁止リスト違反なし |
| 最小出荷 | **0a + 0b + A + B'=N**（0c 前倒し済） |
| 正本単一 | `aiome/commercial/` + aiome 本体（スタンドアロン絶対パスを正本にしない） |
| 購買制御の単一性 | エージェント=MCP whitelist / HTTP=OP-011+eKYC。二重ポリシーなし |
| CI 誠実さ | NurtureEconomy TLC が skip ではなく必須ステップ |

---

## 9. 次アクション（ユーザー向け）

**実装・判断済（2026-07-25）**: Wave 0a / 0b / 0c / A / **B'=N** / **E0=Y + Phase E 親計画**。

残り（実装は [`phase_e_vrm_wiring_plan.md`](phase_e_vrm_wiring_plan.md)）:
1. **E1** 公式素体（Human）→ **E2** VRM 実ロード（明示承認）  
2. **NR-06** BoneChecker コーパス / **NR-07** 有償 KC（Legal）  
3. **E4** Desktop DRM は `src-tauri` 別承認 / **Phase F** Live2D はライセンス後
