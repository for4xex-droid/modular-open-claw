# 残存タスク Foolproof 実装計画（v6・実装コピペ確定版）

> **作成**: 2026-07-09  
> **改訂**: 2026-07-09 **v6**（v5 再監査。sql_exec 確定形・死んだヘルパー・commerce 既存モック再利用を固定）  
> **ステータス**: 計画フェーズ（実装はユーザー明示許可後）  
> **タスク正本**: [`OPEN.md`](../../OPEN.md)

---

## 0. v5 → v6 で潰した抜け

| # | v5 の抜け | 実コード事実 | v6 の固定 |
|---|-----------|--------------|-----------|
| 1 | OP-024 の `sql_exec!` が「dual かも」と曖昧 | Single 形式あり（`shared/db.rs` L423）。`autonomous_demo` が `sql_exec!(&*pool, &q1)` を使用 | **`infrastructure::sql_exec!(&state.job_queue.pool, "DROP TABLE system_settings")`** で確定 |
| 2 | OP-069 を「4 ファイル置換」と書いた | `agent_engine::setup_test_state` は**定義のみ・呼び出しゼロ**（死コード） | **削除のみ**。置換対象は **3 ファイル** |
| 3 | OP-060 テストを抽象的に書いた | `commerce.rs` に axum mock `/internal/coin-charge` + OXP 検証が**既存**（L893–） | **同パターンを再利用**（wiremock 新規設計は不要。依存はあるが既存資産優先） |
| 4 | DLQ SELECT のマクロ形が曖昧 | `sql_fetch_all!(pool, Type, sqlite:, pg:, …)` | 二系統 SQL + `(String, String)` を固定 |
| 5 | `Component` 経由の pool アクセス | `Component` は `Deref` → `state.job_queue.pool` で `UniversalJobQueue.pool` に到達 | 明記 |

### 禁止リスト（変更なし・再掲）

| 禁止 | 代替 |
|------|------|
| DLQ ワーカーから `enqueue_coin_charge_to_nurture` | `attempt_coin_charge_once`（DLQ 非書込） |
| `requeue_dlq_job` → coin-charge | 別テーブル |
| `create_test_server` を軽量に強制 | `create_test_app_state` |
| ヘルパーを「最小共通」だけにする | **和集合**（processor/agent が追加フィールド要求） |
| `process_html_to_document` pub 化 | `html_to_markdown` 抽出 |
| `wf_transform` で OP-067 | 対象外 |
| OXP 新モジュール | `OxiLeanProofCertificate::generate_header` |
| auth 鍵を `api_server_secret` のまま残す（Nurture 検証と不一致） | `NURTURE_INTERNAL_SECRET` + Bearer（2026-07-10 追完） |
| OP-051 で 10 種再棚卸し | `error_handling.md` §2 |
| OP-060 用に別モックサーバ設計をゼロから | `commerce.rs` の axum mock をコピー |

---

## 1. スコープと順序

| ID | 要約 | Wave |
|----|------|------|
| OP-024 | tool_call_router Fail-Closed | 1 |
| OP-067 | html2md → htmd | 1 |
| OP-069 | create_test_app_state（3 置換 + 1 削除）+ ADR-053 | 1 |
| OP-061 | OXP generate_header 統一 🔐 | 2 |
| OP-060 | outbox_dead_letters 再送 🔐 | 2 |
| OP-054 | JobQueue 補助 API | 3 |
| OP-051 | Error 3 階層（設計承認後） | 3 |

```
OP-024 → OP-067 → OP-069 →（承認）OP-061 → OP-060 → OP-054 →（設計承認）OP-051
```

対象外: OP-068/030–034、Human、OP-062、OP-011、OP-020–027/059

---

## 2. OP-024 — Fail-Closed

### アンカー

- `tool_call_router.rs` L288–300: `if let Ok(Some(val)) = …get_setting_value`
- Err 時: `do_get_setting` → `AiomeError::Infrastructure { reason: "Get setting failed…" }`（`settings.rs` L36–39）
- テスト: `setup_mock_state` → `create_test_server()`（変更不要）
- `#[cfg(test)]` 通過時: `Result("[Mock Executed] …")`（L318–319）

### 本番差分

```rust
match state_rc.job_queue.get_setting_value(&key).await {
    Ok(Some(val)) if val == "true" => { /* 既存 suspend メッセージ */ return; }
    Ok(_) => {}
    Err(e) => {
        tracing::error!(error = %e, setting_key = %key,
            "[Billing] mcp_suspended setting read failed; denying MCP tool (fail-closed)");
        emit_tool_event(&tx_clone, ToolExecutionEvent::Error(
            "[Billing] Unable to verify MCP billing status. Request denied.".into(),
        )).await;
        return;
    }
}
```

### N1 テスト（確定マクロ）

