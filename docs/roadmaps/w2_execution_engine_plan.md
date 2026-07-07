# W2 ワークフロー実行エンジン本実装計画（OP-073）+ 残存タスク完遂ロードマップ

**作成日: 2026-07-08（v2: /perfect-plan 検証済み・監査反映） / 対象: OP-073 (W2), U1-8, U2-4, F-2, ローカル整理**
**前提: W1（UI/CRUD/SSE job_ids マッチ）は 2026-07-07 コミット済み**

本計画は実コードの行番号アンカーに基づく。実装者は各タスクの「変更対象」「実装内容」「検証」を上から順に実行すればよい。
🤖 マークの付いたタスクは機械的な単純作業であり、低コストのサブエージェント（composer 系）に委譲してトークン消費を抑えること（§6 参照）。

**再利用必須の既存資産（車輪の再発明禁止）**:

| 用途 | 既存実装 | アンカー |
|---|---|---|
| HTTP クライアント（redirect 無効・共有） | `aiome_core::http::get_http_client()` | `libs/core/src/http.rs:14-22` |
| SSRF 検証（reserved domain + private IP + DNS 解決） | `WorkflowValidator` 内ロジック | `libs/infrastructure/src/workflow/validator.rs:26-42, 177-224` |
| テンプレート展開（`{{input}}` 等） | `minijinja`（依存導入済み、使用例: `prompt_registry.rs`） | `libs/infrastructure/Cargo.toml:67` / `prompt_registry.rs:101-164` |
| HTTP 取得 + 2MB 上限 + 30s timeout の参考実装 | `CortexIngester::ingest_url` | `libs/infrastructure/src/cortex_ingester.rs:73-127` |
| LLM 実行 + テスト用 `CapturingProvider` | `GenericLlmConductor` / seo テスト | `llm_conductor.rs:64` / `seo_content.rs:278-319` |
| ジョブ完了フック | `HookManager::trigger_job_completed` | `libs/infrastructure/src/security/hook_manager.rs:89-123` |
| 既存 WF テスト群（期待値更新の対象） | `libs/infrastructure/src/workflow/mod.rs` | chain `:586-654` / loop `:657-704` / parallel `:706-770` / sub 上限 `:772-810` / timer+wasm `:1160-1209` / parallel N `:1452-1500` |

**注意**: `SecurityPolicy::validate_url`（`libs/shared/src/security.rs:168-181`）は localhost の 8188/11434 ポートを許可するため **wf_http の SSRF ガードには使用しない**こと。

---

## 0. 実コード調査で発見した重大バグ（W2 の前提を壊すため最優先で修正）

### B1 🔴 `job_ids` 不一致 — W1 の SSE マッチングは実際には動作しない

- **事実**: `execute_workflow`（`apps/api-server/src/routes/workflow.rs:250-266`）はトランスパイラが生成した `job.id` を `job_ids` としてフロントに返すが、`JobQueue::enqueue` は**内部で新規 UUID を採番する**（`libs/infrastructure/src/job_queue/core_ops.rs:88` `let id = Uuid::new_v4().to_string();`）。
- **帰結**: フロントの SSE `task_completed` マッチング（`WorkflowBuilder.tsx` の job_ids 追跡）は**永遠にマッチしない**。実行ステータスは RUNNING のまま。
- **さらに**: `karma_directives` 内の `parent_job_id` / `parent_job_ids`（`transpiler.rs:130-135, 182-188`）もトランスパイラ採番 ID を指すため、DB 上のジョブとは**紐付かないダングリング参照**になっている。

### B2 🔴 トランスパイラが node_type のパラメータを topic に含めない

- **事実**: `Timer` / `WasmCode` は `json!({...})` で node_type フィールドを topic に書き込む（`transpiler.rs:259, 284`）が、`LlmPrompt` / `HttpRequest` / `McpToolCall` / `Transform` / `Condition` / `HumanApproval` は `topic: node.config.to_string()`（`transpiler.rs:323`）のみ。
- **帰結**: `url_template` / `method` / `server_name` / `tool_name` / `expression` / `prompt_message` は node_type enum のフィールドであり `config` には入っていないため、**Conductor は実行に必要な情報を一切受け取れない**。

### B3 🔴 `Condition` ノードのジョブは永久 Pending

