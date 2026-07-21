# Wave: UI バックログ整理 + OP-020 再定義 + OP-062（v1.5）

- **ステータス**: Phase A / D / C3 / **C1–C2（OP-022）✅ 2026-07-22**。残は後日 F-5 のみ（本 Wave polish 完了）
- **目的**: OP-064（Human）と並行して、Agent が着手可能な次波を実コード根拠で一本化する
- **対象**: OP-020（再定義済）/ OP-021 ✅ / OP-022 ✅ / OP-026 ✅ / OP-062 ✅
- **非対象**: OP-087 Q5/Q6・OP-011・OP-064・Upstream・**OP-020-F5 コード**（別ゲート）・Ban `expires_at`
- **正本分担（重複排除）**:
  | 主題 | SSOT | 本書の役割 |
  |------|------|------------|
  | 本 Wave 全体・着手順 | **本ファイル** | ゲート・価値順・抜け漏れ防止 |
  | OP-062 実装手順 | foolproof §7 G2 / **OP-088** | 完了済。再実装禁止 |
  | OP-020-F5 実装骨格 | [`h2_f0_f4_f7_implementation_plan.md`](h2_f0_f4_f7_implementation_plan.md) PART 4 | S-1〜S-4 は再記述しない |
  | Federation transport | ADR-053 | 再実装禁止 |
  | FE 画面遷移バス | `a2ui-navigate` + `a2uiTabs.ts` | **新規 Router / URL クエリ層を作らない** |
  | FE i18n ランタイム | `apps/management-console/src/i18n/` | 新規 i18n 機構禁止 |

## 0. 実コード監査サマリ（2026-07-22 再検証）

| ID | 実コード | 判定 |
|----|----------|------|
| **OP-021** | `ban.*` i18n + `/reflexion` Negative ✅ | **完了** |
| **OP-022** | Causal + trajectory API。C1–C2: Timeline → `dispatchA2uiNavigate` + sessionStorage。手動入力維持 | **完了**（§4.2） |
| **OP-026** | Channel Bridges `search_api_key` + VaultKeyStatus ✅ 2026-07-21 | **完了**（A3 疎通 UI は既定スキップのまま） |
| **OP-020** | ADR-053 transport ✅。`SoulSyncRelay` ゼロ | 再定義済。コードは別ゲート |
| **OP-062 / OP-088** | Desktop 既定 InProcess ✅ | **完了**（本 Wave D 再実装禁止） |

### 価値順（**現時点の残**）

```text
1. （後日）OP-020-F5 コード                            … 「OP-020-F5 を実装しろ」
2. OPEN 他レーン（本書外）: OP-087 Q5/Q6 / OP-011 / OP-027 / OP-064 / Upstream
```

~~Phase A / D / C3 / C1–C2~~ は完了。実行順から外す。
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

## 4. Phase C — polish（任意）

| ID | 内容 | 注記 |
|----|------|------|
| C1 | Causal: Timeline / 他画面 → Job ID 受け渡し。手動入力は残す | ✅ 2026-07-22（§4.2） |
| C2 | Causal: Jest で fetch 成功 / 失敗パス（モック） | ✅ 2026-07-22（§4.2） |
| C3 | Ban: ハードコード英語 → i18n | ✅ 2026-07-21（§4.1） |
| ~~C4~~ | ~~expires_at~~ | **本 Wave 外**（BanStore/admin）。別 OP |

### 4.1 C3 / OP-021 i18n — 実装契約（2026-07-21 実コード検証）

#### 事実（推測禁止）