```rust
#[tokio::test]
async fn test_tool_call_router_mcp_suspend_setting_db_error_fail_closed() {
    let router = DefaultToolCallRouter;
    let (mut state, _guard) = setup_mock_state().await;
    state.hook_chain = Component::new(Arc::new(HookChain::new()));
    state.system_agent_id = uuid::Uuid::new_v4();

    // Component: Deref → UniversalJobQueue、pool は pub
    infrastructure::sql_exec!(&state.job_queue.pool, "DROP TABLE system_settings")
        .expect("drop system_settings for negative test");

    let mut rx = router.execute_skill("some_mcp_tool", "{}", &state).await;
    let mut denied = false;
    let mut got_result = false;
    while let Some(evt) = rx.recv().await {
        match evt {
            ToolExecutionEvent::Error(msg)
                if msg.contains("Unable to verify MCP billing status") =>
            {
                denied = true;
            }
            ToolExecutionEvent::Result(_) => got_result = true,
            _ => {}
        }
    }
    assert!(denied);
    assert!(!got_result, "Fail-Open regression if mock executor ran");
}
```

既存 P テストは無変更で PASS すること。

### DoD

- [ ] Fail-Closed match  
- [ ] N1: denied && !got_result  
- [ ] OPEN / CHANGELOG  

---

## 3. OP-067 — html2md → htmd

### アンカー

| 項目 | 値 |
|------|-----|
| 呼び出し | `cortex_ingester.rs` L203 `html2md::parse_html` |
| Cargo | `libs/infrastructure/Cargo.toml` L80 |
| 置換 | `htmd = "0.5"`、`htmd::convert` → `Result` |
| deny | `exceptions` **と** `[[licenses.clarify]]` 両方削除 |

### 本番

```rust
pub(crate) fn html_to_markdown(html: &str) -> Result<String, AiomeError> {
    htmd::convert(html).map_err(|e| AiomeError::Infrastructure {
        reason: format!("HTML to Markdown conversion failed: {e}"),
    })
}
// process_html_to_document:
let raw_md = html_to_markdown(&clean_html)?;
```

空 HTML 拒否は既存 L196–200（trim）が担当。変換関数は触らない。

### テスト（DB 不要）

```rust
#[test]
fn html_to_markdown_strips_heading_tags() {
    let md = crate::cortex_ingester::html_to_markdown("<h1>Hello</h1>").unwrap();
    // 同一モジュール内なら html_to_markdown(...) で可
    assert!(md.contains("Hello"));
    assert!(!md.contains("<h1>"));
}
```

配置: `cortex_ingester.rs` 末尾 `#[cfg(test)]` が最短（`pub(crate)` なら `cortex_ingester_tests.rs` からも可）。

### DoD

- [ ] `rg html2md` 消滅（コード・Cargo・deny）  
- [ ] 単体テスト + `cargo deny check licenses`  
- [ ] OPEN ✅  

---

## 4. OP-069 — test_helpers + ADR-053

### ヘルパー監査結果

| ファイル | 呼び出し | 方針 |
|----------|----------|------|
| `agent_engine.rs` | **0 回**（死コード） | **関数削除のみ**（共通化不要） |
| `system_instructions.rs` | 複数 | → `create_test_app_state` |
| `tool_call_processor.rs` | 複数（MockLlm+hook+arena 必須） | → 同上 |
| `routes/agent.rs` | 複数（cache+prompt_registry 必須） | → 同上 |
| `tool_call_router` / `mcp/server` | `create_test_server` | **触らない** |

### `create_test_app_state`（和集合）

`main.rs` に `#[cfg(test)] mod test_helpers;`

返すフィールド（必須）:

- 共通: registry, wasm_skill_manager, job_queue, config(+resolver)
- processor 用: provider(MockLlm), hook_chain, skill_arena
- agent 用: project_rules_cache, prompt_registry(`MockPromptRegistry` = `NoopPromptRegistry` の alias)

実装スケルトンは v5 と同じ和集合でよい。`async_trait` は api-server 依存済み。

### ADR-053

`docs/decisions/053-federation-unstubbing-acceptance.md` — FederationOps 本実装追認。コード変更なし。

### DoD

- [ ] agent_engine の死ヘルパー削除  
- [ ] 3 ファイルが共通関数を使用  
- [ ] ADR-053  
- [ ] OPEN ✅  

---

## 5. OP-061 — OXP 統一 🔐

| 箇所 | 作業 |
|------|------|
| `oxilean::generate_header` | 正本・変更なし |
| relay / settings | **差分なし**（済・fail-closed） |
| `stripe/mod.rs` `generate_oxp_header` | 本体を `generate_header("aiome-edge-node", oxp, secret)` に置換。`require_oxp_header()` で `None` 時 fail-closed（2026-07-10） |
| `auth.rs` forget | URL は `state.nurture_url`（`NURTURE_API_URL`）。`generate_header("aiome_system", 1000, &nurture_internal_secret)` + `Authorization: Bearer`。secret 欠落はローカル削除前に 500。Nurture 4xx/5xx 時はローカル RTBF 継続（Chesterton） |

**不変**: subject/oxp スコア（auth=`aiome_system`/1000、stripe=`aiome-edge-node`+動的）。課金・Webhook 署名ロジック。