- **事実**: `Condition` はトランスパイラの `_ => "wf_generic"`（`transpiler.rs:311`）に落ちるが、`WorkflowConductor::capable_categories()`（`workflow_conductor.rs:24-36`）に `wf_generic` は**含まれない**。dispatch loop の dequeue はカテゴリでフィルタする（`dispatch_loop.rs:34`）ため、誰にも拾われない。

### B4 🟠 ジョブの依存順序が実行時に無視される

- **事実**: `execute_workflow` は全ジョブを一括 enqueue し、`run_dispatch_loop` は見つけ次第並列 spawn する（`dispatch_loop.rs:131`）。`parent_job_id` は InvariantDag 検証（`dispatch_loop.rs:137`）にのみ使われ、**実行順序のゲートには使われない**。
- **帰結**: 「LLM → HTTP」というワークフローでも 2 ジョブが同時に走る。データの受け渡し（親の出力→子の入力）も存在しない。

### B5 🟠 `workflow_executions` が Running のまま確定しない

- **事実**: `WorkflowStore::update_execution_status`（`libs/infrastructure/src/workflow/store.rs:245-259`）は実装済みだが、**本番コードから一度も呼ばれていない**（テストのみ）。

---

## 1. アーキテクチャ決定（実装前に読むこと）

### 1-1. WorkflowConductor の依存注入

現状 `WorkflowConductor::new()` は無依存（`workflow_conductor.rs:16-20`）。以下の deps 構造体に変更する:

```rust
pub struct WorkflowConductorDeps {
    pub llm: Arc<dyn LlmProvider>,                       // wf_llm 用（bg_provider）
    pub job_queue: Arc<dyn JobQueue>,                    // 依存ゲート・親出力取得用
    pub wasm_manager: Option<Arc<WasmSkillManager>>,     // wf_wasm 用
    pub mcp_invoker: Option<Arc<dyn McpToolInvoker>>,    // wf_mcp 用（後述の新トレイト）
    pub http_client: reqwest::Client,                    // wf_http 用
}
```

- `LlmProvider` の使用例: `GenericLlmConductor`（`libs/infrastructure/src/task_orchestrator/llm_conductor.rs:16-27, 64` `self.llm.complete(&prompt, None).await?`）。
- 登録箇所: `apps/api-server/src/bootstrap/core_services.rs:774-777`。`bg_provider`（`core_services.rs:35`、`bootstrap/llm_providers.rs:54-55`）と `job_queue` は同関数内で入手可能。

### 1-2. MCP 実行の依存方向問題 → 新トレイト `McpToolInvoker`

MCP の実行実体は api-server 側にしかない（`apps/api-server/src/mcp/client.rs:283-291` `McpEndpoint::call_tool`、`McpProcessManager` は `client.rs:324-327`）。infrastructure → api-server の依存は不可。

**解決**: ツール発見で使われている `McpToolSource` トレイト（`libs/aiome-core-contracts/src/traits.rs:983-986`）と同じパターンで、contracts に実行用トレイトを追加する:

```rust
// libs/aiome-core-contracts/src/traits.rs（McpToolSource の直下に追加）
#[async_trait::async_trait]
pub trait McpToolInvoker: Send + Sync {
    async fn invoke_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String, AiomeError>;
}
```

api-server 側で `McpProcessManager` をラップして実装（`active_client_ids()` → `get_client()` → `call_tool()`。走査ロジックは `apps/api-server/src/tool_call_router.rs:348-367` を参考に、`server_name` 一致でクライアントを特定する点だけ変える）。

### 1-3. HumanApproval の一時停止セマンティクス

`conduct()` の戻り値は `Ok`（→ `complete_job`、`dispatch_loop.rs:239`）か `Err`（→ リトライ/失敗、`dispatch_loop.rs:451-541`）の2値しかなく、「承認待ちで停止」を表現できない。トレイトシグネチャ変更は全 Conductor + テストモックに波及するため**行わない**。

**解決（最小波及）**: `AiomeError` に専用バリアントを追加し、dispatch loop の `Err` 分岐の**先頭**でインターセプトする:

1. `libs/aiome-contracts/src/error.rs` の `pub enum AiomeError`（L25）に追加:
   ```rust
   #[error("人間の承認待ち: {reason}")]
   AwaitingApproval { reason: String },
   ```
