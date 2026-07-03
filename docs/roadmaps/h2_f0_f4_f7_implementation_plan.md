# 実装計画書 H2: F-0 Secure Remote Access / F-4 MCP Provider / F-5 Soul Sync / F-6 Proof of Agent Work / F-7 リモート承認

**作成日**: 2026-07-03（偵察による実在確認済み。行番号は当日時点 — 着手時はアンカー文字列で再特定）
**実行順**: F-0 → F-4 → F-6 → F-7 → F-5（F-5 は Federation スコープ決定が未決なら**着手しない**）
**Safety-Critical 注意**: F-4 は `auth.rs` 系（Safety-Critical Zone）に触れる。**各項目は人間レビュー必須**。本計画書は変更内容を極小・明示的に規定する。

---

## 0. 共通の安全網

機能ごとに `feature/f0-remote-access` 等のブランチを切り、以下のベースラインを記録してから着手:

```bash
cargo check --workspace --tests
cargo test -p api-server 2>&1 | grep "test result"
cargo test -p infrastructure 2>&1 | grep "test result"
cargo test -p samsara-hub 2>&1 | grep "test result"      # F-5 のみ
cargo clippy --workspace --all-targets -- -D warnings
cd apps/management-console && npx jest 2>&1 | tail -4
```

共通ルール: 1項目=1コミット / 完了条件未達なら中断・報告 / アンカー文字列で再特定 / assert 緩和禁止。

---

# PART 0: F-0 Secure Remote Access（F-4/F-7 の前提）

## 現状理解

偵察結果（重要）: **api-server は既に `0.0.0.0` + `PORT`（デフォルト3015）で bind しており、localhost 固定ではない**。TLS 終端は未実装（plain HTTP、HSTS ヘッダのみ設定済み）。`docker/caddy/Caddyfile` テンプレートが存在し、`tests/deployment_config_tests.rs` が検証している。CORS は `ALLOWED_ORIGINS` 環境変数（production で未設定なら `exit(1)`）。

よって F-0 の作業は「実装」ではなく**公式手順の確立と検証**が主で、コード変更は最小。

## 作業項目

### Z-1: Caddy リバースプロキシ公式手順のドキュメント化＋設定テスト強化

- **対象**: `docs/guides/REMOTE_ACCESS.md`（新規）、`apps/api-server/docker/caddy/Caddyfile`（必要なら調整）、`tests/deployment_config_tests.rs`
- **変更**: (a) Caddyfile に TLS 自動取得（Let's Encrypt）＋ api-server 3015 への reverse_proxy ＋ WebSocket/SSE パススルー（`/api/stream/*`, `/api/v1/mcp/sse`）を明記。(b) ガイドに「独自ドメイン + Caddy」「Tailscale/WireGuard（TLS 不要の私設網）」の2構成を手順化。`ALLOWED_ORIGINS` と `PORT` の設定例を含める。(c) `deployment_config_tests.rs` に「Caddyfile が SSE パスの `flush_interval -1` を含む」検査を追加。
- **完了条件**: `cargo test -p api-server deployment_config` PASS。ガイドは docs-ui-ux-golden-rules 準拠。
- **依存**: 項目0

### Z-2: 外部公開時のセキュリティ E2E チェックリストと自動検査

- **対象**: `apps/api-server/tests/remote_exposure_tests.rs`（新規）
- **変更**: テストサーバー起動で (a) 未認証アクセスが 401 を返すルート網羅（`/api/v1/workflows` 等代表5系統）、(b) production ビルド相当（`ALLOWED_ORIGINS` 未設定）の CORS 起動失敗ロジック（`bootstrap/helpers.rs` の該当関数を直接ユニットテスト。アンカー: `AllowOrigin::any()`）、(c) レート制限（グローバル 50 req/s、エージェント別 60 req/min）の設定値が変わっていないことの回帰テスト。
- **完了条件**: 新テスト PASS。**Negative Test**: 一時的に `ALLOWED_ORIGINS` 検査を緩めてテストが FAIL することを確認→復元。
- **依存**: Z-1

