> タスク追跡の正本は OPEN.md。本書は履歴・設計参照用。

# Aiome × Nurture シナジー最大化 実装計画書（実行者グレード v2）

> 作成: 2026-07-04 / 改訂: 2026-07-04（実コード検証に基づく実行者グレード化・/perfect-plan 検証2巡反映済み: v3）
> ステータス: **実装済み**（2026-07-04）
> 位置づけ: `synergy_pr_plan.md`（対外表現）の前提となる**機能実装**。
> 実行者への合格条件: この計画書とコードだけで、迷わず安全に完遂できること。
> ⚠️ W-1 / W-3 / W-6 / W-7c は Safety-Critical Zone（commerce / webhook / ledger）。**各項目とも着手前にユーザーの明示的な「実装しろ」承認が必須**。

---

## 1. 現状の構造ギャップ（検証済み事実）

| # | ギャップ | エビデンス | 影響 |
|---|---|---|---|
| G1 | **coin-charge relay の認証不整合** | 送信側 `apps/api-server/src/routes/commerce_webhook/relay.rs` L38–41 は `Authorization: Bearer` のみ。受信側 `commercial/apps/nurture-api/src/routes/internal/mod.rs` は L56 で全 `/internal/*`（18ルート・免除なし）に `require_oxp_certificate`（L59–119: OXP≥900・タイムスタンプ -60〜+300秒）を適用。さらに `main.rs` L291–296 の `internal_auth_middleware`（Bearer）との二重認証 | **本番障害級**: Stripe/Polar 決済 → KC チャージ／月次含み枠（OP-059、実装済み）が 403 で失敗。テストは OXP なし mock（`apps/api-server/src/api_integration_tests/commerce.rs` L612–637）のため検出不能 |
| G2 | **NurturePlugin 未登録** | `plugin_registry.register()` の実呼び出しゼロ。`preflight.rs` L223 で `PluginRegistry::new()` のみ。`database.rs` L130–133 のコメントが未実装を明示 | AgentHook・SecurityPolicy ツール登録・ルートマージが休眠 |
| G3 | **AgentHook 二系統・未到達** | api-server 側: `trigger_transaction_completed` の本番呼び出しゼロ。nurture-api 側: `state.rs` L151–155 の別インスタンスが Webhook 経由で `on_transaction_completed` を呼ぶ（サイドカー内のみ部分動作） | ジョブ完了・証明完了 → KarmaForge 統合が不成立。W-3 で二重発火リスク（ADR-012 で一本化判断） |
| G4 | **MCP ツールの宣言/公開不一致・導線なし** | 下表参照。デフォルトテンプレート（`apps/api-server/src/mcp/discovery.rs` L40–）に nurture なし。`is_skill_whitelisted`（`apps/api-server/src/mcp/server.rs` L391–407）に Nurture ツール名なし | エージェント自律経済が事実上未達 |
| G5 | **SurpriseEngine（A2C）未配線** | `evaluate_bonus`（`commercial/libs/nurture-core/src/a2c/surprise.rs` L10–29 相当、引数: `transaction_amount, daily_bonus_issued, max_daily_bonus, rng`）の呼び出しはテストのみ。`mcp_tools/buy.rs` に組み込みなし。台帳の `EntryType::SurpriseBonus`（`ledger.rs` L34）と記帳ロジックは実装済み | 「AI が恩返しする」の中核が未動作 |
| G6 | **コンソール可視化が断片的** | `NurtureDashboard.tsx`（`components/commerce/`）は `/commerce/points/` と `/commerce/history/` のみ（L74–81）。`ProUpgradeModal` は `App.tsx` に import もマウントもなし | Governed 訴求と実体験の乖離 |

### G4 詳細: ツール宣言と MCP 公開の差分表

| `NurturePlugin.registered_tools()`（plugin.rs L38–46） | nurture-api MCP `tools/list`（mcp/server.rs L134–181） | 状態 |
|---|---|---|
| `marketplace_search` | `market_search` | 名称不一致 |
| `marketplace_buy` | `buy` | 名称不一致 |
| `marketplace_upload` | （なし） | 未公開 |
| `wallet_balance` | （なし） | 未公開 |
| `sandbox_exec` | `sandbox_exec` | 一致 |
| （宣言なし） | `gift_delivery` はハンドラ（`mcp_tools/gift.rs`）実装済み・未公開 | 未公開 |

---

## 2. 安全網（項目0 — 最初に実行）