2. `dispatch_loop.rs:451`（`Err(e) =>` 直後）に追加:
   ```rust
   if let AiomeError::AwaitingApproval { reason } = &e {
       let _ = job_queue_clone.update_job_status(&job_id, JobStatus::AwaitingInput).await;
       let _ = progress_tx.send(TaskEvent::AwaitingInput {
           job_id: job_id.clone(), reason: reason.clone(),
       }).await;
       // active_jobs クリーンアップへ直行（リトライ・Watchtower 診断はスキップ）
       let mut active = active_jobs_clone.write().await;
       active.remove(&job_id);
       return;
   }
   ```
3. **再開フロー（既存機構を再利用、新規エンドポイント不要）**: `POST /api/v1/jobs/{id}/review`（`apps/api-server/src/routes/jobs.rs:176, 192-197`）の approve は `store_execution_log("IMMUNE_BYPASS_APPROVED")` → `requeue_job`（`jobs.rs:225-240`）→ Pending 再ディスパッチ。`wf_approval` の `conduct()` は冒頭で `job.execution_log` に `IMMUNE_BYPASS_APPROVED` が含まれるかを確認し、含まれれば承認済みとして `Ok` を返す（`goal_processor.rs:132` と同じマーカー検出パターン）。

### 1-4. 依存ゲートとデータフロー（B4 対応）

各ワークフロージョブは spawn された独立タスクで走る（dispatch loop 本体はブロックしない）ため、**Conductor 内で親ジョブの完了をポーリング待機する方式**を採る。dequeue の SQL 変更（影響半径大）は行わない。

```
conduct() 冒頭:
  1. karma_directives から parent_job_id（Parallel は parent_job_ids + wait_mode）を取得
  2. loop { fetch_job(parent) } で親のステータスを 500ms 間隔で確認
     - Completed → output_artifacts を「入力」として取得し続行
     - Failed/Cancelled → Err(Infrastructure { "親ジョブ {id} が失敗したため中断" })
     - タイムアウト（既定 600s、env AIOME_WF_DEP_TIMEOUT_SECS）→ Err
  3. 親出力が "__skipped__"（Condition 偽枝マーカー）なら自分も
     Ok(("__skipped__".into(), None)) を返して伝播
```

### 1-5. Condition の分岐セマンティクス

- Condition ジョブは式を評価して `"true"` / `"false"` を出力（`complete_job` の `output_artifacts` に載る）。
- トランスパイラは、source が Condition ノードである edge の `source_handle`（`"true"` / `"false"`、`schema.rs:120`。フロントの ConditionNode ハンドルは W1 実装済み）を子ジョブの `karma_directives.branch` に記録する。
- 子ジョブの conduct は「親が Condition かつ 親出力 ≠ 自分の branch」なら `Ok(("__skipped__", None))` で完了し、以降 1-4 の skip 伝播に乗る。

### 1-6. SubWorkflow の実解決（モック除去）

`transpiler.rs:205-249` のハードコード疑似定義を削除。トランスパイラは同期関数のまま維持し、**呼び出し側で事前解決**する:

```rust
// transpiler.rs — シグネチャ追加（既存 transpile は空マップ委譲で後方互換）
pub fn transpile_with_resolver(
    definition: &WorkflowDefinition,
    execution_id: Uuid,
    resolved: &HashMap<Uuid, WorkflowDefinition>,  // 事前解決済みサブワークフロー
) -> Result<Vec<Job>, TranspilerError>
```

- SubWorkflow ノード処理: `resolved.get(workflow_id)` → あれば `transpile_with_depth(sub_def, ..., depth+1)`（深さ上限 5 は既存 L37-39 を維持）、なければ `ValidationError("SubWorkflow {id} が未解決")`。
- `execute_workflow`（route）側: 実行前に BFS でネストした SubWorkflow ID を収集し、`WorkflowStore::get_workflow`（`store.rs:109`）+ `get_version`（`store.rs:193`）で定義を取得して map を作る。visited set で循環を検出し 400 を返す。
- 既存の再帰上限テストは「自己参照する定義を resolver map に入れる」形に書き換える。

---

## 2. Phase 構成と実装手順

依存関係: **W2-0 → W2-1 → W2-2 → (W2-3 と W2-4 は並行可) → W2-5 → W2-6 → W2-7 → W2-8**

### Phase W2-0: 契約バグ修正（B1/B2/B3）— 0.5日