| 項目 | 根拠 |
|------|------|
| 未 i18n | `BanDashboard.tsx` 全文ハードコード英語（`useTranslation` なし） |
| シェルは済 | `AppHeader` → `t('page.banDashboard')` / `navConfig` → `nav.banDashboard` / `nav.desc.ban-dashboard` / `page.desc.ban-dashboard` は **en+ja 既存** |
| ランタイム | `useTranslation` + nested JSON（`src/i18n/{en,ja}.json`）。parity = `i18n.test.ts` |
| 同型先例 | `CausalVisualizer`（`causal.*`）/ `McpDashboard`（`mcp.*` + 一部 `common.*`） |
| Jest 先例 | `CausalVisualizer.test.tsx`: `t: (key) => key` でキー文字列を assert（**LanguageProvider 不要**） |
| ConfirmModal | 既存再利用済み。Vault は `vault.modal.cancel`。**`common.cancel` は未定義**（`mcp.cancel` / `vault.modal.cancel` のみ） |
| API | `POST /admin/ban` body の `severity` は `LOW\|MEDIUM\|HIGH\|CRITICAL`。空 reason 時デフォルト `"Policy violation"`（英語定数） |

#### 再利用（再発明禁止）

| やること | やらないこと |
|----------|----------------|
| `import { useTranslation } from '../i18n'` | 新規 i18n ライブラリ / Provider ラップ専用コンポーネント |
| ルート名前空間 **`ban.*`**（`causal` / `mcp` と同型） | `nav.*` / `page.*` の二重定義や上書き |
| 汎用ネットワーク toast → **`common.networkError`**（既存） | fetch/ban/unban 用に 3 つの類似 network キーを新設 |
| 汎用 Cancel → **`common.cancel` を en/ja に 1 回追加**して ConfirmModal から利用 | `ban.cancel` 専用キー、または `vault.modal.cancel` のドメイン横断参照 |
| Jest: `jest.mock('../i18n', () => ({ useTranslation: () => ({ t: (k) => k }) }))` | 実辞書を読む LanguageProvider 二重経路 |
| ConfirmModal / Toast / authenticatedFetch 既存 | 新規モーダル・toast 機構 |

#### キー棚卸し（en/ja 同時・parity 必須・1:1）

現行 `BanDashboard.tsx` リテラル → キー（抜け漏れ防止の正本）。**`ban` 名前空間は現時点で en/ja に未存在**（再検証済）。

| 現行英語 | キー |
|----------|------|
| Governance & BAN Compliance Registry | `ban.title`（※`page.banDashboard`≠流用） |
| Mathematically enforced account suspensions… | `ban.subtitle` |
| Issue Policy Suspension | `ban.formTitle` |
| Target Agent UUID | `ban.targetLabel` |
| Violation Reason | `ban.reasonLabel` |
| Describe policy violation (e.g., CSAM…) | `ban.reasonPlaceholder` |
| Severity Level | `ban.severityLabel` |
| LOW — Notice / Cooldown | `ban.severityLow`（`value="LOW"` 維持） |
| MEDIUM — Minor Restriction | `ban.severityMedium` |
| HIGH — High Alert Penalty | `ban.severityHigh` |
| CRITICAL — Hard Permanent BAN | `ban.severityCritical` |
| Enforce Suspension / Executing Suspension... | `ban.submit` / `ban.submitting` |
| Search by UUID or reason... | `ban.searchPlaceholder` |
| Loading compliance registry... | `ban.loading` |
| No active suspensions. Workspace is fully compliant. | `ban.empty` |
| Issued: / Lifted: | `ban.issued` / `ban.lifted` |
| Unban（ボタン + confirmText） | `ban.unban` |
| Restore Agent Access | `ban.confirmTitle` |
| Are you sure you want to restore and unban this agent? | `ban.confirmMessage` |
| Cancel | `common.cancel`（**新設**・`mcp.cancel`/`vault.modal.cancel` は触らない） |
| Failed to fetch ban list. Admin credentials required. | `ban.errorFetch` |
| Target Agent UUID is required. | `ban.errorTargetRequired` |
| Failed to execute ban. / Failed to lift suspension. | `ban.errorBan` / `ban.errorUnban` |
| Agent successfully suspended. / reinstated. | `ban.successBan` / `ban.successUnban` |
| Network error occurred while fetching bans. ほか 2 本 | **`common.networkError` に統合**（既存。文言は汎用に寄せる） |