```bash
cd /Users/motista/Desktop/antigravity/aiome
git checkout main && git pull && git checkout -b feature/synergy-wiring
cargo check --workspace --tests && cargo test --workspace          # ベースライン記録
cargo test -p nurture-api                                          # internal_routes_test の 403 テスト PASS 確認
cd apps/management-console && npm run lint && npm test             # フロントベースライン
```

FAIL があれば中断して報告。**W-1 は「認証を弱める方向の事故」を防ぐため、先に受信側 403 テストの PASS を確認してから着手する。**

---

## 3. 作業項目（実行順・1項目=1コミット）

### W-1: coin-charge relay の OXP 証明書対応 【最優先・本番障害修正】

- **対象**: `apps/api-server/src/routes/commerce_webhook/relay.rs`（全105行）、呼び出し元3箇所、`libs/aiome-core-contracts/src/oxilean.rs`（共通ヘルパー追加）
- **問題**: G1
- **変更手順**:

**(1) relay 関数に OXP プロバイダ引数を追加** — `enqueue_coin_charge_to_nurture`（relay.rs L12–20）のシグネチャに `oxilean_power: std::sync::Arc<std::sync::atomic::AtomicU32>` を追加し、リトライループ内のリクエスト構築（L38–43）を以下に変更:

```rust
// 変更前（L38–43）
match http_client
    .post(&req_url)
    .header("Authorization", format!("Bearer {}", secret))
    .timeout(std::time::Duration::from_secs(30))
    .json(&payload)
    .send()

// 変更後
let mut req = http_client
    .post(&req_url)
    .header("Authorization", format!("Bearer {}", secret))
    .timeout(std::time::Duration::from_secs(30))
    .json(&payload);
if let Some(cert) = aiome_core_contracts::oxilean::OxiLeanProofCertificate::generate_header(
    "aiome-edge-node",
    oxilean_power.load(std::sync::atomic::Ordering::Relaxed),
    &secret,
) {
    req = req.header("X-OxiLean-Proof-Certificate", cert);
}
match req.send()
```

**ヘルパーの配置（車輪の再発明防止・v3 検証で確定）**: 同一ロジックが既に2箇所に存在する — `libs/aiome-commerce/src/stripe/mod.rs` L95–113 `generate_oxp_header()`（私有）と `apps/api-server/src/routes/auth.rs` L300–322（`/internal/forget` 用インライン・OXP=1000 固定）。relay.rs 内に3つ目を書くのは再発明になるため、**共通ヘルパーを型の正本 `libs/aiome-core-contracts/src/oxilean.rs` に追加**する（現状のメソッドは `generate()` L28 / `verify()` L47 のみ）:

```rust
impl OxiLeanProofCertificate {
    /// 証明書を生成し、X-OxiLean-Proof-Certificate ヘッダ値（Base64 JSON）に直列化する
    pub fn generate_header(node_id: &str, oxp: u32, secret: &str) -> Option<String> {
        let ts = chrono::Utc::now().to_rfc3339();
        let cert = Self::generate(node_id.to_string(), oxp, ts, secret);
        let cert_json = serde_json::to_string(&cert).ok()?;
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(cert_json))
    }
}
```

relay.rs からは `OxiLeanProofCertificate::generate_header("aiome-edge-node", oxilean_power.load(Ordering::Relaxed), &secret)` を呼ぶだけにする（api-server は `aiome-core-contracts` に依存済み。`base64` / `chrono` が aiome-core-contracts の依存に無ければ Cargo.toml に追加）。
**スコープ外**: `stripe/mod.rs` と `auth.rs` の既存2箇所をこのヘルパーに置換するリファクタは行わない（両方 Safety-Critical Zone のため。OPEN.md に「OXP ヘッダ生成の共通ヘルパーへの統一」として起票のみ）。

**注意**: OXP ヘッダ生成は**リトライループの内側**で毎回行う（証明書 TTL は5分だが、リトライ遅延は 1s→5s→25s のため再生成が安全）。

**(2) 呼び出し元3箇所に引数追加** — いずれも `state` が利用可能:

- `stripe.rs` L344–353（allowance）: `state.oxilean_power.clone()` を末尾引数に追加
- `stripe.rs` L441–453（pending_coin_charge）: 同上
- `polar.rs` L230–242: 同上

（`AppState.oxilean_power` は `app_state.rs` L169 定義、`bootstrap/mod.rs` L154 で 0 初期化、`internal_services/oxilean_poller.rs` が 60 秒ごとに gRPC `get_oxi_lean_status` で更新）

