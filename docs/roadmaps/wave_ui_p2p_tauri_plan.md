# Wave: UI バックログ整理 + OP-020 再定義 + OP-062（v1.3）

- **ステータス**: Phase A（OP-026）+ Phase D（OP-062）**実装完了**（2026-07-21）。残は任意 Phase C / 後日 F-5
- **目的**: OP-064（Human 進行中）と並行して、Agent が着手可能な次波を実コード根拠で一本化する
- **対象**: OP-020（再定義済）/ OP-021 / OP-022 / OP-026 / OP-062
- **非対象**: OP-087 P4・OP-011・OP-064・Upstream・**OP-020-F5 コード**（別ゲート）
- **正本分担（重複排除）**:
  | 主題 | SSOT | 本書の役割 |
  |------|------|------------|
  | 本 Wave 全体・着手順 | **本ファイル** | ゲート・価値順・抜け漏れ防止 |
  | OP-062 実装手順 | [`remaining_work_foolproof_plan.md`](remaining_work_foolproof_plan.md) §7 Wave G2 | 詳細ステップはそちらへ委譲 |
  | OP-020-F5 実装骨格 | [`h2_f0_f4_f7_implementation_plan.md`](h2_f0_f4_f7_implementation_plan.md) PART 4 | S-1〜S-4 は再記述しない |
  | Federation transport | ADR-053 | 再実装禁止 |

## 0. 実コード監査サマリ（2026-07-21 再検証）

| ID | 実コード | 判定 |
|----|----------|------|
| **OP-021** | `BanDashboard.tsx` + admin bans API。ハードコード英語。`expires_at` は型のみ・**表示なし**。DB 列ありだが `ban()` INSERT が列未書き込み（常に NULL）。`BanRequest` に期限なし | **コア UI ✅**。残 polish=**i18n のみ**。expires=本 Wave 外 |
| **OP-022** | `CausalVisualizer.tsx` + `/api/v1/trajectory/:id`。手動 Job ID 入力。**URL query / Activity 深リンクなし** | **コア UI ✅**。発見性 polish |
| **OP-026** | Probe=`x_signal_probe.rs` ✅。`build_active_trend_sonar` が毎リクエスト/Dream 周期で settings 読込 ✅。UI は `x_bearer_token` のみ。`search_api_key` は ALLOWED_KEYS+SECRETS 済・**別 adapter**（WebSearch/Serp） | **実ギャップ=Channel Bridges 運用 UI** |
| **OP-020** | ADR-053 transport ✅。`HubMessage::SoulSyncRelay` / `paired_devices` **ゼロ** | 再定義済。コードは h2 PART 4 |
| **OP-062** | `NurtureMode::InProcess` ✅（2026-07-21）。既定はなお Local。Desktop 既定化は **OP-088** | **コア完了**。後継: [`desktop_inprocess_default_plan.md`](desktop_inprocess_default_plan.md) |

### 価値順（残コード作業）

```text
1. Phase A  OP-026 Channel Bridges 運用 UI     … 唯一の明確な FE ギャップ
2. Phase D  OP-062 InProcess（明示許可）       … Desktop 品質・Safety-Critical
3. Phase C  polish（任意・低優先）             … 021 i18n / 022 深リンク
4. （後日）OP-020-F5 コード                     … 「OP-020-F5 を実装しろ」
```

Phase L（台帳）・Phase B（再定義 docs）は **完了**。実行順から外す。

---

## 1. Phase L — 台帳衛生 ✅ 完了

OPEN OP-021/022・MEMORY Blind Spot・CHANGELOG 同期済み（2026-07-20）。

---

## 2. Phase A — OP-026 TrendSonar / Channel Bridges 運用 UI

### 2.1 事実（キーと adapter を混同しない）

| 設定キー | Adapter | UI |
|----------|---------|-----|
| `x_bearer_token` | `XSignalProbe` | ✅ Channel Bridges |
| `search_api_key` | `WebSearchAdapter` + `SerpAnalysisAdapter` | ❌ 未配線 |
| 両方 | `build_active_trend_sonar`（`trend_sonar.rs`）が **都度** `get_setting_value` | 再起動不要が実コード根拠 |

**再発明禁止**

- 新規 Probe ページ / 新規疎通 API を作らない
- `/api/v1/settings/test` は **LLM 疎通専用**（`ollama|gemini|openai|anthropic`、`#[cfg(debug_assertions)]` + `AIOME_DEV_MODE=1`）。X/Search には使えない

### 2.2 作業