キー数目安: `ban.*` **26** + `common.cancel` **1**（en/ja 同数で `i18n.test.ts` parity）。

**ハードコード維持（意図的・i18n しない）**

| 項目 | 理由 |
|------|------|
| UUID placeholder `00000000-…` | ロケール非依存の形式見本 |
| API `severity` value / デフォルト reason `"Policy violation"` | **リクエスト本文をロケールで変えない** |
| `showToast` 第1引数 `"error"`/`"success"` | 型トークン（表示文言ではない） |
| `data.message`（サーバ） | API 応答のまま。フォールバックのみ `ban.error*` |
| レコードの `reason` / `actor_id` / バッジの severity 生値 | ユーザ/サーバ由来 |
| `expires_at` UI | C4 本 Wave 外 |

#### 変更ファイル（最大）

1. `BanDashboard.tsx` — `t('ban.*')` / `t('common.*')` 配線のみ  
2. `en.json` + `ja.json` — `ban` オブジェクト + `common.cancel`  
3. `BanDashboard.test.tsx` — i18n mock（Causal 同型）+ assert をキー文字列へ。**ConfirmModal mock の Cancel リテラルを `cancelText` prop 表示に修正**（現状 mock が `Cancel` 固定で prop を無視）

**触らない**: admin routes / BanStore / Immune / AppHeader / navConfig / OP-022 / `mcp.cancel`・`vault.modal.cancel` の一括置換

#### DoD

1. 上表の「現行英語」がコンポーネントから消え、対応キーのみ（維持表を除く）  
2. `BanDashboard` Jest + `i18n.test.ts` parity PASS  
3. Positive: mock で `ban.title` 表示 / 任意で LanguageProvider+ja 煙  
4. Negative: 欠キー → 既存どおり en フォールバック or key 文字（新機構なし）  
5. OPEN OP-021 クローズ + CHANGELOG 1 行（実装時）

#### 着手フレーズ

「OP-021 を実装しろ」または「C3 を実装しろ」

### 4.2 C1–C2 / OP-022 Causal polish — 実装契約（2026-07-22 実コード検証）

#### 事実（推測禁止）

| 項目 | 根拠 |
|------|------|
| API | `GET /api/v1/trajectory/{id}` + `.../diagnosis`（`routes/jobs.rs`）。**Job 一覧 API は本 polish で新設しない** |
| UI | `CausalVisualizer.tsx` — 手動 `jobId` + `validateAndFetch`（`/^[a-zA-Z0-9_\-]+$/`）+ `authenticatedFetch` |
| 二重マウント | `AppRoutes` `activeTab==="causal"` **と** `HomePage` `observeSubTab==="trace"` |
| 画面遷移 | React Router / `URLSearchParams` **なし**。タブは `App` の `activeTab` state + **`a2ui-navigate` CustomEvent**（`detail.tab`、`isValidA2uiNavTab`） |
| Activity | `ActivityView` = Timeline / Diagnostics / PromptStats。`Timeline` は karma 行に `job_id` を**表示のみ**（クリックなし） |
| Jest | `CausalVisualizer.test.tsx` — レンダー / 入力 / overlay 配置のみ。**fetch 成功パスなし** |
| i18n | `causal.*` 済。ハードコード残: `"Failed to fetch trajectory"` / `'Unknown error'`（任意で `causal.fetchFailed` 等へ。C1 必須ではない） |

#### 旧 C1 文言の訂正（車輪の再発明防止）