**(3) 起動直後（OXP=0 < 900）シナリオの扱い** — 403 → 3回リトライ → `outbox_dead_letters` DLQ 保存（relay.rs）。**OP-060（2026-07-09）で自動再送を追加**: `coin_charge_dlq_worker` が**起動直後に 1 バッチ**実行し、以降 60 秒周期で `attempt_coin_charge_once` を再送（成功時 DELETE）。設定欠落時は error ログ + 行保持。不正 JSON は `coin_charge_failed_poison` に隔離。

- **完了条件**:
  - `cargo check -p api-server` PASS
  - Positive: W-2 の本番同等 mock で OXP 付き relay が 200
  - Negative: 受信側 `internal_routes_test.rs` の 403 テスト（証明書なし L90–100 / スコア不足 L173–194）が引き続き PASS
  - `cargo test --workspace` PASS
- **リスク/戻し方**: 決済経路。回帰時は revert 1コミット。DLQ 滞留分の扱いを確認してから revert
- **依存**: なし
- **Safety-Critical**: **高** → 着手前にユーザー承認、完了後に人間レビュー

### W-2: integration test の本番同等化

- **対象**: `apps/api-server/src/api_integration_tests/commerce.rs` L612–637 の mock nurture ルーター
- **問題**: mock が認証検証ゼロのため G1 を検出できなかった（false positive）
- **変更**: mock の `/internal/coin-charge` ハンドラを、ヘッダ検証つきに置換:

```rust
let mock_nurture_app = axum::Router::new().route(
    "/internal/coin-charge",
    axum::routing::post(move |headers: axum::http::HeaderMap, _req: axum::extract::Request| async move {
        // 本番同等: Bearer + OXP 証明書の二重検証
        let bearer_ok = headers.get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(|v| v == "Bearer mock_secret")
            .unwrap_or(false);
        let cert_ok = headers.get("x-oxilean-proof-certificate")
            .and_then(|h| h.to_str().ok())
            .map(|b64| {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.decode(b64).ok()
                    .and_then(|j| serde_json::from_slice::<aiome_core_contracts::oxilean::OxiLeanProofCertificate>(&j).ok())
                    .map(|c| c.verify("mock_secret") && c.oxp_score >= 900)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !(bearer_ok && cert_ok) {
            return (axum::http::StatusCode::FORBIDDEN, axum::response::Json(serde_json::json!({"error":"forbidden"}))).into_response();
        }
        counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        axum::response::Json(serde_json::json!({"status": "success"})).into_response()
    }),
);
```

テスト側では `state.oxilean_power` を 900 以上に設定してから webhook を発火する（`state.oxilean_power.store(950, Ordering::Relaxed)`）。さらに **OXP=0 のまま発火 → sync_counter が増えず DLQ に書かれる** Negative ケースを1本追加

- **完了条件**: W-1 の relay 修正を一時 revert するとこのテストが FAIL する（テスト自体の Negative 検証）。恒久状態では PASS
- **依存**: W-1
- **Safety-Critical**: 低

### W-3: NurturePlugin の in-process 登録（feature = "nurture"）

- **対象**: `apps/api-server/src/bootstrap/database.rs`（登録位置）、`preflight.rs`、`bootstrap/mod.rs`
- **前提1（検証済み）**: Cargo 依存の追加は**不要**。`apps/api-server/Cargo.toml` に `nurture-api = { path = ..., optional = true }` と feature `nurture` が定義済み
- **前提2（必須先行タスク）**: **ADR-012「AgentHook 発火経路の一本化」を起票しユーザー判断を得る**。論点: in-process 登録時、nurture-api サイドカー内の Hook（`state.rs` L151–155、Webhook 用）と api-server 登録 Hook が**二重に KarmaForge 合成**する恐れ。推奨案: in-process モード（`NURTURE_IN_PROCESS=true`）ではサイドカーを起動しない（Tauri の `resolve_nurture_mode()` に `InProcess` variant を追加し、`NurtureMode::Local` より優先判定）
- **登録位置の設計（重要・検証済み）**: hook 登録ループは `database.rs` L131–136 にあり、`job_queue` は同関数内で先に構築される。`create_plugin()`（`commercial/apps/nurture-api/src/plugin.rs` L213–225）の10引数のうち bootstrap 初期で揃わないもの（`auth_manager`, `drm_master_key` 等）があるため、**登録は `init_database` 内・hook ループ直前**に行い、必要な依存はこの時点で構築済みのものを使う。揃わない場合は hook ループを `init_core_services` 後へ移動する改修も許容（その場合は `database.rs` から `HookManager` の finalize を分離し、`bootstrap/mod.rs` の順序 L157–174 に新ステップを挿入）
- **変更スケッチ**（`database.rs` L130 付近、cfg ゲート）:

