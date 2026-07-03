# 実装計画書: F-1 Agent Playbooks — 業務テンプレートのワンクリック導入

**作成日**: 2026-07-03
**対象**: `value_10x_roadmap.md` F-1（Horizon 1 / 推奨着手順 1）
**性質**: 純粋な機能追加。既存 API・既存 UI の挙動は一切変更しない（SetupWizard への「追加ステップ」を除く）。
**成果物**: Playbook マニフェスト型（infrastructure）＋ 公式 Playbook 4本（同梱アセット）＋ import/install/export API（api-server）＋ SetupWizard の Playbook 選択ステップ（management-console）。

---

## 1. 現状理解（実行者への文脈共有）

### 1-1. F-1 が何を作るか

「SEO 運用」等の業務一式（ワークフロー定義＋必要スキル・MCP の宣言）を1つの **Playbook マニフェスト（JSON）** としてパッケージ化し、
(a) バイナリ同梱の公式 Playbook をワンクリック導入（install）、(b) 任意マニフェストの import、(c) 既存ワークフローの export を可能にする。
SetupWizard の初期化完了直後に Playbook 選択画面を挟み、「空のエージェント」問題を解消する。

### 1-2. 利用する既存資産（2026-07-03 実在確認済み・行番号は当日時点）

| 資産 | 場所 | 本計画での使い方 |
|---|---|---|
| workflows テーブル | `libs/infrastructure/migrations/sqlite/20260614000000_workflows.sql` L3-18 | import 先。`visibility='private'` で通常ワークフローとして作成 |
| `WorkflowStore` | `libs/infrastructure/src/workflow/store.rs`（`create_workflow` L52、`save_version` L167、`delete_workflow` L159） | import 実装の中核。**新メソッドは追加しない** |
| `WorkflowDefinition` / `NodeType` | `libs/infrastructure/src/workflow/schema.rs` L14-122 | マニフェストに内包する定義そのもの |
| `WorkflowValidator` + `DefaultConstitutionalValidator` | `routes/workflow.rs` の `validate_workflow` L287-301 が利用例 | import 時に各ワークフローを検証（憲法検査・SSRF 検査込み） |
| workflow ルート配線 | `apps/api-server/src/router.rs` L478-495（`internal_router`、L701 で auth 必須） | playbook ルートも同じ場所に配線 |
| 埋め込みシードの参照パターン | `apps/api-server/src/mcp/discovery.rs` L35-49 | 参考のみ。公式 Playbook は `include_str!` 同梱とし、`~/.aiome` への書き出しは**しない** |
| API 新設の必須4点セット | `.agent/skills/api-route-wiring-check.md` | ハンドラ実装・router 配線・OpenAPI 登録（`api.rs` の `paths(...)`）・テスト Mock 同期 |
| 統合テストの雛形 | `apps/api-server/src/api_integration_tests/workflow.rs`（`test_workflow_routes_auth` L15 ほか5本） | `playbook.rs` を同形式で新設 |
| SetupWizard | `apps/management-console/src/components/SetupWizard.tsx`（`step: number` L44、`handleFinalize` L66-125） | 成功後 `window.location.reload()`（L111）の**手前**に Playbook 選択ステップを挿入 |
| 認証付き fetch | `src/lib/auth.ts` `authenticatedFetch` L39-61 | Playbook API 呼び出しに使用（`setup/init` 成功時に `setAuthToken` 済みのため使用可能） |
| i18n | `src/i18n/ja.json` / `en.json` の `setup.*` ネームスペース | 新キーは `setup.playbook*` に置く（`setupWizard` キーは存在しない） |
| フロントテスト | `SetupWizard.test.tsx`（6本、`global.fetch` を jest.Mock 化） | 既存6本を GREEN 維持しつつ Playbook ステップのテストを追加 |

### 1-3. 実行者が知らないと事故る事実（必読）