---

# PART 1: F-4 Aiome as MCP Provider

## 現状理解

- MCP サーバーは `/api/v1/mcp/sse`（SSE）+ `/api/v1/mcp/messages`（POST）で稼働済み。`auth_middleware` 配下で JWT Bearer 必須。ツール公開は `is_skill_whitelisted()`（`mcp/server.rs`）のハードコード・ホワイトリスト。
- `Permission` enum と `has_permission()` は実装済み（`libs/shared/src/auth.rs`）。**しかし OAuth の `scope` は受理されるだけで JWT claims に未反映**（`authorize_handler` は pkce_cache に `(code_challenge, client_id)` のみ保存、`token_handler` は固定 `Role::Admin` を発行 — アンカー: `roles: vec![shared::auth::Role::Admin]`）。
- **PAT（長命トークン）の永続化テーブルは存在しない**（JWT はステートレス発行のみ）。失効を実現するには DB テーブルが必須。

## 新規契約（確定）

- テーブル `api_tokens`: `id TEXT PK, name TEXT, token_hash TEXT (SHA-256), scopes TEXT (JSON配列), created_at TEXT, expires_at TEXT NULL, revoked_at TEXT NULL`
- スコープ文字列: `mcp:read`（cortex_search/transcribe 等の読み取り系）/ `mcp:execute`（WASM スキル実行）。高リスクツール（fs_writer/terminal_exec/commerce 系）は**スコープに関わらず外部トークンでは常に拒否**（既存 `is_skill_whitelisted` の deny を維持）。
- API: `POST /api/v1/tokens`（発行、平文は応答で1度だけ返す）/ `GET /api/v1/tokens`（一覧、hash 非表示）/ `DELETE /api/v1/tokens/:id`（失効）。すべて Admin ロール必須。

## 作業項目

### T-1: `api_tokens` テーブルとストア（infrastructure）

- **対象**: `libs/infrastructure/migrations/sqlite/2026XXXX_api_tokens.sql`（+postgres 同名）、`libs/infrastructure/src/api_token_store.rs`（新規）、`lib.rs`
- **変更**: 上記スキーマの migration と、`create_token(name, scopes, expires_at) -> (id, plaintext)` / `verify_token(plaintext) -> Option<TokenRecord>`（SHA-256 照合、revoked/expired は None）/ `revoke(id)` / `list()` を持つストア。平文は `aiome_pat_` プレフィックス + 32 byte ランダム（`rand::rngs::OsRng`、既存依存）。
- **テスト**: roundtrip / 失効後 None / 期限切れ None の3本。
- **完了条件**: `cargo test -p infrastructure api_token_store` PASS、`aiome-migrate` でマイグレーション適用成功、clippy。
- **依存**: 項目0

### T-2: 認証ミドルウェアの PAT 受理（**Safety-Critical: 人間レビュー必須**）

- **対象**: `apps/api-server/src/auth.rs` の `Authenticated` エクストラクタ（アンカー: `validate_token(token)`）
- **変更**: Bearer 値が `aiome_pat_` で始まる場合のみ `api_token_store.verify_token` へ分岐し、成功時は `AiomeCustomClaims` 相当（`roles: vec![Role::Agent]`, scopes を拡張フィールドでなく **リクエスト拡張（axum Extension）に `TokenScopes(Vec<String>)` として格納**）を構築。JWT 経路は**1文字も変えない**。レート制限（`state.rate_limiter.check`）は PAT でも通す。
- **テスト**: `test_pat_authenticates` / `test_pat_revoked_rejected_immediately`（失効→即 401、受け入れ基準2）/ `test_jwt_path_unchanged`（既存 JWT テストが GREEN のまま）。
- **完了条件**: 3本 PASS、既存 auth 系統合テスト全 GREEN、clippy。**このコミットは人間レビューを経てからマージ**。
- **依存**: T-1

### T-3: MCP ツールのスコープ分離