| # | 変更対象 | 実装内容 |
|---|---|---|
| 0-1 | `apps/api-server/src/routes/workflow.rs:250-266` | enqueue が返す実 ID を採用し、`HashMap<transpiler_id, actual_id>` で `karma_directives` の `parent_job_id` / `parent_job_ids` を enqueue 前に書き換える（トランスパイラはトポロジカル順で親が先に出るため、マップは常に先行して埋まる）。`job_ids` レスポンスには実 ID を入れる |
| 0-2 | `libs/infrastructure/src/workflow/transpiler.rs:300-334` | `LlmPrompt`/`HttpRequest`/`McpToolCall`/`Transform`/`Condition`/`HumanApproval` の topic を「node_type フィールド + node.config のマージ JSON」にする。例: HttpRequest → `json!({"method": m, "url_template": u, "config": node.config})` |
| 0-3 | `transpiler.rs:311` + `workflow_conductor.rs:24-36` | Condition のカテゴリを新設 `"wf_condition"` にし、`capable_categories` に `"wf_condition"` と保険として `"wf_generic"` を追加 |
| 0-4 | `transpiler.rs:314-318`（`karma_directives` 構築） | 親エッジの `source_handle`（`schema.rs:120`）を `"branch"` キーとして `karma_directives` に記録する（§1-5 の Condition 分岐で使用。現状は `workflow_execution_id`/`node_id`/`parent_job_id` のみで branch 情報が欠落） |

**検証**: `cargo test -p infrastructure workflow`（既存テストは**すべて `libs/infrastructure/src/workflow/mod.rs` に集約**されている。topic 期待値の更新対象: chain `:586-654` / loop `:657-704` / parallel `:706-770` / timer+wasm `:1160-1209`）+ `cargo test -p api-server test_workflow_execute_api`（現状は `execution_id` のみ assert（`api_integration_tests/workflow.rs:269-273`）。**`job_ids` が jobs テーブルの実 ID と一致するアサーションを新規追加**）。🤖 既存テスト期待値の機械的更新はサブエージェント委譲可。

### Phase W2-1: WorkflowConductor DI 化 — 0.5日

| # | 変更対象 | 実装内容 |
|---|---|---|
| 1-1 | `libs/aiome-core-contracts/src/traits.rs`（L983 付近） | `McpToolInvoker` トレイト追加（§1-2） |
| 1-2 | `libs/infrastructure/src/task_orchestrator/workflow_conductor.rs` | `WorkflowConductorDeps` 導入（§1-1）。`new(deps)` に変更 |
| 1-3 | `apps/api-server/src/mcp/client.rs` または新規 `apps/api-server/src/mcp/invoker.rs` | `McpProcessManagerInvoker`（`McpProcessManager` ラッパー）で `McpToolInvoker` 実装 |
| 1-4 | `apps/api-server/src/bootstrap/core_services.rs:774-777` | `bg_provider.clone()` / `job_queue.clone()` / `mcp_manager` ラッパー / `wasm_skill_manager` を注入して登録 |

**検証**: `cargo check --workspace --tests`。`WorkflowConductor::new` の呼び出し箇所は `core_services.rs:776` と `workflow/mod.rs` 内テスト（`:814-826, 1249-1296`）のみ（**`workflow/mod.rs:7-11` の re-export は playbook/schema/store/transpiler/validator のみで conductor は含まれない**。誤解に注意）。

### Phase W2-2: 依存ゲート + データフロー + skip 伝播 — 1日

- `conduct()` 冒頭に §1-4 の待機ロジックを実装。新規ヘルパー `async fn await_parents(&self, job: &Job) -> Result<ParentOutcome, AiomeError>` として `workflow_conductor.rs` 内に分離（`ParentOutcome { inputs: Vec<String>, skipped: bool }`）。
- Parallel（`wf_parallel`）: `karma_directives.wait_mode`（`transpiler.rs:182-188` で書き込み済み）に従い All / Any / N(usize) のゲートを実装。
- **注意**: MockJQ の `fetch_job` は現状**常に `Ok(None)` を返すスタブ**（`libs/infrastructure/src/testing/mock_jq.rs:149-150`）。依存待機テストのために「事前登録したジョブを返す `HashMap` バッキング」への拡張が必要（🤖 委譲可）。既存 conductor テスト（`workflow/mod.rs:1249-1296`）は親なしジョブなので影響最小だが、`parent_job_id` が付くケースでは Completed を返すよう調整。

**検証**: 新規ユニットテスト「親 Pending → 子待機 → 親 Completed 後に実行」「親 Failed → 子 Err」「Any モードで1つ完了なら通過」。

### Phase W2-3: ノード実行の本実装 — 2日