| ID | 内容 | Effort |
|----|------|--------|
| A1 | Channel Bridges（cockpit 既存セクション）に `search_api_key` を **Bearer と同型**で追加: `SettingInput` + `isPassword` + `integrations` + `VaultKeyStatus(isVaultSet('SEARCH_API_KEY'))`（`ALLOWED_VAULT_SECRETS` 既存）。**i18n 再利用** `settings.searchApiKey` / `searchApiKeyNotice` — 新規キー・新規コンポーネント禁止 | S |
| A2 | `settings.xBearerTokenNotice`（en/ja）を「Dream / trends は動的読込。再起動不要」に修正。`searchApiKeyNotice` は原則維持 | S |
| A3 | **既定スキップ**（明示要求時のみ）: TrendView へのテキスト導線だけ可。新 API / `settings/test` / Trends 再実装禁止 | — |
| A4 | Jest: Channel Bridges に search 入力 + 既存 i18n キー | S |
| A5 | `test_frontend_used_keys_are_allowed` に `x_bearer_token` / `search_api_key` を追加 | S |

**Out of scope**: 専用 Probe ページ、X MCP、レート制限ダッシュボード、`settings/test` 拡張、i18n/Vault キー新規作成、A3 疎通 UI 新設。  
**注意**: CHANGELOG 旧「search_api_key UI 解消」でも **現行 UI なし**。マスク `"••••••••"` 上書き無視は settings.rs 既存 — 再発明しない。

### 2.3 検証

1. Positive: cockpit で `search_api_key` 保存 200 → GET settings でマスク表示。**外部 X/Search 実応答は必須にしない**  
2. Negative: マスク再保存が無視される / 秘密がエラー本文に出ない  
3. Revert: 設定削除で復帰  
4. A5 の Rust テスト GREEN  

**実装許可**: 「OP-026 を実装しろ」または「Phase A を実装しろ」。

---

## 3. Phase B — OP-020-F5 再定義 ✅ 完了（docs）

命名・ゲート・value_10x / h2 / implementation_plan 衝突注記は同期済み。

| 旧称 | 正称 | 意味 |
|------|------|------|
| OP-020「Phase 5 製品 P2P」 | **OP-020-F5 Soul Sync** | クロスデバイス人格同期 |
| implementation_plan Phase 5 | Cognitive Observability | ≠ F-5。触らない |
| ADR-053 | transport ✅ | Karma/Immune/Arena。再実装禁止 |

**コード着手**: 実装骨格は h2 PART 4（S-1〜S-4）。許可フレーズは **「OP-020-F5 を実装しろ」** のみ。本 Wave ではコードを書かない。

**再利用（再発明禁止）**: `ZeroMetadataCommuneRelay` 封筒パターン、`federation.rs` broadcast、`soul_store.record_version`、hub timeline Automerge。**新規必須**は `HubMessage::SoulSyncRelay` のみ（現状ゼロ）。

---

## 4. Phase C — polish（任意・A/D の後）

| ID | 内容 | 注記 |
|----|------|------|
| C1 | Causal: Activity / 履歴 → Job ID 受け渡し（`?id=` または props）。手動入力は残す | 新 API 不要 |
| C2 | Causal: Jest で fetch 成功パス（モック） | 既存 `CausalVisualizer.test.tsx` 拡張 |
| C3 | Ban: ハードコード英語 → i18n | `BanDashboard.tsx` のみ。**admin API 非変更** |
| ~~C4~~ | ~~expires_at~~ | **本 Wave 外**。INSERT が `expires_at` 未書き込み + `BanRequest` 未対応 → BanStore/trait/Mock + admin（Safety-Critical 近傍）。別 OP |

---

## 5. Phase D — OP-062 Tauri `NurtureMode::InProcess`

### 5.1 事実

| 項目 | 状態 |
|------|------|
| ADR | `commercial/docs/decisions/012-agenthook-fire-path-unification.md` Accepted |
| api-server | `bootstrap/plugins.rs` 既存 — **再発明しない** |
| Tauri | `lib.rs` `NurtureMode` / `resolve_nurture_mode` / `start_sidecars` |
| 現行優先 | **Cloud → Disabled → Local**（InProcess なし） |
| Desktop ビルド | `desktop_sidecar_manager.py` が `api-server` を **feature なし**で build |
| `.env.example` | `NURTURE_IN_PROCESS` コメント済 |

### 5.2 実装詳細の委譲

手順・検証表の正本: **foolproof §7 Wave G2**。本書は差分と注意のみ。

| ID | 内容 |
|----|------|
| D1 | `NurtureMode::InProcess` + `start_sidecars` 腕（nurture-api **非 spawn**、`nurture_status=in_process`、api-server に `NURTURE_IN_PROCESS=true`、`NURTURE_API_URL` 非注入） |
| D2 | `resolve_nurture_mode` 優先順位（下記） |
| D3 | `NurtureStatus.mode` / tray / `get_nurture_status` に `"in_process"`。既存 unit test を拡張 |
| D4 | **Q2 必須**: `desktop_sidecar_manager.py` の `api-server` build に `--features nurture`。**入れないと InProcess は env だけで実効ゼロ**（plugins skip 警告）。foolproof G2 検証表にも同項あり |
| D5 | `.env.example`: 既存コメントに「Desktop/Tauri `resolve_nurture_mode` も読む」を追記 |
| D6 | 既存 unit test 更新: Cloud+Disabled 同時 → Disabled 勝ち（挙動変更）。InProcess Positive/Negative 追加 |