1. **`is_template` 列は SELECT のみで INSERT/UPDATE 経路が存在しない**（`store.rs` 全体を確認済み）。本計画では `is_template` を**使わない**。公式 Playbook は DB 行ではなくバイナリ同梱 JSON であり、import されたワークフローは通常の private ワークフローになる。`is_template` の書込対応は F-3（Marketplace）のスコープ。
2. **フロントの validate 呼び出しは `POST /api/v1/workflows/validate`（`useWorkflowApi.ts` L75）だが、バックエンドのルートは `/api/v1/workflows/:id/validate`（router.rs L492）** という既存の不整合がある。これは**本計画で修正しない**（P-7 で OPEN.md に記録のみ）。
3. **workflow 系ルートは OpenAPI（`api.rs`）に未登録**。playbook ルートは4点セットに従い**登録する**（既存 workflow ルートの登録は本計画のスコープ外）。
4. SetupWizard は**ステップ配列を持たない**。`useState(0)` の数値 `step` と `{step === N && (...)}` の条件分岐（L167-480）、プログレスドットは固定配列 `[0,1,2,3,4]`（L156）。Playbook ステップは step 5（Finalizing）と同様に**ドット対象外の step 6** として追加する。
5. **認証トークンは `setup/init` の成功レスポンスで初めて手に入る**（L107 で `setAuthToken`）。よって Playbook 選択ステップは初期化成功の**後**にしか置けない。playbook ルートは auth 必須の `internal_router` に配線して問題ない。
6. `SubWorkflow` ノードは他ワークフローを UUID で参照する。マニフェスト内 UUID と import 後の新規 UUID の付け替え（remap）は複雑なため、**v1 マニフェストでは `SubWorkflow` ノードを含む Playbook を 400 で拒否**する（1-5 のやらないこと参照）。
7. pre-push フックが `cargo fmt --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`cargo audit`・全テストを強制する。フロントは `npx jest` が CI 対象。
8. AGENTS.md 準拠: 本番コードで `unwrap()`/`expect()` 禁止、テストの assert 緩和禁止、`AiomeError` メッセージはデバッグに十分な文脈を含めること。

### 1-4. 新規 API 契約（本計画で確定。実行者は変更しない）

| メソッド | パス | 認証 | 入出力 |
|---|---|---|---|
| GET | `/api/v1/playbooks` | 必須 | → `200 [PlaybookSummary]`（同梱4本の一覧） |
| POST | `/api/v1/playbooks/:id/install` | 必須 | → `200 PlaybookInstallResponse` / `404`（未知 id）/ `422`（依存欠落） |
| POST | `/api/v1/playbooks/import` | 必須 | body=`PlaybookManifest` → `200 PlaybookInstallResponse` / `400`（構造違反）/ `422`（依存欠落） |
| GET | `/api/v1/workflows/:id/export` | 必須 | → `200 PlaybookManifest`（単一ワークフロー）/ `404` |

エラー時のボディ規約:
- `400`: `AppError::bad_request` に**違反内容を列挙した文字列**（例: `"playbook validation failed: id must match ^[a-z0-9-]{1,64}$; workflows must be 1..=10"`）。
- `422`: JSON `{"missing_skills": ["..."], "missing_mcp_servers": ["..."]}`（受け入れ基準2「具体的な欠落一覧を返して失敗」の実装。サイレント部分適用禁止）。

### 1-5. マニフェスト形式 v1（本計画で確定）

```json
{
  "playbook_version": 1,
  "id": "seo-operations",
  "name": "SEO 運用",
  "description": "サイトの SEO 監査と改善提案を定期実行する",
  "tags": ["seo"],
  "required_skills": [],
  "required_mcp_servers": [],
  "workflows": [ /* WorkflowDefinition の配列（1〜10個） */ ]
}
```

構造バリデーション規則（`PlaybookManifest::validate_structure`）:
- `playbook_version == 1` / `id` は `^[a-z0-9-]{1,64}$`（パストラバーサル対策。受け入れ基準3）/ `name` 1〜100文字 / `description` 1000文字以下
- `workflows` は 1〜10 個、各 `WorkflowDefinition` に `NodeType::SubWorkflow` を含まない
- 違反時は `AiomeError`（既存の validation 系バリアント）で**全違反を列挙**して返す

---

## 2. 項目0: 安全網の構築（最初に必ず実行）

### 0-a. 作業前コミット

```bash
cd /Users/motista/Desktop/antigravity/aiome
git status   # 未コミットの変更が「ない」ことを確認。あれば中断してユーザーに報告
git checkout -b feature/f1-agent-playbooks
git log -1 --oneline   # 開始点のハッシュを作業メモに記録
```

### 0-b. ベースライン確認

```bash
cargo check --workspace --tests
cargo test -p infrastructure workflow          # schema/validator/transpiler/store の既存テスト
cargo test -p api-server api_integration_tests::workflow   # 統合テスト5本
cargo clippy -p infrastructure -p api-server --all-targets -- -D warnings
cd apps/management-console && npx jest SetupWizard workflowConverter && cd ../..
```

すべて成功しない場合、**実装を開始せず**失敗内容をユーザーに報告して中断する（開始前から壊れているものの修理はスコープ外）。
`test result` 行と jest のサマリを記録し、以降の各項目で「passed 数が同数以上・failed 0」と比較する。

### 0-c. ベースラインの性質

本計画は新機能追加であり、既存挙動の特性テストは 0-b の既存スイートで足りる。新規コードのテストは各項目内で**実装と同一コミットで**追加する（RED を先に書ける項目は P-1・P-4 に明記）。

---

## 3. 作業項目リスト（実行順）

> 共通ルール: 1項目 = 1コミット（メッセージは `feat(playbooks): <項目ID> <要約>`）。完了条件をすべて満たしてから次へ。行番号は変動するため各項目のアンカー文字列で再特定する。

---

### P-1: `PlaybookManifest` 型と構造バリデーション（infrastructure）

- **対象**: `libs/infrastructure/src/workflow/playbook.rs`（新規）、`libs/infrastructure/src/workflow/mod.rs`（`pub mod playbook;` の1行追加のみ）
- **目的**: マニフェストのパース・構造検証を api-server から分離した純粋ロジックとして置く。
- **変更**:
  1. 1-5 の形式どおりの `PlaybookManifest` を定義（`serde::{Serialize, Deserialize}` + `schemars::JsonSchema` は不要、`utoipa::ToSchema` は P-3 で api-server 側 DTO に付けるためここでは付けない）:

```rust
use super::schema::{NodeType, WorkflowDefinition};
use aiome_core::error::AiomeError; // store.rs L9 と同じ既存パス（実在確認済み）

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlaybookManifest {
    pub playbook_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub required_skills: Vec<String>,
    #[serde(default)]
    pub required_mcp_servers: Vec<String>,
    pub workflows: Vec<WorkflowDefinition>,
}

