# Agentic 本番硬化計画（v1.3・Wave D 本番クローズ 2026-07-19）

- **ステータス**: Wave A+B+D ✅ / 本番 key-proxy 再ビルド・B1 telemetry 反映済 / A1 本番 Unauthenticated 0。**Wave C**: **OP-051 ✅** / **OP-083 ✅** / **OP-011 ✅ 2026-07-22**
- **目的**: Human 後回しで、エージェントがコードから本番稼働の確実性を上げる
- **正本 ID**: OPEN **OP-086**
- **継承**: `billing_closeout_plan` v1.5（R4 済）/ OPEN / R4 本番ログ
- **原則**: Safety-Critical（auth/commerce/Vault 暗号・Tauri）の**ロジック改変はしない**。auth ログの構造化・compose volume は OP-086 範囲。Verification Protocol 必須

## 0. レーン分離

| レーン | 対象 | 本計画 |
|---|---|---|
| **Agent** | コード・compose・テスト・台帳 | **実行対象** |
| **Human** | OP-064 / NT-7 / Vault 鍵操作 / テスティモニアル | **後回し** |
| **Blocked** | OP-020 製品 P2P 残（任意）・OP-030〜034 Upstream | OP-011 ✅。OP-083-C/D は Q2+SC（ADR-053 非ブロッカー） |

## 1. コード根拠（事実）

計画着手時のギャップ（F1–F4）は Wave A で解消済み。下表は **2026-07-19 時点の現行**。

| ID | 事実（現行） | 根拠 |
|---|---|---|
| F1 | ✅ api-server に `A2A_AUTH_TOKEN` / `A2A_NODE_TOKEN`（同値）を注入 | `docker-compose.production.yml`（api-server environment） |
| F2 | ✅ poller / FormalProofGate は `a2a_grpc_auth_token()`（AUTH→NODE）を使用 | `oxilean_poller.rs` / `shared::config` |
| F3 | ✅ AUTH / NODE / 空 `AIOME_DB_PATH` を unset 扱い | `libs/shared/src/config.rs` + `a2a_` 単体 |
| F4 | ✅ key-proxy healthcheck は `curl -f …/api/v1/health` | compose key-proxy healthcheck |
| F5 | main compose は Postgres 前提。本番ホストは SQLite（overlay / hotfix 手順） | `docker-compose.production.sqlite.yml` / sync スクリプト |
| F6 | nurture-api Postgres 依存は sqlite overlay / profile で制御 | `production.sqlite.yml` |
| F7 | OP-083-C/D = **Q2 + SC**（[`op083_cd_x402_plan.md`](op083_cd_x402_plan.md)）。ADR-053 transport 済みは非ブロッカー。OP-051 ✅ | op083_cd / ADR-054 |
| F8 | ✅ key-proxy Vault は `./data/key-proxy` volume + `ABYSS_VAULT_PATH` | compose + `.gitignore` |
| F9 | ✅ B1 telemetry 本番反映（key-proxy イメージ再ビルド + recreate） | OPEN OP-086 Wave D |

## 2. 優先度 Wave（Agent のみ・上から実行）

### Wave A — 本番確実性（最優先）

| ID | 作業 | サイズ | 検証 |
|---|---|---|---|
| **A1** | ✅ コード: compose に `A2A_AUTH_TOKEN` / AUTH 空文字 filter / `a2a_grpc_auth_token()` フォールバック（poller + FormalProofGate）/ 単体4本 PASS | S | ✅ 本番: poller `Unauthenticated` 0（Wave D 2026-07-19） |
| **A2** | ✅ コード: key-proxy healthcheck → `curl -f …/api/v1/health` | S | ✅ 本番: healthy（Wave D 2026-07-19） |
| **A3** | ✅ `docker-compose.production.sqlite.yml` + `scripts/sync_production_sources.sh`（compose 既定スキップ） | M | `compose config --services` に postgres が出ない |
| **A4** | ✅ sqlite overlay で nurture/samsara を SQLite URL + postgres depends 除去 | M | nurture Restarting 解消（ホスト適用後） |

### Wave B — 運用可観測性・低リスク硬化

