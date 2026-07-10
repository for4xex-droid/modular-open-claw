# Aiome & Nurture v1.0.0 Release Roadmap — Revised (Deep Audit v2 反映版)

Phase 8.5 完了後の本番リリースに向けた開発計画です。
Deep Audit v2 で発見された **Federation 層のハリボテ化**、**scrub_env 矛盾**、**SSRF 脆弱性** 等の構造的欠陥を反映し、**Phase 3.5 (Infrastructure Remediation)** を最優先として挿入しました。

---

## 🚦 改訂版 Execution Order

| 優先度 | フェーズ | 概要 | 推定工数 | 前提条件 |
|:---:|:---|:---|:---:|:---|
| **P0** | **Phase 3.5: Infra Remediation** | 致命的バグ修正 + 構造整理 | **1-2日** | なし（即時着手可能） |
| P1 | Phase 4: Biome Reputation | 信頼性基盤の実装 | 1-2週 | P0完了 + Open Question 決定 |
| P2 | Phase 5: Cognitive Observability | 思考プロセス可視化 | 1週 | P1完了 |
| P3 | UI/UX 強化 | 管理コンソール統合 | 1-2週 | P1,P2完了 |
| P4 | Release Preflight & Audit | 最終セキュリティ監査 | 2-3日 | P3完了 |

---

## [P0] Phase 3.5: Infrastructure Remediation 🔴

> [!CAUTION]
> このフェーズは v1.0.0 リリースの**前提条件**であり、省略不可。3件の CRITICAL バグが本番環境で確実に障害を引き起こす。

### 即時修正（CRITICAL — 1日以内）

#### [MODIFY] [auth.rs](../../apps/api-server/src/routes/auth.rs)
- **問題（当時）:** `std::env::var("API_SERVER_SECRET")` が bootstrap の `scrub_env` と矛盾し、アカウント削除APIが常に失敗する
- **修正（完了・OP-061 追完 2026-07-10）:** Nurture forget は `state.nurture_internal_secret`（`NURTURE_INTERNAL_SECRET`）で OXP 署名 + `Authorization: Bearer`。URL は `state.nurture_url`（`NURTURE_API_URL`）。**`API_SERVER_SECRET` で forget を署名してはならない**（Nurture 側は `NURTURE_INTERNAL_SECRET` で検証する）

#### [MODIFY] [forecast.rs](../../apps/api-server/src/routes/forecast.rs)
- **問題:** L42 の `series_id` がエスケープなしに URL 文字列結合（SSRF/URL Injection）、L39 の `Client::new()` がコネクション枯渇リスク
- **修正:** `url::form_urlencoded` でパラメータエスケープ、`aiome_core::http::get_http_client()` に置換

#### [MODIFY] [main.rs](../../apps/api-server/src/main.rs) + [samsara-hub main.rs](../../apps/samsara-hub/src/main.rs)
- **問題:** `panic!()` が残存（CWE-209 情報漏洩リスク）
- **修正:** `error!()` + `std::process::exit(1)` パターンに統一

### 構造整理（HIGH — 1日）

#### [MODIFY] [docker_conductor.rs](../../libs/infrastructure/src/docker_conductor.rs)
- Podman 互換のポート割り当て（`portpicker` クレート導入 or ホスト側の動的取得）

#### [MODIFY] [federation.rs](../../libs/infrastructure/src/job_queue/federation.rs)
- 5つのスタブメソッドに `tracing::warn!("Federation stub: not implemented")` + 明示的なドキュメントコメントを追加
- Phase 4 で本実装する設計意図を ADR として記録

#### [NEW] `apps/api-server/src/test_helpers.rs`
- 4ファイルに重複する TempDir + SQLite + WasmSkillManager 初期化コードを `create_test_app_state()` に抽出

### コード品質（MEDIUM — 任意のタイミング）

#### [MODIFY] 7箇所の `unimplemented!()` → `Err(AiomeError::Infrastructure { reason: "Not implemented in mock" })`
- gig_gateway.rs, cortex_query.rs, planner.rs, discovery.rs