- **対象**: `apps/api-server/src/mcp/server.rs` の `tools/call` ハンドラ（アンカー: `is_skill_whitelisted`）
- **変更**: リクエスト拡張から `TokenScopes` を取得（JWT 経路 = 拡張なし = フルアクセスで従来通り）。PAT の場合: `cortex_search`/`transcribe` は `mcp:read` 必要、WASM スキル実行は `mcp:execute` 必要。不足は JSON-RPC error（-32603, "insufficient scope"）+ HTTP 403。
- **統合テスト**: `test_mcp_read_scope_cannot_execute_skill`（read トークンで cortex_search 成功・スキル実行 403 — 受け入れ基準1）/ `test_mcp_jwt_unaffected`。
- **完了条件**: 2本 PASS、既存 MCP テスト GREEN、clippy。
- **依存**: T-2

### T-4: トークン管理 API + Settings UI

- **対象**: `routes/token.rs`（新規）+ router/api.rs 配線、`SettingsPage.tsx` に「API トークン」セクション
- **変更**: 発行（応答に平文1度だけ + 「再表示不可」文言）/ 一覧 / 失効。UI は発行フォーム（name, scopes チェックボックス, 期限）と一覧テーブル + 失効ボタン。
- **完了条件**: 統合テスト3本（auth 401 / 発行→一覧→失効 roundtrip / 失効後 MCP 401）PASS、Negative 配線確認、jest 追加2本 PASS。
- **依存**: T-3

### T-5: OAuth scope の claims 反映＋公開ドキュメント＋ドキュメント同期

- **対象**: `routes/auth.rs`（**Safety-Critical: 人間レビュー必須**）、`docs/guides/MCP_PROVIDER.md`（新規）
- **変更**: pkce_cache の値を `(code_challenge, client_id, scope)` に拡張し、`token_handler` で scope が `mcp:read` のみなら `roles: vec![Role::Agent]` を発行（scope 未指定は従来通り Admin — **後方互換維持**）。ガイドに Cursor/Claude からの接続手順（F-0 の公開手順を前提に）を記載。CHANGELOG/OPEN/roadmap 同期。
- **完了条件**: `test_oauth_scope_reflects_role` PASS、既存 OAuth テスト（`test_auth_full_oauth_workflow` 等）GREEN、docs-sync-check PASS。
- **依存**: T-4, Z-1

## F-4 やらないこと

1. `is_skill_whitelisted` の deny リスト（terminal_exec/fs_writer/forge_publish）を緩めない。
2. `AiomeCustomClaims` の構造体フィールドを追加しない（互換性破壊。スコープは axum Extension で運ぶ）。
3. OAuth の既定ロールを Admin から変えない（scope 明示時のみ縮小）。
4. OP-024（tool_call_router Fail-Closed）が未消化ならスキル実行系ツールの公開拡大をしない。

---

# PART 2: F-6 Proof of Agent Work

## 現状理解

- ハッシュチェーン: `InvariantDag::verify_chain()`（`libs/infrastructure/src/invariant_dag.rs`）が実装済み。ノードは `InvariantDagNode { hash, parent_hash, step_id, job_id, action, verified_invariants, timestamp }`。
- `OxiLeanProofCertificate` は **HMAC-SHA256**（共有秘密）であり**第三者検証には使えない**。第三者検証には Ed25519（公開鍵検証）が必要 → F-3 M-1 の `skills/package.rs` 署名関数を流用する。
- 検証系 API の参照実装: `routes/proof_verifier.rs`（パス検査＋SHA-256 ハッシュ束縛のパターン）。
- CLI の慣例: `apps/<name>/src/main.rs` + workspace members 追加。

## 証明書スキーマ v1（確定）

```json
{
  "certificate_version": 1,
  "job_id": "...", "category": "...", "completed_at": "...",
  "dag_nodes": [ /* InvariantDagNode の配列（該当 job のみ） */ ],
  "outcome": { "status": "completed" },
  "issuer_pubkey_b64": "...",
  "signature_b64": "<Ed25519 over sha256(canonical_json(上記全フィールド))>"
}
```