| 旧計画 | 問題 | 固定方針 |
|--------|------|----------|
| `?id=` URL クエリ | App にクエリ層がない。導入は Router 新設に近い | **やらない** |
| 「Activity 連携」曖昧 | ActivityView 自体に job 導線なし。実データは **Timeline の `job_id`** | Timeline → Causal を第一導線 |
| props のみ | 二重マウント + App 非接続で props 貫通が重い | **既存イベントバス拡張**を優先 |

#### 再利用（再発明禁止）

| やること | やらないこと |
|----------|----------------|
| `a2ui-navigate` の `detail` を `{ tab: string, jobId?: string }` に**後方互換拡張** | react-router / hash ルーティング新設 |
| Causal 側でイベント購読 **または** `sessionStorage` キー 1 つ（例: `aiome.causalJobId`）で初期 ID 受領 | グローバル Context / Redux 新設 |
| `Timeline` で `job_id` がある行だけクリック可能にし `tab: 'causal'` + `jobId` を dispatch | 新規「Job ピッカー」画面・jobs 一覧 API |
| 手動入力 + Enter / ボタンは**維持** | 手動入力削除 |
| C2: `authenticatedFetch` mock で trajectory JSON 成功パス | vis-network 実描画の E2E 必須化 |
| 既存 `validateAndFetch` / trajectory・diagnosis エンドポイント | 新 diagnosis UI・新 graph ライブラリ |

#### C1 作業分割

| ステップ | 内容 | ファイル目安 |
|----------|------|----------------|
| C1-a | `a2ui-navigate` detail 型を文書化（`a2uiTabs.ts` 近傍 or コメント）。`App.tsx` は `tab` のみでも動作継続 | `a2uiTabs.ts` / `App.tsx`（jobId を無視してよい） |
| C1-b | `CausalVisualizer`: mount 時 + イベントで `jobId` を受けたら `setJobId` → `validateAndFetch`。無効 ID は既存 invalid メッセージ | `CausalVisualizer.tsx` |
| C1-c | `Timeline`: `job_id` ありの karma 行に「Trace」操作（button）。`dispatchEvent(a2ui-navigate, { tab:'causal', jobId })` | `Timeline.tsx` + 最小 i18n 1 キー可（`timeline.openCausal`） |
| C1-d | Home `trace` 埋め込みでも同じイベントを聴く（二重マウント耐性）。cockpit `causal` タブへ寄せるなら `tab:'causal'` で十分 | 新 Home 状態機械は作らない |

#### C2 検証契約

| 種別 | 内容 |
|------|------|
| Positive | mock `trajectory/{id}` 200 + nodes → fetch が呼ばれ loading が終わり（空状態 `causal.enterJobId` が消えるか、エラーが無い） |
| Negative | 不正 ID → `causal.invalidJobId`（fetch 未呼出）。`!ok` → エラー表示（文言は現行英語でも可） |
| 非目標 | Network 実サーバ、vis-network ノード数の DOM assert |

#### 変更ファイル（最大）

1. `CausalVisualizer.tsx`（+ 任意 i18n エラーキー）  
2. `Timeline.tsx`（+ 任意 `timeline.openCausal`）  
3. `a2uiTabs.ts` または小さな型コメント（detail 拡張の正本）  
4. `CausalVisualizer.test.tsx` / `Timeline.test.tsx`（クリック導線）  
5. `App.tsx` は **変更最小**（tab 切替既存で足りるなら触らない）

**触らない**: trajectory/diagnosis ハンドラ、Ban、OP-020-F5、React Router、jobs 一覧 API、OP-087

#### DoD

1. Timeline の `job_id` 付き行から Causal が開き、同じ ID で fetch が走る（手動入力も残る）  
2. C2 Jest Positive + Negative PASS  
3. 新 Router / 新 API ゼロ  
4. OPEN OP-022 更新 + CHANGELOG 1 行（実装時）

#### 着手フレーズ

「OP-022 を実装しろ」または「C1–C2 を実装しろ」／「OP-022 polish を実装しろ」

---

## 5. Phase D — OP-062 Tauri `NurtureMode::InProcess`（✅ 完了・履歴）