すべて `workflow_conductor.rs` の `match job.category` に実装。**共通**: 親出力の `{{input}}` / `variables` 展開は**自作の文字列置換を書かず、導入済みの `minijinja` を使う**（依存: `libs/infrastructure/Cargo.toml:67`、使用例: `prompt_registry.rs:101-164`）。HTTP クライアントは**新規構築せず** `aiome_core::http::get_http_client()`（`libs/core/src/http.rs:14-22`、redirect 無効済み）を使う。

| カテゴリ | 実装内容 | Safety |
|---|---|---|
| `wf_llm` | topic JSON から `prompt`（config 由来）+ `model`/`temperature` を取得。`{{input}}` 展開後 `deps.llm.complete(&prompt, None).await?`。出力 = `response.content`（`llm_conductor.rs:64` と同型） | prompt 空は `Validation` エラー |
| `wf_http` | `method` + `url_template`（minijinja 展開）→ **送信直前に SSRF 再検証**: `validator.rs` の `is_private_ip`（`:26-42`）**だけでは不十分**。reserved domain チェック + DNS 解決後 IP チェックを含む `:177-224` の一連のロジックを `pub(crate) async fn assert_url_safe(url: &str)` として `validator.rs` に関数抽出し、バリデーション時と実行時の両方から呼ぶ（重複実装禁止）。クライアントは `get_http_client()` + タイムアウト 30s + レスポンス上限 2MB（実装参考: `cortex_ingester.rs:87-127` の `MAX_BODY_BYTES` パターン） | バリデーション時だけでなく**実行時にも** SSRF ガード（TOCTOU 対策）。`SecurityPolicy::validate_url` は localhost 8188/11434 を許可するため使用禁止。POST 等の外部送信を含むため、実装後に人間レビューを依頼すること（AGENTS.md Safety-Critical #5） |
| `wf_mcp` | `deps.mcp_invoker` が None なら `Validation` エラー。`invoke_tool(server_name, tool_name, config_args)` | 既存 MCP レイヤの権限に委譲 |
| `wf_condition` | `mode: Expression` → 式を親出力 JSON に対する単純比較（`$.path == "value"` / `contains` / 真偽値キー参照の3形式のみサポート。パーサは 100 行以内の素朴実装とし、eval 系は使わない）。`mode: LlmJudge` → `deps.llm.complete` に「true/false のみ返せ」プロンプト。出力は `"true"` / `"false"` | 式は文字列比較のみ、コード実行なし |
| `wf_transform` | `expression` が `$.` 始まりなら親出力 JSON からのパス抽出、それ以外は `{{input}}` テンプレート展開の2形式のみ | 同上 |
| `wf_wasm` | `language == "javascript" or "typescript"` → `deps.wasm_manager.run_code_mode_js(code, &manifest)`（`libs/infrastructure/src/skills/mod.rs:588-594`。実体は 5 命令ミニインタープリタ `code_mode.rs:78-82`）。`manifest` は全 false + `allow_network: false` の最小権限。`rust` は「ランタイムコンパイル未対応」の `Validation` エラー（validator.rs:301-314 の許可言語と整合させる） | PermissionManifest 最小権限固定。ユーザー入力コードに shell/fs 権限を渡さない |
| `wf_loop` | MVP: 各イテレーションジョブは logging 付き pass-through（現挙動維持）。`loop_index` を出力 JSON に含める | — |
| `wf_parallel` | W2-2 のゲートのみ（合流後は親出力の配列を JSON で出力） | — |
| `wf_timer` | 実装済み（現状維持） | — |

**検証**: カテゴリ別ユニットテスト（LLM は `CapturingProvider`＝`seo_content.rs` テスト参照、HTTP は `wiremock` or ローカル `axum` テストサーバ、SSRF は `http://127.0.0.1` / `http://169.254.169.254` が**拒否される Negative Test 必須**）。

### Phase W2-4: HumanApproval（wf_approval）— 0.5日

§1-3 のとおり。実装順: ① `AiomeError::AwaitingApproval` 追加 → ② `dispatch_loop.rs:451` インターセプト → ③ `wf_approval` conduct（マーカー検出 → 未承認なら `Err(AwaitingApproval)`、承認済みなら `Ok((prompt_message, None))`。`timeout_seconds` は `job.created_at` からの経過で判定し、超過なら `Err(Infrastructure)`）。