impl PlaybookManifest {
    /// 1-5 の規則をすべて検査し、違反を「全件列挙」した Err を返す
    pub fn validate_structure(&self) -> Result<(), AiomeError> { /* 違反を Vec<String> に集めて join */ }
}
```

  2. id 検査は `regex` クレート既存依存を使わず `chars().all(...)` で実装してよい（依存追加禁止のため。`regex` が infrastructure の既存依存に**ある場合のみ**使用可）。
  3. `SubWorkflow` 検出は `self.workflows.iter().flat_map(|w| &w.nodes).any(|n| matches!(n.node_type, NodeType::SubWorkflow { .. }))`。
  4. 同ファイル末尾に `#[cfg(test)] mod tests` を置き、**先に RED で書いてから実装**する: `test_playbook_valid_manifest_ok` / `test_playbook_rejects_bad_id`（`../../etc`・大文字・65文字）/ `test_playbook_rejects_subworkflow_node` / `test_playbook_rejects_empty_and_11_workflows` / `test_playbook_error_lists_all_violations`（複数違反が1つの Err に列挙される）。
- **完了条件**: `cargo test -p infrastructure workflow::playbook` で新テスト5本 PASS、`cargo check --workspace --tests` 成功、clippy 成功、0-b の既存テストと同数以上 PASS。
- **リスク**: 低（追加のみ・既存コード非接触）。失敗時は `git checkout -- libs/infrastructure/src/workflow/` で戻す。
- **依存**: 項目0

---

### P-2: 公式 Playbook アセット4本の同梱（api-server）