```rust
#[cfg(feature = "nurture")]
if std::env::var("NURTURE_IN_PROCESS").map(|v| v == "true" || v == "1").unwrap_or(false) {
    let plugin = nurture_api::create_plugin(
        nurture_pool, system_id, event_sender.clone(), job_queue.clone(),
        cancel_token.clone(), nurture_secret, stripe_webhook_secret,
        polar_webhook_secret, auth_manager.clone(), drm_master_key,
    ).await?;
    plugin_registry.register(plugin);
    tracing::info!("🔌 [Plugin] Nurture registered in-process");
}
// 既存: for hook in plugin_registry.get_agent_hooks() { ... }
```

- **スコープ外（明示）**: `plugin.commerce_engine()` の DI は行わない（CommerceEngine は既存 Factory の HTTP プロキシを正とする。二重台帳リスク回避）。`merge_routes()` によるルートマージは行うが、`/api/v1/mcp` の重複が既存ルートと衝突しないか `router.rs` L709 で確認
- **完了条件**:
  - `cargo check --workspace`（feature なし）が現状と同一警告数で PASS（デフォルト挙動不変）
  - `cargo test -p api-server --features nurture` PASS
  - Positive: `NURTURE_IN_PROCESS=true` 起動で `get_agent_hooks()` が1件以上返り、`trigger_job_completed` 経由で KarmaForge 合成が呼ばれる統合テスト
  - Negative: feature なしビルドに nurture シンボルが存在しない（`cargo tree -p api-server | grep nurture` が空）
- **依存**: W-1、ADR-012 承認
- **Safety-Critical**: 中 → 着手前にユーザー承認

### W-4: MCP ツール名称統一と公開範囲拡張

- **対象**: `commercial/apps/nurture-api/src/mcp/server.rs`（tools/list L134–181、tools/call L225–377）、`commercial/apps/nurture-api/src/plugin.rs` L38–46
- **変更手順**:
  1. tools/list の `market_search` → `marketplace_search`、`buy` → `marketplace_buy` に改名（description / input_schema は維持）
  2. tools/call の match を後方互換に: `"marketplace_search" | "market_search" => ...`、`"marketplace_buy" | "buy" => ...`
  3. `wallet_balance` を tools/list に追加（input_schema: `{"type":"object","properties":{"agent_id":{"type":"string"}},"required":["agent_id"]}`）し、tools/call から**既存の `mcp_tools::wallet::handle_get_balance`（`mcp_tools/wallet.rs` L20–25。HTTP ルート `routes/wallet.rs` L48 で使用実績あり）を呼び出す**。ハンドラの新設は不要（v3 検証で既存確認済み・再発明禁止）
  4. `marketplace_upload` の公開: **CSAM 3層検査は MCP ハンドラ側で実施済みと検証確認**（`mcp_tools/upload.rs` L148–168 で `state.csam_pipeline.run_all()` を呼んでいる。2026-07-04 v3 検証）。公開してよいが、tools/call への配線時に CSAM 拒否パスの Negative テスト（違反コンテンツ相当の mock で reject）を必ず追加する
  5. `gift_delivery` は W-6 とセットで判断（本項では公開しない）
  6. `plugin.rs` `registered_tools()` を tools/list と完全一致に更新
- **完了条件**: `tools/list` 応答と `registered_tools()` の一致を検証するユニットテスト新設。旧名（`market_search`/`buy`）での tools/call が引き続き 200 を返すテスト。`cargo test -p nurture-api` PASS
- **依存**: なし（W-3 と並行可）
- **Safety-Critical**: 低〜中（upload 公開時のみ CSAM 確認必須）

### W-5: エージェントへの Nurture MCP 導線（認証プロキシ + seed）