ゲート: 「OP-061 を実装しろ」+ 人間レビュー。

---

## 6. OP-060 — DLQ 自動再送 🔐

### スキーマ / ペイロード

- 表: `outbox_dead_letters(id, event_type, payload, error_reason, created_at)`
- `event_type = "coin_charge_failed"`
- payload JSON: `actor_id`, `amount`, `currency`, `stripe_event_id`, `idempotency_key`

### `attempt_coin_charge_once`（relay.rs）

```rust
pub async fn attempt_coin_charge_once(
    http_client: &reqwest::Client,
    nurture_url: &str,
    secret: &str,
    oxilean_power: u32,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let req_url = format!("{}/internal/coin-charge", nurture_url.trim_end_matches('/'));
    let mut req = http_client
        .post(&req_url)
        .header("Authorization", format!("Bearer {secret}"))
        .timeout(std::time::Duration::from_secs(30))
        .json(payload);
    let Some(cert) = aiome_core_contracts::oxilean::OxiLeanProofCertificate::generate_header(
        "aiome-edge-node", oxilean_power, secret,
    ) else {
        return Err(format!(
            "oxp_header_generation_failed for coin-charge; request denied (fail-closed)"
        ));
    };
    req = req.header("X-OxiLean-Proof-Certificate", cert);
    let res = req.send().await.map_err(|e| format!("network: {e}"))?;
    if res.status().is_success() { Ok(()) } else { Err(format!("http {}", res.status())) }
}
```

- **DLQ INSERT 禁止**  
- `enqueue_coin_charge_to_nurture` の 1 試行をこれに置換。3 失敗後 INSERT は現状維持  
- OXP ヘッダ生成失敗は **fail-closed**（送信しない）

### ワーカー SELECT / DELETE（確定マクロ）

```rust
// pool: &*state.db_pool.get_inner()  （relay と同じ Arc 参照）
let rows: Vec<(String, String)> = infrastructure::sql_fetch_all!(
    pool,
    (String, String),
    sqlite: "SELECT id, payload FROM outbox_dead_letters WHERE event_type = ? ORDER BY created_at ASC LIMIT 10",
    pg: "SELECT id, payload FROM outbox_dead_letters WHERE event_type = $1 ORDER BY created_at ASC LIMIT 10",
    "coin_charge_failed"
)?;

// 成功時:
infrastructure::sql_exec!(
    pool,
    sqlite: "DELETE FROM outbox_dead_letters WHERE id = ?",
    pg: "DELETE FROM outbox_dead_letters WHERE id = $1",
    &id
)?;
```

`run(state, cancel)` は起動直後に 1 バッチ実行し、以降 60 秒周期。`spawn_all` に `SupervisedTask` `"CoinChargeDlq"` 登録。  
URL/secret 未設定かつ DLQ 行がある場合は `error!` ログ + 行保持。不正 JSON は `coin_charge_failed_poison` に隔離。

### テスト（既存資産再利用）

**モック**: `api_integration_tests/commerce.rs` L893– の axum `Router` + `TcpListener` + Bearer/OXP 検証をコピー。新規 HTTP モック枠組みを作らない。

| ID | 内容 |
|----|------|
| P1 | DLQ INSERT → `process_one_batch(&state)`（`pub(crate)`）→ mock 200 → 行 DELETE |
| N1 | mock 403/500 → 行数不変（再 INSERT なし） |
| N2 | 不正 JSON payload → poison 隔離（再送対象外） |
| R1 | relay 経由の既存 coin-charge テストが回帰しない |

`process_one_batch` を `run` ループから切り出し、テストは sleep なしで呼べるようにする。

### ゲート

「OP-060 を実装しろ」。OP-061 後推奨。人間レビュー。

---

## 7. Wave 3

### OP-054

| メソッド | 方針 |
|----------|------|
| `with_llm` | 呼び出しゼロ → `pub(crate)` |
| `from_pool` | `pub` 維持 |
| `set_embedding_provider` | `pub` 維持（api-server） |
| `get_embedding_provider` | `pub(crate)` 候補 |

### OP-051

`error_handling.md` §2 を正本に 3 階層 Decision（ADR-054 等）。一括置換禁止。設計承認後。

---

## 8. 検証プロトコル

1. Positive  
2. Negative（各 N*）  
3. Revert & Report  

---

## 9. コミット粒度

1. `fix(billing): OP-024 MCP suspend fail-closed`  
2. `refactor(deps): OP-067 html2md → htmd`  
3. `refactor(test): OP-069 create_test_app_state`（死コード削除含む）  
4. `docs(adr): OP-069 ADR-053`  
5. `refactor(oxp): OP-061`（承認後）  
6. `feat(commerce): OP-060 attempt_once + DLQ worker`（承認後）  
7+. OP-054 / OP-051  

---

## 10. 実装開始テンプレ

```
現在は実装フェーズです。Wave 1 の OP-024 から着手します。
OP-061/060 および OP-051 には触れません。
```