PII 禁止則: `action` はツール名・ステップ種別のみ（既存 DAG の仕様通り）。プロンプト本文・output_json は**含めない**。

## 作業項目

### W-1: 証明書生成モジュール（infrastructure）

- **対象**: `libs/infrastructure/src/proof_of_work.rs`（新規）、`lib.rs`
- **変更**: `generate_certificate(dag_nodes, job_meta, signing_key) -> Certificate` と `verify_certificate(cert_json) -> Result<(), String>`（(1) Ed25519 署名、(2) `InvariantDag::verify_chain` と同一ロジックのチェーン整合 — 関数を `pub` 化して再利用、コピー禁止）。鍵は F-4 と別の `PROOF_SIGNING_KEY_B64`（未設定時は 503 設計、自動生成禁止）。
- **テスト**: roundtrip / 1バイト改ざん FAIL / チェーン断絶 FAIL / PII 非含有（`output_json` 文字列がシリアライズ結果に現れない）の4本。
- **完了条件**: `cargo test -p infrastructure proof_of_work` 4本 PASS、clippy。
- **依存**: 項目0（`verify_chain` の `pub` 化は既存テスト GREEN 維持を確認）

### W-2: export API（api-server）

- **対象**: `routes/proof_of_work.rs`（新規）+ 配線 + OpenAPI
- **変更**: `GET /api/v1/jobs/:id/proof` — 該当 job の DAG ノードを取得（`dispatch_loop.rs` が DAG をどこに永続化しているかを実装時に確認し、未永続なら**中断して報告** — 本計画の前提が崩れるため）→ W-1 で署名 → JSON 返却。job が Completed 以外は 409。
- **完了条件**: 統合テスト2本（roundtrip / 非完了 409）PASS、Negative 配線確認。
- **依存**: W-1

### W-3: 独立検証 CLI `aiome-verify`

- **対象**: `apps/aiome-verify/`（新規 bin クレート、workspace members 追加）
- **変更**: `aiome-verify <cert.json> [--pubkey <b64>]` — DB 接続なしで W-1 の `verify_certificate` を呼び PASS/FAIL を exit code（0/1）と stdout で返す。依存は infrastructure（または proof_of_work を `libs/` の軽量クレートに置く判断は**しない**。infrastructure 依存で可）。
- **完了条件**: `cargo run -p aiome-verify -- fixture.json` が PASS、改ざん fixture で exit 1（**Negative Test**）、`cargo check --workspace`。
- **依存**: W-1

### W-4: F-2 連携とドキュメント同期

- **対象**: `OutcomeLedgerWidget`（F-2 実装済みの場合のみ）に「証明書を出力」リンク、CHANGELOG/OPEN/roadmap
- **完了条件**: docs-sync-check PASS。F-2 未実装ならこの項目はスキップし OPEN.md に後続タスクとして登録。
- **依存**: W-2, W-3

## F-6 やらないこと

1. OxiLean HMAC 証明書の置き換え・変更（Nurture 連携で稼働中。並存させる）。
2. ブロックチェーン・タイムスタンプ局への公証。
3. DAG スキーマの変更（`InvariantDagNode` に手を加えない）。

---

# PART 3: F-7 リモート承認（PWA）

## 現状理解

- 承認 API は完備: `GET /api/v1/jobs/awaiting-input`（AwaitingInput フィルタ）/ `POST /api/v1/jobs/:id/review`（approved→`requeue_job`、rejected→`fail_job`）。SSE `task_awaiting_input` イベントも `/api/stream/vitality` で配信済み（`stream.rs`）。
- **PWA 資産はゼロ**（vite-plugin-pwa / manifest / sw いずれも 0 件）。本機能は10機能中唯一フロント基盤を新設する。
- 前提: F-0 完了（HTTPS 到達）。**Web Push は HTTPS 必須**。

## 作業項目

### R-1: PWA 化（manifest + Service Worker、通知なし）