| ID | 作業 | サイズ | 検証 |
|---|---|---|---|
| **B1** | ✅ OP-025: `telemetry.rs`（sanitize + metrics）/ span `caller_id` / auth 401 構造化（秘密非出力）+ Negative 単体 | M | ✅ `cargo test -p key-proxy` 34 PASS + 本番 401 スモーク（Wave D） |
| **B2** | ✅ OP-082: `docker-compose.quickstart.native-ollama.yml` に `extra_hosts: host.docker.internal:host-gateway` | S | compose 構文 OK |
| **⚠ 本番教訓** | ✅ compose `./data/key-proxy` volume + recreate 警告 + `CELL_ID`。健全時 `--status-only`、wipe 時のみ restore PUT | — | Vault `STRIPE_API_KEY` + `/health` 200 |
| **B3** | ✅ OP-023: ホットパス棚卸し（grpc/security/llm/vault/key-proxy/api bootstrap/internal/shadow）。`enforce_unwrap_deny.py` → **0 violations**。テスト内 unwrap は非対象 | L | スクリプト PASS。本番置換なし（既にクリーン） |

### Wave D — 本番クローズ（2026-07-19）

| ID | 作業 | 検証 |
|---|---|---|
| **D0** | ✅ Preflight: volume + Vault DB バックアップ + baseline healthy | vault_db 12288 bytes、backup 済 |
| **D1** | ✅ `sync_production_sources.sh` + `SYNC_COMPOSE=1` | allowlist + compose 同期 |
| **D2** | ✅ key-proxy `build` + `up -d --force-recreate --no-deps --no-build`（`-f docker-compose.production.yml` のみ。sqlite overlay は samsara→postgres 依存で key-proxy 単体 recreate に不要） | healthy、新イメージ適用 |
| **⚠ 教訓** | ✅ recreate 後 `CELL_ID` 必須 → compose key-proxy に `CELL_ID=${CELL_ID:-cell-0}` 追加 | クラッシュループ解消 |
| **D3** | ✅ Vault 整合: configured=17/18、`stripe_set=True`、DB サイズ維持 | `restore_vault_from_env.py --status-only`（健全時 PUT 禁止） |
| **D4** | ✅ A1 本番: api-server 30m ログ `Unauthenticated` **0** | — |
| **D5** | ✅ B1 スモーク: 不正 Bearer → 401 + 構造化 WARN（秘密非出力） | — |

### Wave C — ゲート待ち（計画に残すだけ）

| ID | ゲート | 着手条件 |
|---|---|---|
| C1 OP-051 | ✅ ADR-054 Accepted + **P1–P4 2026-07-20** | [`op051_error_hierarchy_plan.md`](op051_error_hierarchy_plan.md) |
| C2 OP-083-C/D | ✅ Q2 + C/D **2026-07-20** | [`op083_cd_x402_plan.md`](op083_cd_x402_plan.md) |
| C3 OP-011 | ✅ 2026-07-22（KC マーケット S2S + wiremock P/N。有償 KC 非対象） | OPEN / KC_LEGAL |
| C3' OP-020 製品 P2P 残 | 任意 polish | 本計画外（2b/4 / F5 ✅） |

## 3. 成功基準

1. Wave A 完了後、本番で OxiLean poller が Unauthenticated 連発しない（A1）
2. key-proxy が compose 上 `healthy`（A2）
3. main compose の Postgres 前提を SQLite ホストへ無差別適用しても、override/手順で再発防止できる（A3）
4. nurture-api の無意味な Restarting が解消または意図的停止（A4）
5. Human（OP-064 等）をブロッカーにしない
6. Safety-Critical パス（commerce/Vault/auth 本体/Tauri shell）を変更しない

## 4. /perfect-plan 第1周

| Gate | 結果 |
|---|---|
| 1 構造 | PASS — 新規クレートなし。compose + config + poller + healthcheck のみ |
| 2 NURTURE | PASS — 経済/認証コア非変更。A1 は既存 A2A 配線の穴埋め |
| 3 波及 | PASS — `a2a_auth_token` 参照は poller/core_services。Mock 影響なし |
| 4 Red Team | 最悪=AUTH を二重定義して shadow と不一致 → 同一 `${A2A_AUTH_TOKEN}` で固定。やらない=Federation 前の C/D |
| 5 順序 | PASS — **A1→A2→A3→A4→B***。C はゲート後 |

**判定: ✅ PASS（v1.0 を実行正本とする）**

## 5. 明示的後回し（Human / 非エージェント）

- OP-064 / NT-7 ベータ獲得
- Vault 鍵ローテ・Stripe Dashboard
- プロモ動画（OP-040）
- 本番 Postgres 本移行（別プロジェクト扱い）