- **対象**: `apps/api-server/assets/playbooks/`（新規ディレクトリ）に `seo-operations.json` / `sns-operations.json` / `competitor-research.json` / `support-triage.json` の4ファイル、`apps/api-server/src/routes/playbook.rs`（新規、この項目ではレジストリ部分のみ）
- **目的**: バイナリ同梱（`include_str!`）で改ざん・欠落なく公式 Playbook を配布する。`~/.aiome` への書き出しはしない（discovery.rs 方式は採らない）。
- **変更**:
  1. 各 JSON は 1-5 形式のマニフェスト。ワークフローは既存 `NodeType` のみで構成し、**トリガーは全て `Manual`**（Cron スケジューラは存在しないため。1-3 参照）。最小構成の指針:
     - `seo-operations`: Start(Manual) → LlmPrompt（監査観点の生成）→ HttpRequest（GET、`variables.target_url` 参照）→ LlmPrompt（改善提案）
     - `sns-operations`: Start → LlmPrompt（ドラフト生成）→ HumanApproval（`prompt_message` に投稿文レビュー依頼）
     - `competitor-research`: Start → HttpRequest → Transform → LlmPrompt（要約）
     - `support-triage`: Start → LlmPrompt（分類）→ Condition(LlmJudge) → HumanApproval（エスカレーション側の分岐）
     - `required_skills` / `required_mcp_servers` は4本とも**空配列**とする（Mock Commerce・素の環境で完走可能にするため。受け入れ基準4）。
     - 各 `WorkflowDefinition` の `id` は有効な UUID 文字列、`version: 1`、`created_at`/`updated_at` は固定文字列で可（import 時に使われない）。ノードの `position` は `{x, y}` を適当な格子で埋める。
  2. `playbook.rs` にレジストリを実装:

```rust
static BUNDLED_PLAYBOOKS: &[(&str, &str)] = &[
    ("seo-operations", include_str!("../../assets/playbooks/seo-operations.json")),
    ("sns-operations", include_str!("../../assets/playbooks/sns-operations.json")),
    ("competitor-research", include_str!("../../assets/playbooks/competitor-research.json")),
    ("support-triage", include_str!("../../assets/playbooks/support-triage.json")),
];

pub(crate) fn load_bundled() -> Vec<PlaybookManifest> { /* serde_json::from_str。パース失敗はスキップせず配列から除外+warn!（unwrap 禁止） */ }
```

  3. `apps/api-server/src/routes/mod.rs`（アンカー: 既存の `pub mod workflow;`）に `pub mod playbook;` を追加。
  4. テスト（`playbook.rs` 内 `#[cfg(test)]`）: `test_bundled_playbooks_all_parse_and_validate` — 4本すべてが `from_str` 成功かつ `validate_structure()` OK かつ全 Start ノードが `Manual` トリガーであること。**このテストがアセットの品質ゲート**。
- **完了条件**: `cargo test -p api-server playbook` で新テスト PASS、`cargo check --workspace --tests` 成功、clippy 成功。
- **リスク**: 低〜中（アセット JSON の手書きミス）。テストが構造を検証するため検出可能。失敗時はコミットを `git revert`。
- **依存**: P-1

---

### P-3: ハンドラ実装 — list / install / import / export（api-server）

- **対象**: `apps/api-server/src/routes/playbook.rs`（P-2 のファイルに追記）、`apps/api-server/src/routes/workflow.rs`（`export_workflow` を1ハンドラ追加）
- **目的**: 1-4 の API 契約を実装する。
- **変更**:
  1. DTO（`utoipa::ToSchema` derive を付ける）:

```rust
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PlaybookSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub workflow_count: usize,
    pub required_skills: Vec<String>,
    pub required_mcp_servers: Vec<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PlaybookInstallResponse {
    pub playbook_id: String,
    pub created_workflow_ids: Vec<String>,
}
```

  2. `list_playbooks`: `load_bundled()` を `PlaybookSummary` に写像して返すだけ。
  3. `install_playbook(Path(id))`: `load_bundled()` から `id` 一致を検索（無ければ 404）→ 共通関数 `install_manifest` へ。
  4. `import_playbook(Json(manifest))`: `manifest.validate_structure()`（違反は `AppError::bad_request`）→ `install_manifest` へ。
  5. 共通関数 `install_manifest(state, auth, manifest) -> Result<PlaybookInstallResponse, AppError>` の処理順（**この順序を変えない**）:
     - (a) **依存検査**: `required_skills` を実在スキル一覧と突合（スキル一覧の取得方法は `routes/skill.rs` の `list_skills` ハンドラが `AppState` から取得している方法をそのまま踏襲する — アンカー: `pub async fn list_skills`）。`required_mcp_servers` は `~/.aiome/mcp_servers.json` のキーと突合（読み方は `mcp/discovery.rs` L35-36 のパス解決を踏襲。ファイル不存在は「全 MCP 欠落」として扱う）。欠落があれば `422` で `{"missing_skills": [...], "missing_mcp_servers": [...]}` を返し、**DB には一切書かない**。
     - (b) **全ワークフローの事前検証**: 各 `WorkflowDefinition` を `routes/workflow.rs` の `validate_workflow`（アンカー: `WorkflowValidator::validate`）と同一の方法で検証。1つでも失敗したら `400`（どのワークフローが失敗したか名前を含める）で、**DB には一切書かない**。
     - (c) **作成**: 各ワークフローについて `Uuid::new_v4()` で**新規 ID を採番**し（マニフェスト内の id は使わない）、`store.create_workflow(new_id, creator_id, &wf.name, &wf.description, "private", manifest.tags.clone())` → `store.save_version(new_id, 1, &wf_with_new_id, "Imported from playbook <manifest.id>")`。`creator_id` の取得は `routes/workflow.rs` の `create_workflow` ハンドラ（アンカー: `pub async fn create_workflow`）と同一の方法を踏襲。保存する定義は `id` フィールドを新 ID に差し替えたコピーとする。
     - (d) **失敗時ロールバック**: (c) の途中でエラーが出たら、そこまでに作成した workflow を `store.delete_workflow` でベストエフォート削除してからエラーを返す（部分適用を残さない。削除失敗は `warn!` ログのみ）。
  6. `export_workflow(Path(id))`（`routes/workflow.rs` に追加。アンカー: `pub async fn fork_workflow` の直後）: `get_workflow`（無ければ 404）→ `get_version(id, current_version)` → 単一ワークフローの `PlaybookManifest`（`playbook_version: 1`, `id: format!("wf-{}", id.simple())` を小文字化、`required_*` は空）を返す。
- **完了条件**: `cargo check --workspace --tests` 成功、`cargo test -p api-server playbook` PASS、clippy 成功。（ルート未配線のため統合テストは P-4 で実施）
- **リスク**: 中。`AppState` のフィールド名・`creator_id` の取得方法は既存ハンドラの写経で解決すること（推測で書かない）。`install_manifest` のロールバック漏れに注意。失敗時はコミットを `git revert`。
- **依存**: P-2

---

### P-4: router 配線・OpenAPI 登録・統合テスト（4点セットの完成）

- **対象**: `apps/api-server/src/router.rs`（アンカー: workflow ルート群 L478-495 の直後）、`apps/api-server/src/api.rs`（アンカー: `crate::routes::skill::list_skills` を含む `paths(...)`）、`apps/api-server/src/api_integration_tests/playbook.rs`（新規）と `api_integration_tests/mod.rs`
- **目的**: `.agent/skills/api-route-wiring-check.md` の必須4点（ハンドラ・配線・OpenAPI・Mock 同期）を満たし、受け入れ基準の Negative Test を CI に固定する。
- **変更**:
  1. `internal_router` の workflow ルート直後に追加:

```rust
.route("/api/v1/playbooks", get(routes::playbook::list_playbooks))
.route("/api/v1/playbooks/import", post(routes::playbook::import_playbook))
.route("/api/v1/playbooks/:id/install", post(routes::playbook::install_playbook))
.route("/api/v1/workflows/:id/export", get(routes::workflow::export_workflow))
```

  2. 4ハンドラに `#[utoipa::path(...)]`（`routes/skill.rs` L29-37 の形式を踏襲）を付け、`api.rs` の `paths(...)` に4本、`components(schemas(...))` に `PlaybookSummary` / `PlaybookInstallResponse` を登録。
  3. 統合テスト `playbook.rs`（`api_integration_tests/workflow.rs` の `test_workflow_routes_auth` L15 / `test_workflow_crud_roundtrip` L40 を雛形にする）:
     - `test_playbook_routes_auth`: 4ルートすべて未認証で 401。
     - `test_playbook_list_returns_bundled_four`: 認証付き GET で4本返る。
     - `test_playbook_install_roundtrip`: `seo-operations` を install → 200 と `created_workflow_ids` → 各 id で `GET /api/v1/workflows/:id` が 200・`visibility=="private"`。
     - `test_playbook_import_rejects_invalid_manifest`: `id: "../etc"` と `workflows: []` の2パターンで 400（受け入れ基準3）。
     - `test_playbook_import_rejects_missing_deps`: `required_skills: ["no-such-skill"]` で 422、ボディに `missing_skills` 配列、**かつ workflows 一覧が増えていない**こと（受け入れ基準2＝サイレント部分適用禁止の検証）。
     - `test_workflow_export_roundtrip`: workflow を1つ作成 → export → 返ったマニフェストをそのまま `POST /api/v1/playbooks/import` → 200（import/export の対称性）。
     - `api_integration_tests/common.rs` の Mock/AppState 構築がスキル一覧・MCP 設定に触る場合は同一シグネチャで同期する（4点セットの Mock 同期）。
  4. `api_integration_tests/mod.rs` に `pub mod playbook;` を追加。
- **完了条件**: `cargo test -p api-server api_integration_tests::playbook` 全 PASS、`cargo test -p api-server api_integration_tests::workflow` が 0-b と同数 PASS、`cargo check --workspace --tests`・clippy 成功。加えて **Negative 配線確認**: `router.rs` の playbook ルート4行を一時コメントアウトして該当統合テストが 404 系で FAIL することを確認し、**必ず元に戻して**再度 GREEN を確認する（api-route-wiring-check の完了条件）。
- **リスク**: 中。テスト環境の MCP 設定ファイル有無で `required_mcp_servers` 検査の結果が揺れる → 統合テストの依存検査は `missing_skills` 側で行い、MCP 側はユニットテスト（P-3 の共通関数を直接呼ぶ）で担保してよい。失敗時はコミットを `git revert`。
- **依存**: P-3

---

### P-5: SetupWizard に Playbook 選択ステップを追加（management-console）

- **対象**: `apps/management-console/src/components/SetupWizard.tsx`、`src/i18n/ja.json` / `en.json`
- **目的**: 初期化成功直後に公式 Playbook を選択導入できるようにする（スキップ可能）。
- **変更**:
  1. `handleFinalize` 成功パス（アンカー: `window.location.reload()`）を変更: `setAuthToken(data.access_token)` と `onComplete()` はそのまま、`window.location.reload()` を削除して `setStep(6)` に置換。**失敗パス（`setStep(4)`）と Step 0〜5 の JSX・文言は1文字も変えない**。プログレスドット `[0,1,2,3,4]`（L156）も変更しない。
  2. Step 6 の JSX を Step 5（アンカー: `step === 5`）の直後に追加。挙動:
     - マウント時に `authenticatedFetch(\`${API_BASE}/api/v1/playbooks\`)` で一覧取得（`useModelStatus.ts` のパターンを踏襲。`authenticatedFetch` は `src/lib/auth.ts` から import 追加）。
     - 各 Playbook をカード表示（`name` / `description`）。クリックで `POST ${API_BASE}/api/v1/playbooks/${id}/install` → 成功したカードに導入済み表示。
     - 「スキップ」ボタンと「開始」ボタン（どちらも `window.location.reload()` を呼ぶ）。**一覧取得や install が失敗しても reload への導線は常に残す**（セットアップを人質にしない）。
     - 422 応答時は `missing_skills` / `missing_mcp_servers` をエラーメッセージとしてカード下に表示する。
     - スタイルは既存ステップと同じくインライン + `var(--...)` トークン（L131-149 の使用例に倣う）。新規 CSS ファイルは作らない。
  3. i18n キー追加（ja/en 両方。`setup` ネームスペース内）: `setup.playbookTitle`（「業務テンプレートを選ぶ」/ "Choose a Playbook"）、`setup.playbookDesc`、`setup.playbookInstall`、`setup.playbookInstalled`、`setup.playbookSkip`、`setup.playbookStart`、`setup.playbookError`。