- **対象**: `apps/management-console/package.json`（`vite-plugin-pwa` を devDependencies に追加 — **本計画で唯一の依存追加。npm install はユーザー承認を得て実行**）、`vite.config.ts`、`public/` アイコン
- **変更**: `VitePWA({ registerType: 'autoUpdate', manifest: {...} })` を plugins に追加。`rollupOptions.input` の2エントリ（index.html, biome-popup.html）を壊さないこと。オフライン時は「接続が必要です」のフォールバックのみ（アプリ全体のオフライン対応はしない）。
- **完了条件**: `npm run build` 成功、dist に `manifest.webmanifest` と `sw.js` が生成される、既存 jest 全 GREEN。
- **依存**: 項目0

### R-2: 承認専用の軽量ビュー `/approve`

- **対象**: `src/components/ApprovalPage.tsx`（新規）、ルーティング（App.tsx のビュー切替慣例を実装時に確認して踏襲）
- **変更**: `TaskApprovalOverlay.tsx` のロジック（アンカー: `awaiting-input`）を共有フック `useApprovalQueue.ts` に抽出し（Overlay 側は挙動不変のままフック利用に置換）、スマホ幅最適化の一覧＋Approve/Reject ボタンのページを新設。**二重タップ防止**: 送信中は全ボタン disabled（既存 `isSubmitting` パターン踏襲）。ネットワーク断は明示エラー表示。
- **テスト**: `ApprovalPage.test.tsx` 3本（一覧表示 / 承認 POST / 二重クリックで POST 1回のみ）＋既存 `TaskApprovalOverlay` テストが GREEN 維持。
- **完了条件**: jest PASS、tsc 新規エラーなし。
- **依存**: R-1

### R-3: Web Push（VAPID）バックエンド

- **対象**: `libs/infrastructure/migrations/.../push_subscriptions.sql`（endpoint, keys_p256dh, keys_auth, created_at）、`routes/push.rs`（新規: `POST /api/v1/push/subscribe`, `DELETE /api/v1/push/subscribe`）、`dispatch_loop.rs` の `TaskEvent::AwaitingInput` 発火点近傍（アンカー: `trigger_job_completed` と同様の hook 位置）
- **変更**: VAPID 鍵は `VAPID_PRIVATE_KEY_B64` 環境変数。Web Push 送信は **RFC8291 実装が必要 = `web-push` 系クレートの依存追加が必要**。依存追加はユーザー承認事項のため、**実装前に候補クレート（`web-push` v0.x）と cargo audit 結果を報告して承認を得ること**。通知ペイロードは「承認要求があります」の定型文のみ（ジョブ内容・理由を**含めない** — 受け入れ基準4）。
- **完了条件**: 購読 roundtrip 統合テスト、送信はモックトランスポートでユニットテスト（実 push サービスへは送らない）。
- **依存**: R-2、F-0（HTTPS）

### R-4: 承認トークンの短命化・単回性

- **対象**: `routes/jobs.rs` の review ハンドラ**は変更せず**、`routes/push.rs` に「承認ディープリンク」発行を追加
- **変更**: push 通知に含める URL は `/approve?ticket=<one-time>`。ticket は `moka::future::Cache`（TTL 15分、使用時 invalidate — PKCE キャッシュと同じパターン、アンカー: `pkce_cache`）で管理し、`GET /api/v1/push/ticket/:ticket` が job_id を1度だけ返す。**review API 自体の認証は従来どおり JWT/PAT**（ticket は「どのジョブか」の解決のみで、認証代替にしない — 認証バイパスの構造的排除）。
- **統合テスト**: ticket 再利用 401 / 期限切れ 401（**Negative Test 必須**）。
- **完了条件**: 2本 PASS、既存 jobs テスト GREEN。
- **依存**: R-3

### R-5: E2E 検証とドキュメント同期

- 実機（スマホ）での E2E は手動チェックリスト（`docs/guides/REMOTE_APPROVAL.md` 新規）に記録。CHANGELOG/OPEN/roadmap 同期。
- **依存**: R-4