**検証**: 統合テスト（`apps/api-server/src/api_integration_tests/workflow.rs` に追加）: execute → job が AwaitingInput になる → `GET /api/v1/jobs/awaiting-input`（`jobs.rs:288`）に載る → `POST /jobs/{id}/review` approve → 再ディスパッチで完了。**Negative**: reject → `fail_job` される（`jobs.rs:243-267`）。

### Phase W2-5: SubWorkflow 実解決 — 0.5日

§1-6 のとおり。変更: `transpiler.rs`（モック削除 + resolver 化）、`routes/workflow.rs::execute_workflow`（事前解決 BFS）、既存トランスパイラテスト書き換え。

**検証**: `cargo test -p infrastructure transpiler`（再帰上限・未解決エラー・正常展開の3系統）+ 統合テスト「親 WF から子 WF のジョブが展開される」。

### Phase W2-6: 実行ステータス確定（B5）— 0.5日

- 新規 `apps/api-server/src/workflow_execution_tracker.rs`: `execute_workflow` が `(execution_id, Vec<job_id>)` を登録する in-memory トラッカー（`Arc<Mutex<HashMap<...>>>` を AppState に追加）。`core_services.rs` で `event_sender.subscribe()`（`core_services.rs:50` のチャネル）を監視する tokio タスクを spawn し、`CoreEvent::TaskCompleted` / `TaskFailed`（マッピング実体は `dispatcher.rs:59-172`、Completed/Failed は `:76-90`）で該当 execution の残ジョブを減算。
- **🔴 ステータス文字列は DB CHECK 制約に厳密一致させること**: `libs/infrastructure/migrations/sqlite/20260614000000_workflows.sql:40-41` の制約は `'Running' | 'Completed' | 'Failed' | 'Cancelled'`（**大文字始まり**）。全完了 → `update_execution_status(execution_id, "Completed", ...)`、1つでも失敗 → `"Failed"`。小文字を渡すと CHECK 違反で UPDATE が失敗する（既存テスト `workflow/mod.rs:917-931` も `"Completed"` を使用）。
- **既知の制約（明記して許容）**: サーバ再起動でトラッカーは消える。復旧は将来課題として OPEN.md に残す。
- フロント: `useWorkflowApi` に `listExecutions`（`GET /api/v1/workflows/:id/executions`、route 実装済み `workflow.rs:460-483`）を追加。レスポンスは `ExecutionRecord` 配列（フィールド名: `id`, `workflow_id`, `version`, `status`, `input_variables`, `output_result`, `root_job_id`, `started_at`, `completed_at` — snake_case のまま。`store.rs:30-40`）。SSE 断絶時のフォールバックポーリング（10s 間隔、実行中のみ）を `WorkflowBuilder.tsx` に実装。🤖 `listExecutions` の型定義 + フェッチ関数 + Jest テスト追加は `useWorkflowApi.ts:163-198` の既存 `listWorkflows` の複製で済むためサブエージェント委譲可。

### Phase W2-7: フロント補完 — 0.5日

| # | 変更対象 | 実装内容 |
|---|---|---|
| 7-1 | `WorkflowBuilder.tsx:577-599`（LlmPrompt フォーム） | **prompt 入力欄が存在しない**（現状 model/temperature のみ。`workflowConverter.ts:11, 64` にも `config.prompt` は未定義）。textarea を追加し `node.config.prompt` に保存する config 更新ハンドラを新設（`updateNodeTypeDetails` は node_type フィールド用のため使わない。W2-0-2 のマージ topic で Conductor に届く） |
| 7-2 | `WorkflowBuilder.tsx`（HumanApproval） | 現在 `JSON_CONFIG_TYPES`（`WorkflowBuilder.tsx:44-51`）に含まれ JSON エディタ扱い。`prompt_message` / `timeout_seconds` の専用フォームに昇格し、Set から除去 |
| 7-3 🤖 | `i18n/en.json` / `ja.json` | `workflowBuilder.config.prompt`, `promptMessage`, `timeoutSeconds` 等の追加キー（両言語同期、`i18n.test.ts` に検証追加）。機械的作業のためサブエージェント委譲 |

~~7-4（task_failed エラー表示）は**実装済みのため削除**（`WorkflowBuilder.tsx:204-206` で `eventData.error` を表示済み）~~

**検証**: `npm test`（Jest）+ `npm run build` + `python3 scripts/test_ui_hex_violations.py`（新 CSS があれば）。

### Phase W2-8: 総合検証 — 0.5日