- **完了条件**: `cd apps/management-console && npx tsc --noEmit` 成功、`npx jest SetupWizard` は**この時点では既存テストの FAIL を許容**（reload タイミング変更の影響。P-6 で修正するため、FAIL する場合は P-5 と P-6 を**連続で実施し、コミットは分けたまま** P-6 完了時点で両方 GREEN にする）。
- **リスク**: 中。既存 e2e テストへの影響が確実にある（P-6 で対処）。失敗時は `git checkout -- apps/management-console/src/` で戻す。
- **依存**: P-4（API が先に存在すること）

---

### P-6: フロントテストの更新と追加

- **対象**: `apps/management-console/src/components/SetupWizard.test.tsx`
- **目的**: 既存6テストを GREEN に戻し、Playbook ステップの挙動をテストで固定する。
- **変更**:
  1. 既存 `should complete the full wizard flow end-to-end`（L32）: fetch モックを「`setup/init` は成功レスポンス、`/api/v1/playbooks` は `[]`」の分岐モックに変更し、最終確認を「reload 呼び出し」から「Playbook ステップ表示 → スキップ押下 → reload 呼び出し」に更新する。`window.location.reload` のモック方法は既存テストの慣例（存在しなければ `Object.defineProperty(window, 'location', ...)`）で行う。`lib/auth` のモック（L21-23）に `authenticatedFetch` を追加。
  2. 新規テスト3本:
     - `should list playbooks after successful setup and install selected one`: 一覧4件モック → カード表示 → 1件クリック → `POST .../install` が呼ばれ導入済み表示。
     - `should allow skipping playbook selection`: スキップ押下で reload。
     - `should surface missing dependencies from 422 response`: install モックが 422 + `{"missing_skills":["x"]}` → エラー文言表示、reload 導線が残る。
  3. **既存テストの assert を弱めない**（検証内容の削除禁止。reload タイミングの変更に伴う「移動」のみ許可）。
- **完了条件**: `npx jest SetupWizard` 全 PASS（既存6本＋新規3本）、`npx jest workflowConverter WorkflowBuilder` が 0-b と同数 PASS、`npx tsc --noEmit` 成功。
- **リスク**: 低〜中（jsdom での fetch 分岐モック）。失敗時は P-5/P-6 のコミットを `git revert`（P-6 → P-5 の順）。
- **依存**: P-5

---

### P-7: ドキュメント同期

- **対象**: `CHANGELOG.md`（[Unreleased] > Added）、`.context/RIPPLE_MAP.md`、`OPEN.md`、`docs/roadmaps/value_10x_roadmap.md`、`memory/2026-07-03.md`（当日中の場合のみ）
- **変更**:
  1. CHANGELOG: 「F-1 Agent Playbooks: 公式業務テンプレート4本の同梱と `GET /api/v1/playbooks`・install/import/export API、SetupWizard の Playbook 選択ステップを追加」。
  2. RIPPLE_MAP: 新規ファイル（`workflow/playbook.rs`, `routes/playbook.rs`, `assets/playbooks/*`, `api_integration_tests/playbook.rs`）と影響（workflows テーブルへの書込増、SetupWizard 完了フロー変更）を追記。
  3. OPEN.md: F-1 完了の記録と、新規発見課題「フロント `POST /workflows/validate` とバック `/:id/validate` のパス不整合」を新 OP 番号で登録。
  4. value_10x_roadmap.md: F-1 に「実装済み（YYYY-MM-DD、本計画書参照）」の注記。
  5. `bash scripts/docs-sync-check.sh` が存在すれば実行して PASS を確認。
- **完了条件**: 上記5点の反映、`cargo test -p api-server api_integration_tests::playbook` が引き続き PASS（ドキュメントのみの変更で壊れないことの形式確認）。
- **リスク**: なし。
- **依存**: P-6

---

### 実行順トレース検証（作成者による事前確認済み）