- **対象**: api-server 新規ルート、`apps/api-server/src/mcp/server.rs` L391–407、`apps/api-server/src/system_instructions.rs` L80–92、`apps/api-server/src/mcp/discovery.rs`
- **設計原則**: **`NURTURE_INTERNAL_SECRET` をエージェント・フロント・MCP 設定ファイルに書かない**。api-server がプロキシとして Bearer＋OXP を付与
- **変更手順**:
  1. **プロキシルート新設** `apps/api-server/src/routes/nurture_mcp_proxy.rs` — 認証必須（既存 auth middleware 配下）。`Authorization: Bearer {NURTURE_INTERNAL_SECRET}` を付与して nurture-api へ転送する。**転送先の実構造（v3 検証で確定）**: nurture の MCP は単一 POST エンドポイントではなく **SSE 2段構成**（`routes/mod.rs` L27 で `/api/v1` 配下に `.nest("/mcp", …)`、`mcp/server.rs` L28–30: `GET /api/v1/mcp/sse` でセッション確立 → SSE が返す `POST /api/v1/mcp/message?sessionId=…` へ JSON-RPC を送る）。認証は `main.rs` L310–315 の `internal_auth_middleware`（Bearer のみ・OXP 不要）。したがってプロキシは (a) `GET /api/v1/nurture-mcp/sse` と `POST /api/v1/nurture-mcp/message` の2ルートを素通しプロキシする、または (b) プロキシ内部で SSE セッションを管理し単一 `POST /api/v1/nurture-mcp` に集約する、のどちらかを選ぶ。**推奨は (a) 素通し**（セッション状態を持たず単純）。`NURTURE_API_URL` 未設定時は 503 + `{"error":"nurture_disabled"}`。router.rs と OpenAPI に登録
  2. **ツール検出**: HTTP 型 MCP エントリは**サポート済みと検証確認**（`apps/api-server/src/mcp/config.rs` L35–38 `McpTransport::Http`＋L55 `url` フィールド、`discovery.rs` L229–357 `connect_http_server()`。※ `libs/infrastructure/src/mcp/` は存在しない。実装はすべて `apps/api-server/src/mcp/`）。discovery テンプレート（`discovery.rs` L40 `default_config`）に `"nurture": {"transport": "http", "url": "http://localhost:{API_PORT}/api/v1/nurture-mcp/sse"}` を seed する（既存テンプレの HTTP 型記法 L117–144 に合わせる）
  3. **whitelist 追加**（`mcp/server.rs` L391–407）: `"marketplace_search" | "market_search" | "wallet_balance" => true` を追加。**`marketplace_buy` は本項では `false` のまま**（購買は EconomyInterceptor＋日次上限の防壁があるが、承認キュー統合の検証が終わるまで自律実行は解禁しない。解禁判断は独立タスクとして OPEN.md に起票）
  4. **システムプロンプト**（`system_instructions.rs` L80–92 の `economy_prompt`）: `economic_context` がある場合の文言に「利用可能ツール: marketplace_search（市場検索）, wallet_balance（残高確認）」を追記
- **完了条件**:
  - Positive: 認証済みユーザーのプロキシ経由 `tools/list` が nurture のツールを返す統合テスト
  - Negative: 未認証で 401。`NURTURE_API_URL` 未設定で 503。`marketplace_buy` の tool call が whitelist で拒否される
  - `cargo test -p api-server` PASS
- **依存**: W-4
- **Safety-Critical**: 中（エージェントに経済読取能力を付与。購買は未解禁）

### W-6: SurpriseEngine（A2C）の配線

- **対象**: `commercial/apps/nurture-api/src/mcp_tools/buy.rs`（決済成功後 L183 付近）、`commercial/libs/nurture-core/src/policy.rs`、`commercial/specs/NurtureEconomyProtocol.tla`
- **必須先行タスク（TLA+）**: 現仕様の不変条件は `CoinsConserved == TotalCoins = (Cardinality(Users) * 100)`（L59–61）で **mint を許容しない**。SurpriseBonus はコイン発行（mint）のため、**先に仕様を改訂**する: `minted` 変数を追加し `CoinsConserved == TotalCoins = (Cardinality(Users) * 100) + minted`、SurpriseBonus アクションに `minted' = minted + bonus` と日次上限ガードを定義 → TLC モデル検査 PASS → 実装着手。仕様改訂は `commercial/specs/` の変更として独立コミット
- **実装スケッチ**（buy.rs、`settle()` 成功後・DRM 発行の後）:

```rust
// A2C: サプライズボーナス評価（W-6）
// 注意: sum_today / record_surprise_bonus は現存しない（v3 検証）。sum_today は新規追加、
// 記帳は既存 trait メソッド record_entry()（nurture-core/src/ledger.rs L54）＋ LedgerEntry で行う
let today_issued = state.ledger.sum_today(EntryType::SurpriseBonus).await.unwrap_or(u64::MAX);
let mut rng = rand::thread_rng();
if let Some(bonus) = SurpriseEngine::evaluate_bonus(
    receipt.amount, today_issued, state.policy.max_daily_surprise_bonus, &mut rng,
) {
    let entry = LedgerEntry {
        entry_type: EntryType::SurpriseBonus,
        // buyer_id / bonus / receipt.transaction_id を既存 SurpriseBonus 記帳経路
        // （nurture-infra/src/economy/ledger.rs L460–572）と同じフィールド構成で埋める
        ..
    };
    state.ledger.record_entry(entry).await?;
}
```

  - `EconomyPolicy`（policy.rs L18–28、現9フィールド）に `pub max_daily_surprise_bonus: u64` を追加（default は控えめに例: 500 KC/日。値はユーザー確認）。**波及箇所（v3 検証で全列挙済み）**: `nurture-core/src/policy.rs`（Default L32–45・`validate()` L48+）、`nurture-api/src/main.rs` L85–150（DB `nurture_settings.economy_policy` ロード — 既存レコードとの後方互換のため `#[serde(default = ...)]` 必須）、`nurture-api/src/plugin.rs` L226、`nurture-infra/src/economy/interceptor.rs`、`bridge/mod.rs` L274 `reload_policy()`、`settlement.rs` L279+、`nurture-api/tests/*.rs` の fixture 多数
  - `sum_today(EntryType) -> u64` を `EconomyLedger` trait（`nurture-core/src/ledger.rs` L52–68）と `DatabaseEconomyLedger` に**新規追加**（既存の日次集計メソッドは無いことを確認済み）
  - 記帳は既存の `EntryType::SurpriseBonus` 経路（`nurture-infra/src/economy/ledger.rs` L460–572 に記帳処理実装済み）を使用し、Merkle チェーンに乗せる
- **完了条件**:
  - TLC: 改訂後 `NurtureEconomyProtocol.tla` のモデル検査 PASS
  - Positive: 当選条件を固定 RNG（`StdRng::seed_from_u64`）で再現し、記帳＋Merkle 整合（記帳後の `verify_chain` 相当）を統合テストで確認
  - Negative: `max_daily_surprise_bonus` 超過時に付与されない／`daily_bonus_issued >= max` で `evaluate_bonus` が None（既存ユニットテストを流用）
  - `cargo test -p nurture-api -p nurture-core -p nurture-infra` PASS
- **依存**: W-1（台帳整合の前提）、TLA+ 改訂の完了
- **Safety-Critical**: **高**（コイン発行）→ 着手前にユーザー承認、完了後に人間レビュー

### W-7: コンソール可視化の補完（サブ項目ごとに独立コミット）

- **対象**: `apps/management-console/src/`
- **W-7a: KC+KP 統合表示** — `components/commerce/NurtureDashboard.tsx` の `fetchData`（L71–81）の `Promise.all` に `authenticatedFetch(\`${API_BASE}/api/v1/commerce/balance/${agentId}\`)` を追加し、既存 balance カード群（L121 以降の grid）に「AiomeCoin (KC)」カードを追加。表示は既存 `system-panel` スタイル踏襲
- **W-7b: SurpriseBonus/ギフト履歴** — 既存 `/api/v1/commerce/history/:id` の応答に含まれる entry_type でフィルタするタブを NurtureDashboard に追加（`SurpriseBonus` / gift 系を抽出表示）。バックエンド変更なしで実現できるか history 応答スキーマを確認し、entry_type が落ちている場合はバックエンド側の応答拡張を OPEN.md に起票して**本項では見送り**
- **W-7c: ProUpgradeModal マウント（OP-058）** — `App.tsx` に lazy import（L62 付近の既存パターン踏襲: `const ProUpgradeModal = React.lazy(() => import("./components/commerce/ProUpgradeModal"));`）を追加しルートレベルでマウント。**props 契約と 402 検知は検証済み（v3）**: props は `priceId: string` 必須・`agentId?: string`（`ProUpgradeModal.tsx` L13–16）。モーダル自身が `stripe-402-payment-required` カスタムイベントを購読しており（L22–30）、`lib/auth.ts` L54–57 の `authenticatedFetch` が 402 応答時に同イベントを dispatch する。よってマウントするだけで既存 402 連携が成立する。`priceId` は Pro プランの Stripe Price ID を設定（env/設定値の取得元をユーザーに確認）。**commerce 系 Safety-Critical のため独立承認**
- **W-7d: 月間支出上限（OP-059 残）** — バックエンド: `nurture_wallets` に月次上限カラム or `system_settings` キー `monthly_spend_limit` を追加し `EconomyInterceptor` のプリフライトに月次判定を追加。フロント: SettingsPage に入力欄。**DB マイグレーションを伴うため設計を ADR 化してから着手**
- **完了条件**: 各サブ項目で `npm run lint && npm test` PASS。W-7a/b は表示ユニットテスト追加。W-7d は Interceptor の Negative テスト（上限超過購入が拒否）必須
- **依存**: W-1（表示データの正しさ）。W-7b は W-6 完了後
- **Safety-Critical**: W-7c 中・W-7d 高