### 5.3 優先順位（foolproof と一致・現行との差分を明示）

```text
1. NURTURE_DISABLED=true|1     → Disabled
2. NURTURE_CLOUD_URL 非空      → Cloud(url)
3. NURTURE_IN_PROCESS=true|1   → InProcess   ← 新規
4. else                        → Local
```

**注意**: 現行は Cloud が Disabled より先。foolproof は **kill switch（Disabled）を Cloud より優先**する意図的変更。実装時に既存テスト（Cloud / Disabled）を更新し、両方同時指定の Negative を追加すること。

### 5.4 検証

1. Positive: InProcess → `nurture_child=None` + api-server env  
2. Negative: InProcess 時に nurture-api 二重起動なし  
3. `cargo check -p management-console`  
4. Q2=入れる場合: Desktop ビルド経路で `--features nurture` が付くこと（`desktop_sidecar_manager.py`）

### 5.5 禁止

- CommerceEngine in-process DI、CSP / externalBin の無関係変更  
- api-server Plugin 登録の再実装  
- 「OP-062 を実装しろ」なしの `lib.rs` 編集  

**実装許可**: 明示 **「OP-062 を実装しろ」** + Q2 回答推奨。

---

## 6. 実行順（v1.3）

```text
[済] Phase L / B
  → Phase A  OP-026（FE）
  → Phase D  OP-062（明示許可 + Q2）   ※ A と並列可（別ドメイン）
  →（任意）Phase C polish
  →（後日）OP-020-F5 コード（h2 PART 4 + 明示許可）
```

---

## 7. Open Questions（Human）

| # | 問い | 既定提案 |
|---|------|----------|
| Q1 | OP-020-F5 実装を本四半期に入れるか | **今は再定義のみ** |
| Q2 | OP-062 で Desktop api-server に `--features nurture` を入れるか | **入れる**（入れないと InProcess 無効） |
| Q3 | Ban `expires_at` API 拡張を今やるか | **やらない**（本 Wave 外） |

---

## 8. 着手条件

| フェーズ | 許可フレーズ |
|----------|----------------|
| A | 「OP-026 を実装しろ」 |
| D | 「OP-062 を実装しろ」（+ Q2） |
| C | 「OP-022 polish を実装しろ」等 |
| F-5 コード | 「OP-020-F5 を実装しろ」 |

---

## 9. `/perfect-plan` 検証結果

### Round 1–2（要約）
旧 A3=`settings/test` 誤指定・キー混同・expires 過大・台帳陳腐化・foolproof Q2 抜けを v1.1–v1.2 で解消。

### Round 3（2026-07-21）— 実コード再照合

#### Gate 1: 構造スキャン
- ✅ A1–A5 / D1–D6 が触るシンボルはすべて実在。新規クレート・新規 API なし
- ✅ `SEARCH_API_KEY` は `ALLOWED_VAULT_SECRETS` 済 → Bearer と同型の `VaultKeyStatus` 再利用可（v1.3 で A1 に明記）
- ✅ マスク上書き無視（`••••••••`）は settings.rs 既存 — 再発明不要
- ✅ OPEN / REMAINING_TASKS / foolproof G2 は v1.2 同期済みで再実装誘発なし

#### Gate 2: 要件カバレッジ
- ✅ §2/§3/§4/§8 カバレッジは v1.2 と同じ。追加要件なし
- ✅ A3 を既定スキップにし、TrendView 再発明リスクを除去

#### Gate 3: 依存関係
- ✅ Channel Bridges は cockpit 限定（既存）— A1 も同スコープ
- ✅ OP-062 SSOT 分担（wave ゲート / foolproof 手順）維持。手順の二重記述は差分注記のみ
- ✅ 検証を「外部 API 実応答必須」から外し、偽 Negative を防止

#### Gate 4: 悪魔の弁護人
1. **最悪**: 変わらず Q2 なし InProcess 偽成功 — 計画・foolproof に明記済
2. **誤前提（Round 3 新規）**: 「trends 200 + 実データ」を DoD にすると外部障害で実装失敗扱い → **検証基準を緩和**
3. **やらないメリット**: 計画は十分。これ以上の docs 磨きは収穫逓減 — **実装許可待ちが正解**

#### Gate 5: 実行順序
- ✅ A(必須 A1/A2/A4/A5) → D（∥可）→ C → F-5。矛盾なし

### 判定
- [x] ✅ **PASS** — v1.3 微修正のみ。抜け漏れ・重複・車輪の再開発は実務上クリア。フェーズ許可後に実行してよい