P-1（純粋型・非接触）→ P-2（P-1 の型でアセットを検証）→ P-3（P-2 のレジストリを使うハンドラ。未配線なので既存挙動に影響ゼロ）→ P-4（配線して初めて外部から見える。統合テストで契約を固定）→ P-5（P-4 の API を呼ぶ UI）→ P-6（P-5 の変更をテストで固定）→ P-7（記録）。
前の項目を覆す変更は存在しない。P-5 と P-6 の間のみ一時的にフロントテストが RED になり得るが、コミット単位の独立性は保たれる（P-5 単体 revert 時は P-6 も同時に revert する）。

---

## 4. やらないことリスト（実行者への禁止事項）

1. **Cron スケジューラを実装しない**: `TriggerType::Cron` は schema 定義のみで実行系が存在しない。Playbook のワークフローはすべて `Manual` トリガーとする。スケジューラ新設は別計画。
2. **`is_template` / `visibility` の marketplace 値を使わない**: F-3（Skill Marketplace）のスコープ。import されたワークフローは `visibility="private"` 固定。`WorkflowStore` に新メソッド・新カラムを追加しない。
3. **既存の validate パス不整合（1-3 の 2）を修正しない**: OPEN.md への記録のみ（P-7）。
4. **既存 workflow ルートの OpenAPI 未登録を解消しない**: 新設 playbook ルートのみ登録する。
5. **`SubWorkflow` を含む Playbook をサポートしない**: UUID remap は v2 課題。v1 は 400 で拒否。
6. **URL からの Playbook リモート import・署名検証を実装しない**: 署名基盤は F-3 の成果物。v1 は同梱アセットとローカル JSON のみ。
7. **SetupWizard の既存ステップ 0〜5 の文言・遷移・プログレスドットを変更しない**（`window.location.reload()` の移設のみ許可）。
8. **依存クレートを追加・更新しない**: serde / utoipa / uuid はすべて既存依存。`Cargo.toml` の変更は不要のはず。必要になったら計画の前提が崩れているので中断・報告。
9. **Safety-Critical Zone に触れない**: `auth.rs` / `commerce.rs` / `.github/workflows/` / `src-tauri/` / `tauri.conf.json` は非接触。playbook ルートの認証は既存 `auth_middleware` に**乗るだけ**で、認証コード自体を変更しない。
10. **テストの assert を緩めない・削除しない・`#[ignore]`/`skip` にしない**。落ちたらプロダクションコード側を疑い、解決できなければ中断・報告。
11. **`git push` をしない**: 全項目完了後もプッシュはユーザーの判断に委ねる。
12. **`commercial/` 配下に触らない**。

---

## 5. 実行者への指示文（このままコピペして渡す）

```
docs/roadmaps/f1_agent_playbooks_implementation_plan.md の実装計画を実行してください。

ルール:
1. 「項目0」から順に P-1 → P-2 → P-3 → P-4 → P-5 → P-6 → P-7 の実行順で1項目ずつ実施する。順序の入れ替え禁止。
2. 1項目完了するごとに「完了条件」のコマンドをすべて実行し、期待結果を確認してから、その項目のみを1コミットとして記録する（メッセージは「feat(playbooks): <項目ID> <要約>」形式）。
3. 完了条件を満たせない場合は、それ以上進まず、項目ID・実行コマンド・エラー出力全文を報告して中断する。assert の緩和・#[ignore]・スコープ外ファイルの修正による回避は禁止。
4. 「やらないことリスト」を先に読み、全項目で遵守する。
5. 行番号は変動するため、各項目記載の「アンカー文字列」で対象を再特定してから編集する。AppState のフィールド名や creator_id の取得方法など計画書が「既存ハンドラを踏襲」と指定した箇所は、必ず該当ハンドラを読んでから書く（推測で書かない）。
6. すべて完了したら、各項目のコミットハッシュ一覧、最終の cargo test の test result 行、npx jest のサマリを報告する。git push はしない。
```

---

## 付録: 全体ロールバック手順

- 全体を破棄する場合: `git checkout main && git branch -D feature/f1-agent-playbooks`
- 特定項目のみ戻す場合: 各項目は独立コミットのため `git revert <該当コミット>`。ただし依存の逆順で戻すこと（P-6 → P-5 → P-4 → P-3 → P-2 → P-1）。P-5 を戻す場合は P-6 も必ず同時に戻す。