### W-8: ドキュメント同期＋訴求解禁

- **対象**: `CHANGELOG.md`、`OPEN.md`（OP-058/059 消し込み・新規起票: marketplace_buy 自律解禁判断、DLQ 自動再送の要否、W-7b 応答拡張、stripe/auth の OXP ヘッダ生成を共通ヘルパーへ統一するリファクタ等）、`docs/architecture/AIOME_NURTURE_SYNERGY.md`（シーケンス図・依存マップ同期: AGENTS.md ルール11）、`.context/RIPPLE_MAP.md`（relay.rs / buy.rs / surprise.rs / nurture-api mcp/server.rs を追記）、ADR-012（W-3 前に作成済みのはず）、`.env.example`（`NURTURE_IN_PROCESS` 追加時）
- **訴求解禁**: 完了した W 項目に応じて `synergy_pr_plan.md` の「やらないことリスト」該当項目を解除し、`MESSAGING.md` §2.5 に追記（例: W-5 完了 → 「エージェントが市場を自分で検索できる」解禁。W-6 完了 → 「AI からのサプライズ還元」解禁）
- **完了条件**: `bash scripts/docs-sync-check.sh --ci` PASS
- **依存**: W-1〜W-7 の完了状況に応じ段階実施

---

## 3.5 パーフェクトプランニング検証結果（2026-07-04 実施・v2 に反映済み）

- **Gate 1（構造）**: 核心事実はコードベースと一致。パス誤記2件（discovery.rs / commerce.rs）修正済み。W-3 の Cargo 依存は追加不要と確認
- **Gate 2（要件）**: 独立要件定義書は不在。`gift_delivery` の CSAM 確認を W-4 に明文化。OP-059 バックエンドは実装済み＝「W-1 の 403 で塞がれている」が正確
- **Gate 3（依存）**: relay 修正の波及は3経路（stripe.rs ×2 / polar.rs）。PluginRegistry 参照は8ファイル。RIPPLE_MAP に relay/buy/surprise の記載なし → W-8 で追記
- **Gate 4（悪魔の弁護人）**: ①Hook 二重発火 → ADR-012 を W-3 前に必須化 ②「OXP≥900 が常に満たされる」前提 → DLQ 再送テストを W-1/W-2 に組込 ③W-1/W-2 のみ先行するフェーズ分割は合理的・全面保留は不可
- **Gate 5（順序）**: W-1→W-2→W-3 妥当。W-4 は W-3 と並行可。W-6 の TLA+ 改訂は先行着手可

**判定: ⚠️ PATCH → 修正反映済み、実行可能**

## 3.6 検証2巡目（2026-07-04・車輪の再発明スキャン・v3 に反映済み）

実コードベースを対象に「既存実装の見落とし・再発明・パス誤り」を全項目照合した結果:

| 項目 | 発見 | 反映 |
|---|---|---|
| W-1 | OXP ヘッダ生成が既に2箇所に重複実装されていた（`stripe/mod.rs` L95–113 私有・`auth.rs` L300–322 インライン）。relay に3つ目を書くのは再発明 | 共通ヘルパー `OxiLeanProofCertificate::generate_header()` を型の正本 `oxilean.rs` に追加する方式へ変更。既存2箇所の置換は Safety-Critical のためスコープ外（OPEN.md 起票） |
| W-1(3) | DLQ 自動再送は OP-060 で実装済み（`coin_charge_dlq_worker`） | 手動再送ドキュメントは補助。自動再送が正本 |
| W-4 | wallet 残高ハンドラ `handle_get_balance` が `mcp_tools/wallet.rs` L20–25 に既存（HTTP ルートで使用実績あり）。新設は再発明 | tools/call から既存関数を呼ぶ配線のみに変更 |
| W-4(4) | `mcp_tools/upload.rs` L148–168 で CSAM 3層検査（`csam_pipeline.run_all()`）実施済みと確認 | 「grep で確認」条件を「検証済み・公開可（Negative テスト必須）」に確定 |
| W-5 | nurture MCP は単一 POST ではなく SSE 2段構成（`GET /mcp/sse` → `POST /mcp/message?sessionId=…`）。認証は Bearer のみ（OXP 不要）。HTTP transport は `config.rs` L35–38 でサポート済み。`libs/infrastructure/src/mcp/` は不存在 | プロキシ設計を「SSE+message の素通し2ルート」に修正。HTTP seed を確定案に昇格 |
| W-6 | `sum_today` / `record_surprise_bonus` とも不存在。記帳は trait の `record_entry()`（ledger.rs L54）経由が正。`EconomyPolicy` は現9フィールドで DB ロード時の serde 後方互換が必要 | スケッチを `record_entry()` 使用に修正。`#[serde(default)]` 必須と波及8箇所を明記 |
| W-7c | props（`priceId` 必須）・402 イベント連携（`stripe-402-payment-required`、`auth.ts` L54–57 dispatch / モーダル L22–30 購読）を確認 | 「確認して」を確定事実に置換。`priceId` の値取得元のみユーザー確認事項として残存 |
| W-2 / W-3 / W-8 | 計画のまま妥当 | 変更なし |

**判定: ⚠️ PATCH → v3 反映済み、実行可能**

---

## 4. 効果×リスク マトリクス

| 項目 | 効果 | リスク | Safety-Critical | 優先 |
|---|---|---|---|---|
| W-1 relay OXP | ★★★（本番障害修正） | 低 | 高 | 1 |
| W-2 テスト同等化 | ★★ | 低 | 低 | 2 |
| W-3 Plugin 登録 | ★★★ | 中（二重化→ADR-012） | 中 | 3 |
| W-4 MCP 名称統一 | ★★ | 低 | 低〜中 | 4 |
| W-5 MCP 導線 | ★★★ | 中 | 中 | 5 |
| W-6 SurpriseEngine | ★★ | 中（mint→TLA+先行） | 高 | 6 |
| W-7 コンソール | ★★ | 低〜中 | 一部高 | 7 |
| W-8 文書＋解禁 | ★★★ | なし | なし | 8 |

## 5. やらないことリスト

1. **CommerceEngine の in-process DI**（W-3 で明示除外。二重台帳リスク。将来 ADR）
2. **価格・手数料・EconomyPolicy 既存フィールドのデフォルト値変更**（`max_daily_surprise_bonus` の新規追加を除く）
3. **`/internal/*` の認証弱体化**（OXP 免除案は不採用）
4. **nurture-api サイドカー内 Webhook Hook の削除**（一本化は ADR-012 の判断。**ADR-012 は W-3 のコード変更前に完了必須**）
5. **エージェントへの internal Bearer 直接配布**（プロキシ必須）
6. **`marketplace_buy` の自律実行解禁** — 追跡は [`nurture_remaining_ledger_plan.md`](nurture_remaining_ledger_plan.md) **NR-09 / Wave B'**（既定: MCP whitelist 凍結。新 `PurchasePolicy` 禁止）
7. **未検証機能の対外訴求**（W-8 で段階解禁）

## 6. 検証プロトコル（全項目共通・AGENTS.md 準拠）

1. **Positive**: 期待通り動作
2. **Negative（必須）**: 認証欠如・上限超過・feature 無効・RNG 非当選で正しく拒否
3. **Revert & Report**: 注入障害を戻し正常復帰を確認して報告

## 7. 実行者への指示文

> `docs/roadmaps/synergy_maximization_plan.md` に従い、項目0（安全網）→ W-1 → W-2 → …の順に実施してください。
> - 1項目ずつ実施し、1項目ごとにコミット（Conventional Commits）
> - **W-1 / W-3 / W-6 / W-7c / W-7d は着手前にユーザーの明示的な「実装しろ」を必ず得ること**
> - W-3 の前に ADR-012、W-6 の前に TLA+ 仕様改訂＋TLC PASS を完了させること
> - 各項目の完了条件（Positive/Negative）を満たせなければ中断して報告
> - 計画にない機能追加・リファクタリング・依存更新は禁止