> **2026-07-21 以降**: OP-062 コア + **OP-088** Desktop 既定 InProcess でクローズ。以下は当時の差分メモ。**再実装しない**。

### 5.1 事実（完了時点の記録）

| 項目 | 状態（計画当時 → 現在） |
|------|------|
| ADR | `012-agenthook-fire-path-unification` Accepted |
| 現在の正本 | OP-088 / `desktop_inprocess_default_plan.md` / Settings Mode UI |
| Desktop | 既定 InProcess。公式 sidecar は api-server+key-proxy（nurture-api 非同梱） |

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

**実装許可**: ~~「OP-062 を実装しろ」~~ → **完了**（OP-088 含む）。再実行禁止。

---

## 6. 実行順（v1.5）

```text
[済] Phase L / B / A / D / C3 / C1–C2（OP-022）
  →（後日）OP-020-F5 コード（h2 PART 4 + 明示許可）
  →（本書外）OP-087 Q5/Q6 / OP-011 / OP-027 / OP-064 / Upstream
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
| C3 / OP-021 | ✅ 完了 |
| C1–C2 / OP-022 | ✅ 完了（2026-07-22） |
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

### Round 4（2026-07-21）— OP-021 / C3 単体ブラッシュアップ

| 検出 | 対応（§4.1） |
|------|----------------|
| シェル（nav/page）は既に i18n 済なのに C3 が「Ban 全体」と読める | 対象を **コンポーネント内リテラルのみ** に限定 |
| `ban.cancel` 新設だと `mcp.cancel` / `vault.modal.cancel` と三重 | **`common.cancel` を共通新設**（既存ドメインキーは触らない） |
| network toast 3 文言の複製 | **`common.networkError` 再利用** |
| API default reason / severity value を `t()` すると ja で本文が変わる | **リクエスト本文は英語定数維持** |
| LanguageProvider ラップを新規導入しがち | **Causal と同型の key-assert mock** |
| `page.banDashboard` を h3 に流用するとヘッダと衝突・文言後退 | **`ban.title` を別キー**（Causal と同型） |

判定: [x] ✅ **PASS** — C3 は §4.1 契約で実装可。コード変更は明示許可後。

### Round 4b（2026-07-21）— C3 文字列 1:1 再監査

- ✅ `BanDashboard.tsx` ユーザー向けリテラルを列挙し §4.1 表と突合（成功 toast / severity 4 option / Issued·Lifted 含む）
- ✅ `ban` 名前空間・`common.cancel` は辞書未存在（新設のみ。既存 nav/page に触れない）
- ✅ network 3 文言 → `common.networkError` 統合は意図的トレードオフ（再発明回避）
- ✅ テスト側 ConfirmModal mock の `Cancel` 固定を契約に明記（実装時に `cancelText` へ）
- 判定: [x] ✅ **PASS** — 追加の計画肥大化は不要。実装許可待ち

### Round 5（2026-07-22）— 残存フェーズ（OP-022）実コード再検証

| 検出 | 対応（§4.2 / v1.5） |
|------|---------------------|
| 価値順が A/D 未完了のまま | A/D/C3 完了済みに更新。残=C1–C2 のみ |
| C1 の `?id=` が Router 新設を誘発 | **禁止**。`a2ui-navigate` + 任意 sessionStorage |
| 「Activity 連携」が ActivityView 全体に聞こえる | 実導線は **Timeline.`job_id`** に限定 |
| Causal 二重マウント未言及 | Home `trace` + AppRoutes `causal` を契約に明記 |
| Job ピッカー / 一覧 API を勝手に広げうる | **Out of scope** 明示 |
| C2 が「成功パス」だけで Negative 欠落 | invalid ID + `!ok` を契約化 |

判定: [x] ✅ **PASS** — OP-022 は §4.2 で実装可。コード変更は明示許可後。