## F-7 やらないこと

1. review API の認証を ticket で代替しない（上記 R-4 の通り）。
2. アプリ全体のオフラインキャッシュ対応をしない。
3. ネイティブアプリ（iOS/Android）を作らない。
4. 通知に承認内容の平文を含めない。

---

# PART 4: F-5 Soul Sync（**着手ゲートあり**)

> [!WARNING]
> **着手条件**: `implementation_plan.md` の Open Question「Federation スコープ」で (A) 本実装が採択されていること。未決のまま着手することを禁止する。以下は採択後の計画。

## 現状理解

- 同期チャネル: samsara-hub に Automerge CRDT の実績あり（`handlers/timeline.rs` の `AutoCommit` merge、`hub_timeline.automerge_blob`）。E2E 暗号は `libs/shared/src/crypto.rs`（`ed25519_pubkey_to_x25519` / `encrypt_message` / `decrypt_message`）。
- Soul 側: `UniversalSoulStore::save_soul()`（`experience_buffer_json`）、`record_version()` + `parent_hash` によるバージョン履歴あり。**Soul 専用 diff API は未実装**。
- **ペアリングフローは存在しない**（federation は `FEDERATION_SECRET` 共有 or JWT）。

## 作業項目（要約粒度 — 着手決定後に本計画書と同粒度へ詳細化すること）

- **S-1**: `HubMessage` に `SoulSyncRelay(EncryptedEnvelope)` バリアント追加（`contracts.rs` アンカー: `ZeroMetadataCommuneRelay` の直後。hub は**中身を復号できない**封筒のみ中継 — 受け入れ基準2をアーキテクチャで担保）＋ hub 側 broadcast（`federation.rs` の `state.tx.send` パターン）。ログ・DB に平文が入らないことのテスト付き。
- **S-2**: ペアリング: 端末 A が `crypto.rs` の X25519 公開鍵を含む QR/コードを表示 → 端末 B が入力 → 相互に `paired_devices` テーブル（新規 migration）へ登録。解除 API で削除（解除後の同期拒否テスト必須）。
- **S-3**: Soul スナップショットの差分同期: `record_version` の `parent_hash` を lamport 的に利用し、`experience_buffer_json` を Automerge ドキュメント化（timeline.rs のパターン流用）。同一 Experience の二重適用が冪等であるテスト（受け入れ基準3）。
- **S-4**: 2ノード統合テスト（`libs/infrastructure/tests/` の federation テスト慣例に従う）: 端末 A の Experience が 60 秒以内に B へ反映。
- **やらないこと**: Soul 全体の実時間同期（バッチ同期のみ）/ hub への平文保存 / ペアリングなしの自動発見同期。

---

## 実行者への指示文（このままコピペ）

```
docs/roadmaps/h2_f0_f4_f7_implementation_plan.md を実行してください。
1. 実行順は F-0（Z-1→Z-2）→ F-4（T-1→T-5）→ F-6（W-1→W-4）→ F-7（R-1→R-5）。F-5 は着手ゲート未解除なら着手しない。
2. 1項目=1コミット。完了条件を満たしてからコミット。満たせなければ中断・報告。
3. T-2 と T-5 は Safety-Critical Zone（auth.rs 系）のため、コミット後に人間レビューを明示的に要求し、承認まで次項目へ進まない。
4. R-1 の npm 依存追加と R-3 の cargo 依存追加は、実行前に候補と監査結果を報告してユーザー承認を得る。
5. 「やらないこと」を先に読み全項目で遵守。アンカー文字列で対象を再特定。計画が「実装時に確認」と指定した箇所（W-2 の DAG 永続化等）は確認結果が前提と異なれば中断・報告。
6. 完了後、コミットハッシュ一覧と最終テスト結果を報告。git push はしない。
```

## ロールバック

各項目独立コミット。revert は各 PART 内で逆順。migration を含む項目（T-1, R-3, S-2）の revert 時は対応する DOWN 手順（テーブル DROP）を**ユーザー確認の上で**実施。