1. **Positive**: Start → LlmPrompt → Transform → HttpRequest（httpbin 相当のローカルモック）を UI から実行し、COMPLETED 遷移と `workflow_executions.status = completed` を確認。
2. **Negative（Verification Protocol Step 2、省略禁止）**:
   - HttpRequest に `http://127.0.0.1:8080` を設定 → バリデーション/実行の両方で拒否されること
   - WasmCode に `language: rust` → Validation エラー
   - Condition の親出力を壊し、偽枝ジョブが `__skipped__` になること
   - HumanApproval を reject → 実行が failed になること
3. **Revert & Report**: 注入した障害設定を戻し、正常系が再度 PASS することを確認して3段階を報告。
4. `cargo check --workspace --tests && cargo test --workspace` / `cargo clippy` / `cargo fmt --check`。
5. E2E: 既存 `e2e/workflow-builder.spec.ts` に「実行ボタン → ステータスパネル表示」のスモークを1本追加（バックエンドはモック fetch）。

---

## 3. W2 以外の残存タスク

### U1-8: 決済後ランディング `/checkout/success`（0.5日）

- LP（`commercial/` 配下）に success ページを追加し、Stripe Payment Link の `success_url` から着地。内容: 「決済完了 → アプリでのアンロック手順（ライセンスキー/メール確認）」。`docs/roadmaps/ui_overhaul_plan.md` の U1-8 定義に従う。
- **注意**: 決済→Pro 自動有効化のコード（OP-057-R）は Safety-Critical 凍結中。**このタスクは表示ページのみ**でスコープを固定し、Webhook や有効化ロジックには触れない。

### U2-4: `LockedOverlay` variant props 統合（0.25日）

- `apps/management-console/src/components/ui/LockedOverlay.tsx` の `variant`（W1 で追加済み）を、旧スタイルの Pro バッジを直書きしている残存箇所へ適用統一。`rg "locked-badge|Pro" src/components` で対象を洗い出してから着手。

### F-2: フルレスポンシブ（1日、独立実行可）

- `App.css` のブレークポイント整理（既存: サイドバー collapse のみ）。優先順: ①サイドバー drawer 化（<768px）②ダッシュボード grid 1カラム化 ③WorkflowBuilder は W1 で部分対応済みのため確認のみ。hex ゲートを通すこと。

### ローカル整理（0.1日、ユーザー承認後）

- `git stash list` の一時 stash（playwright-report 等）を `git stash drop`（**破壊的 — 実行前にユーザー確認必須**）。
- 未追跡ファイル `biz_value_report.html` / `rewrite_main.py` の要否をユーザーに確認。

### Human 専用（エージェント対象外）

- **OP-002**: BiomeBackground 目視検証 / **OP-070 残**: 本番 env 反映・R3-4 実走・R4 ローンチ資材。

---

## 4. /perfect-plan ゲート検証（v2: 実コード監査済み）

### v2 監査で修正した誤り（初版からの差分）

1. **W2-6 ステータス文字列**: 初版の `"completed"`/`"failed"` は DB CHECK 制約違反（正: `"Completed"`/`"Failed"`）→ 修正済み。
2. **W2-1 の re-export 確認指示**: `workflow/mod.rs` は conductor を re-export していない → 記述修正済み。
3. **W2-2 MockJQ アンカー**: `fetch_job` は常に `None` を返すスタブ（`:149-150`）であり、拡張が必要 → 修正済み。
4. **W2-3 SSRF**: `is_private_ip` 単体では不十分（reserved domain + DNS 解決が必要）、`SecurityPolicy::validate_url` は使用禁止 → 関数抽出方式へ修正済み。
5. **W2-7-4**: `task_failed` エラー表示は実装済みだったため削除。
6. **車輪の再発明の排除**: `{{input}}` 自作置換 → minijinja、reqwest 新規構築 → `get_http_client()`、テストモック → `CapturingProvider` 再利用へ変更。
7. **W2-0-4 追加**: Condition 分岐に必須の `source_handle` → `branch` 記録がタスク化されていなかった抜けを補完。

### 重複計画ドキュメントとの境界（車輪の再発明防止）

| ドキュメント | 境界 |
|---|---|
| `ui_overhaul_plan.md` | WorkflowBuilder の見た目/HEX/レスポンシブは向こう。W2-7 は実行機能に必要なフォームのみ |
| `f1_agent_playbooks_implementation_plan.md` | WorkflowStore/Validator/transpiler の**基盤構築**は完了済み資産。W2 はその上の実行のみ |
| `release_master_plan.md` | validate パス不整合・`as any` 解消は解決済み（R1-13/14）。W2 で再着手しない |
| `implementation_plan.md` Phase 3.5 | auth/SSRF の全体修復は別トラック。W2 は wf_http の実行時ガードのみ |