---

## [P1] Phase 4: Biome Reputation の構築

> [!IMPORTANT]
> **前提条件:** Phase 3.5 の Federation スタブ整理が完了していること。Open Question の決定が必要。

### 実装内容

- `KarmaForge::evaluate_trust_score` の精度向上（トランザクション履歴 + 行動履歴に基づく動的スコアリング）
- `BiomeRegistry` との連携による Reputation の永続化とスナップショット生成
- 悪意のあるアクターの自動検知とペナルティ（Slash/Stake 没収）ロジック
- **Federation `do_import_federated_data` の本実装**（Open Question の決定次第でスコープ変動）

---

## [P2] Phase 5: Cognitive Observability の高度化

### 実装内容

- プロンプトチェーンおよび推論パスの構造化ロギング（OpenTelemetry / Prometheus 統合拡充）
- `sage_meditation` のサンドボックス内実行トレーサビリティ
- Causal Visualizer API (`/api/v1/observability/causal-graph`)
- **出力前 Data Masking**（CWE-209 対策、PII/APIキーのスクラバー適用）

---

## [P3] UI/UX 強化

### 実装内容

- LoRA マーケットプレイスのフロントエンド
- Biome / Karma リアルタイムダッシュボード
- Settings UI の Nurture 経済圏パネル拡充

---

## [P4] Release Preflight & Audit

### 実装内容

- `/red-team` + `/codeql-scan` による全方位ペネトレーションテスト
- ライセンス監査 (`/license-check`)
- 本番 CI/CD パイプライン完成（署名検証、デプロイメント戦略）

---

## 🚨 Gate Checks（改訂版）

| Gate | 視座 | v1 判定 | v2 改訂 |
|:---|:---|:---:|:---|
| **Gate 1** | 構造 | ✅ | ⚠️ Federation 層のハリボテ + God Object (bootstrap 1,446行, samsara-hub 2,320行) を検出。Phase 3.5 で対処。 |
| **Gate 2** | 要件 | ✅ | ⚠️ Federation import/sync が未実装のため、P2P Hub 要件は**未充足**。Phase 4 でスコープ決定が必要。 |
| **Gate 3** | 波及 | ⚠️ | ⚠️ NAPI Bridge の独立初期化パスが将来の統合で衝突する可能性。文書化で対処。 |
| **Gate 4** | Red Team | - | 🔴 SSRF (forecast.rs) + scrub_env 矛盾 (auth.rs) を新たに検出。Phase 3.5 で即時修正。 |
| **Gate 5** | 順序 | ✅ | ✅ Phase 3.5 → 4 → 5 → UI → Audit の論理的依存関係を担保。 |

---

## 🙋 Open Questions

> [!IMPORTANT]
> ### Federation 実装のスコープ決定
>
> Phase 4 (Biome Reputation) は Federation 基盤の上に乗る設計ですが、現在 Federation 層は**ハリボテ**です。
>
> **(A) Phase 4 で Federation import/sync を本実装する**
> - 工数: +1週。P2P Hub の完全な Karma 同期が可能になるが、v1.0.0 のスケジュールに影響する可能性。
>
> **(B) v1.0.0 はローカルノードのみの Reputation として設計する**
> - 工数: 変動なし。Federation は v1.1.0 以降にスコープ。ただし Samsara Hub の役割が限定的になる。
>
> **(C) Federation export のみを維持し、import は手動承認（管理コンソール経由）で代替する**
> - 工数: +2-3日。完全自動同期は後回しだが、管理者が手動でノード間の Karma を統合できる。

---

## Verification Plan

### Automated Tests
```bash
# Phase 3.5 完了後
cargo check --workspace --tests
cargo test --workspace
# Docker Conductor テスト修正後
cargo test --test docker_conductor_e2e_test -- --ignored
```

### Manual Verification
- アカウント削除 API の動作確認（`DELETE /api/v1/auth/delete`）
- Forecast API に不正な `series_id` を送信し、適切にバリデーションされることを確認
- Samsara Hub への Federation push が stub ログを出力することを確認