### 初版ゲート検証（維持）

- **Gate 1 構造スキャン**: ✅ 全変更対象を実ファイル・実行番号で確認済み（幻覚なし）。`McpToolInvoker` は新規だが、`McpToolSource`（traits.rs:983）の前例に沿う。二重実装なし（infrastructure に MCP 実行パスが存在しないことを確認済み）。
- **Gate 2 要件カバレッジ**: §3 MCP 統合（wf_mcp は既存 MCP レイヤに委譲）、§4 セキュリティ（SSRF 実行時再検証、Wasm 最小権限、式評価にコード実行なし）に対応。§2 経済（LLM コストは既存の Job-level Budget Check `dispatch_loop.rs:187-223` が自動適用される）。
- **Gate 3 依存関係**: `AiomeError` バリアント追加は非破壊（新規追加のみ）。`WorkflowConductor::new` シグネチャ変更の参照元は `core_services.rs` のみ。`TaskConductor` トレイトは**不変**のため MockJQ / 他 Conductor / `api_integration_tests` への波及なし。トランスパイラ topic 変更は既存トランスパイラテストの期待値更新が必要（W2-0 検証に含む）。
- **Gate 4 悪魔の弁護人**:
  1. *最悪シナリオ*: 依存ゲートのポーリングが親ジョブ失敗の検出漏れで永久待機 → タイムアウト（600s）を必須実装とし、Negative Test で検証。
  2. *誤った前提*: 「トランスパイラはトポロジカル順で親が先」— Loop の複数ジョブや Parallel 合流では例外があり得るため、W2-0-1 の ID リマップで「マップ未登録の親 ID」を検出したらエラーログ + parent なし扱いにフォールバックする防御を入れる。
  3. *やらないメリット*: W2 を放置すれば Builder は「絵を描くだけの UI」であり続け、W1 投資が回収不能。実行エンジンなしのリリースはプロダクト価値の中核を欠く。実行する。
- **Gate 5 実行順序**: W2-0（契約）が全フェーズの前提。W2-3/W2-4 は W2-2 完了後に並行可。W2-6 は B1 修正（W2-0-1）に依存。矛盾なし。既知 tech debt（OP-054 JobQueue API 乖離）とは干渉しない（トレイト変更を回避したため）。

## 5. 実装完了時のドキュメント同期（AGENTS.md 準拠）

- [ ] CHANGELOG.md [Unreleased] に W2 の具体的変更を追記
- [ ] `.context/RIPPLE_MAP.md` に `McpToolInvoker` / `workflow_execution_tracker.rs` / `AwaitingApproval` の影響範囲を追記
- [ ] `docs/architecture/AIOME_NURTURE_SYNERGY.md`（トレイト追加のため該当図を同期）
- [ ] OPEN.md: OP-073 をクローズし、トラッカー再起動復旧を新規 OP として起票
- [ ] `.env.example`: `AIOME_WF_DEP_TIMEOUT_SECS` 追加

## 6. サブエージェント委譲方針（トークン節約）

🤖 マークのタスクは判断を伴わない機械的作業のため、低コストモデルのサブエージェントに委譲する。委譲時は「変更対象ファイル・期待する差分・検証コマンド」を明示したプロンプトを渡すこと。

| 委譲対象 | 内容 | 検証コマンド |
|---|---|---|
| W2-0 検証 | `workflow/mod.rs` 既存テストの topic 期待値の機械的更新 | `cargo test -p infrastructure workflow` |
| W2-2 | MockJQ `fetch_job` の HashMap バッキング拡張 | `cargo test -p infrastructure` |
| W2-6 | `useWorkflowApi.listExecutions` 追加（`listWorkflows` の複製）+ Jest | `npm test -- useWorkflowApi` |
| W2-7-3 | i18n キー追加（en/ja 同期） | `npm test -- i18n` |
| ドキュメント同期 §5 | CHANGELOG / RIPPLE_MAP / .env.example の追記 | 目視 + docs-sync |

**委譲しないもの**: SSRF ガード、HumanApproval フロー、dispatch loop 変更（Safety-Critical 隣接のため本体エージェント + 人間レビュー）。

**総工数目安: W2 = 約6.5日 / 残存 UI = 約1.85日**
