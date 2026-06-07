# 🏗️ Project NURTURE — 環境構築計画（完全版）

> **対象**: `../Project-Nurture` (商用拡張リポジトリ)  
> **依存先**: `../aiome` (Aiome OSSリポジトリ)  
> **策定日**: 2026-03-14

---

## スキャン結果: 旧版で欠落していた14カテゴリ

Aiome プロジェクトの全62ファイル/24ディレクトリをスキャンした結果:

| # | カテゴリ | Aiome での実態 | 旧計画の状態 |
|---|--------|-------------|------------|
| 1 | CI/CD ワークフロー | `ci.yml`(5ジョブ) + `formal-verify.yml` | ❌ 完全欠落 |
| 2 | `cargo-deny` (依存監査) | `deny.toml` 253行 | ❌ 完全欠落 |
| 3 | ライセンスヘッダースクリプト | `apply_license.sh` (AGPL→**BSL 1.1に変更必要**) | ❌ 完全欠落 |
| 4 | CLA (再ライセンス同意書) | `CLA.md` 61行 | ❌ 完全欠落 |
| 5 | CONTRIBUTING ガイド | `CONTRIBUTING.md` | ❌ 完全欠落 |
| 6 | PR テンプレート | CLA署名チェック付き | ❌ 完全欠落 |
| 7 | ISSUE テンプレート | bug_report + feature_request | ❌ 完全欠落 |
| 8 | OpenAPI 仕様 | `docs/openapi.json` 1490行 | ❌ 完全欠落 |
| 9 | TLA+ 形式検証基盤 | 4 spec + `tla2tools.jar` + CI | ❌ 完全欠落 |
| 10 | Dockerfile | rootless 強化済み | ❌ 完全欠落 |
| 11 | `.env.example` | 9カテゴリ79行 | △ 概要のみ |
| 12 | テストスクリプト | swarm/bft/load テスト | ❌ 完全欠落 |
| 13 | SECURITY.md | 脆弱性報告ポリシー | ❌ 完全欠落 |
| 14 | README (日英2版) | 15KB + 11KB | ❌ 完全欠落 |

---

## 完全版ディレクトリ構造

```
Project-Nurture/
├── Cargo.toml                     ← Step 2: ワークスペース定義
├── LICENSE                         ← Step 3: BSL 1.1
├── CLA.md                          ← Step 4: CLA (BSL 1.1 対応版)
├── README.md                       ← Step 14: プロジェクト概要
├── CONTRIBUTING.md                 ← Step 5: コントリビューションガイド
├── SECURITY.md                     ← Step 13: 脆弱性報告ポリシー
├── CODE_OF_CONDUCT.md              ← Step 5: 行動規範
├── .gitignore                      ← Step 3: Rust + DB + .env
├── .env.example                    ← Step 11: NURTURE 固有環境変数
├── deny.toml                       ← Step 6: cargo-deny (BSL-1.1 追加)
│
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                  ← Step 7: Rust CI (5ジョブ)
│   │   └── formal-verify.yml       ← Step 7: TLA+ 形式検証
│   ├── pull_request_template.md    ← Step 8: PR テンプレート
│   └── ISSUE_TEMPLATE/
│       ├── bug_report.md           ← Step 8: バグレポート
│       └── feature_request.md      ← Step 8: 機能リクエスト
│
├── libs/
│   ├── commerce-protocol/          ← Crate 0
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── identity.rs
│   │       ├── commodity.rs
│   │       ├── transaction.rs
│   │       ├── offer.rs
│   │       ├── settlement.rs
│   │       ├── reputation.rs
│   │       └── mcp_commerce.rs
│   ├── nurture-core/               ← Crate 1
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── coin.rs
│   │       ├── points.rs
│   │       ├── ledger.rs
│   │       ├── policy.rs
│   │       ├── a2a/mod.rs
│   │       ├── a2c/mod.rs
│   │       └── b2a/mod.rs
│   └── nurture-infra/              ← Crate 2
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── economy/mod.rs
│           ├── marketplace/mod.rs
│           ├── drm/mod.rs
│           ├── csam/mod.rs
│           ├── gift/mod.rs
│           ├── stripe/mod.rs
│           ├── sandbox/mod.rs      ← [G4]
│           ├── sidecar/mod.rs      ← [G5]
│           ├── a2a/mod.rs
│           ├── b2a/mod.rs
│           └── protocol/mod.rs
│
├── apps/
│   ├── nurture-api/                ← Crate 3
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── marketplace.rs
│   │       │   └── wallet.rs           ← [機能追加] Wallet API ハンドラ
│   │       ├── mcp_tools/
│   │       │   ├── mod.rs
│   │       │   ├── search.rs
│   │       │   ├── buy.rs
│   │       │   ├── wallet.rs           ← [機能追加] Wallet 情報取得ツール
│   │       │   ├── gift.rs             ← 空モジュール
│   │       │   └── sandbox_exec.rs     ← 空モジュール
│   │       └── auth/mod.rs         ← [G9]
│   └── nurture-ui/                 ← Crate 4
│       └── package.json            ← ✅ Vite + React 19 構築済み
│
├── specs/
│   ├── NurtureEconomyProtocol.tla  ← Step 9: TLA+ 初期仕様
│   └── NurtureEconomyProtocol.cfg
│
├── assets/
│   └── base-body/                  ← [G6] 公式素体
│       └── README.md
│
├── docker/
│   └── production.Dockerfile       ← Step 10: Rootless Dockerfile
│
├── scripts/
│   ├── apply_license.sh            ← Step 3: BSL 1.1 ヘッダー適用
│   └── test_all.sh                 ← Step 12: テスト実行
│
├── tests/
│   └── (後日: E2E / 負荷テスト)
│
└── docs/
    ├── nurture_commercial_separation.md  ← v3 計画書コピー
    └── openapi_nurture.json              ← Step 9: NURTURE API 仕様
```

---

## 実行手順の詳細

### Step 1: ディレクトリ構造の一括作成

`mkdir -p` で全ディレクトリを作成。

---

### Step 2: ワークスペース `Cargo.toml`

Aiome の `Cargo.toml` から共通依存を流用しつつ、NURTURE 固有の依存を追加:

```toml
[workspace]
members = [
    "libs/commerce-protocol",
    "libs/nurture-core",
    "libs/nurture-infra",
    "apps/nurture-api",
]
resolver = "2"

[workspace.dependencies]
# === Aiome 共通 (流用) ===
tokio = { version = "1.35", features = ["full"] }
axum = { version = "0.7", features = ["ws", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
reqwest = { version = "0.12", features = ["json", "stream"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
chacha20poly1305 = "0.10"
ed25519-dalek = "2.1"
base64 = "0.22"
rand = "0.8"
once_cell = "1.18"
futures = "0.3"
tokio-stream = "0.1"
tower = { version = "0.4", features = ["limit"] }
utoipa = "5"

# === NURTURE 固有 ===
pyo3 = { version = "0.23", features = ["auto-initialize"] }
gltf = "1.4"
oauth2 = "4.4"
jsonwebtoken = "9.2"
async-stripe = "0.41"

[workspace.lints.clippy]
unwrap_used = "warn"
```

---

### Step 3: BSL 1.1 ライセンス + ヘッダースクリプト + .gitignore

**LICENSE** — BSL 1.1 全文（Change Date: 2030-04-01 → Apache 2.0 へ自動転換）

**scripts/apply_license.sh** — Aiome の `apply_license.sh` をベースに BSL 1.1 ヘッダーへ変更:

```
Business Source License 1.1
Copyright (C) 2026 motivationstudio, LLC
Licensed under BSL 1.1. Change Date: 2030-04-01
Change License: Apache License 2.0
```

**.gitignore** — Aiome の `.gitignore` を流用（target/, .env, *.db, *.log 等）

---

### Step 4: CLA（BSL 1.1 対応版）

Aiome の `CLA.md` をベースに、§2 再ライセンス条項を BSL 1.1 に対応:

> 「BSL 1.1 および将来のライセンス変更を含む異なるライセンス条項の下で配布できる」

---

### Step 5: CONTRIBUTING.md + CODE_OF_CONDUCT.md

Aiome のものをベースに、NURTURE 固有のルール（商用コード取り扱い注意等）を追記。

---

### Step 6: `deny.toml`（cargo-deny 設定）

Aiome の `deny.toml` を流用し、`licenses.allow` に **`"BUSL-1.1"` を追加**:

```toml
allow = [
    "MIT", "Apache-2.0", ...,
    "BUSL-1.1",    # ← NURTURE 商用ライセンス
]
```

---

### Step 7: CI/CD ワークフロー

Aiome の `ci.yml` を流用し、以下を調整:

| ジョブ | 調整内容 |
|--------|---------|
| **Test & Lint** | ワークスペースパスを NURTURE に変更 |
| **Frontend** | nurture-ui 完成後に有効化 |
| **E2E** | 後日（Phase 2 以降） |
| **Audit** | `cargo deny check` をそのまま流用 |
| **Formal Verify** | `NurtureEconomyProtocol.tla` を対象に |

`formal-verify.yml` — TLA+ 経済プロトコルの自動検証:

```yaml
- name: Run TLC (NurtureEconomyProtocol)
  run: java -cp tla2tools.jar tlc2.TLC specs/NurtureEconomyProtocol.tla ...
```

---

### Step 8: GitHub テンプレート (PR + ISSUE)

Aiome のテンプレートを流用し、チェックリストに NURTURE 固有項目を追加:

```markdown
- [ ] BSL 1.1 ライセンスヘッダーが付与されている
- [ ] 商用コードが OSS 側に混入していない
- [ ] `cargo deny check license` が通る
```

---

### Step 9: TLA+ 初期仕様 + OpenAPI 仕様

**TLA+ 仕様**: 二重通貨のアトミック取引を形式検証する最小仕様ファイル。Aiome の `SamsaraKarmaProtocol.tla` のパターンを流用。

**OpenAPI**: NURTURE 商用 API（wallet, marketplace, merchant, gift）のエンドポイント定義。Aiome の `openapi.json` のスキーマ構造を流用。
※ `docs/openapi_nurture.json` にWallet関連（balance, points, history）のエンドポイントが含まれることを確認。

---

### Step 10: Dockerfile

Aiome の `production.Dockerfile` を流用:
- ビルドターゲットを `nurture-api` に変更
- rootless + read-only 設定維持
- ポートを `3020` に変更（Aiome の `3015` と競合回避）

---

### Step 11: `.env.example`

NURTURE 固有の環境変数:

```bash
# --- NURTURE Commercial Configuration ---
NURTURE_DB_PATH=sqlite://data/nurture.db
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
TREMENDOUS_API_KEY=...
TREMENDOUS_CAMPAIGN_ID=...

# Aiome OSS 連携
AIOME_API_URL=http://localhost:3015
AIOME_API_SECRET=...

# OAuth 2.1 (MCP Commerce)
OAUTH_CLIENT_ID=...
OAUTH_CLIENT_SECRET=...
OAUTH_ISSUER_URL=...
```

---

### Step 12: テストスクリプト

`scripts/test_all.sh` — 全 Crate テスト + cargo-deny + clippy:

```bash
#!/bin/bash
set -e
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo deny check
```

---

### Step 13: SECURITY.md

Aiome の `SECURITY.md` をベースに NURTURE 固有の脆弱性報告先と対応ポリシーを明記。

---

### Step 14: README.md

プロジェクト概要、アーキテクチャ図、Getting Started、ライセンス表記。

---

## 各 Crate の Cargo.toml

### commerce-protocol

```toml
[package]
name = "commerce-protocol"
version = "0.1.0"
edition = "2021"
license = "BUSL-1.1"
publish = false

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
async-trait = { workspace = true }
```

### nurture-core

```toml
[package]
name = "nurture-core"
version = "0.1.0"
edition = "2021"
license = "BUSL-1.1"
publish = false

[dependencies]
commerce-protocol = { path = "../commerce-protocol" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
```

### nurture-infra

```toml
[package]
name = "nurture-infra"
version = "0.1.0"
edition = "2021"
license = "BUSL-1.1"
publish = false

[dependencies]
commerce-protocol = { path = "../commerce-protocol" }
nurture-core = { path = "../nurture-core" }
aiome-core = { path = "../../../modular-open-claw/libs/core" }
sqlx = { workspace = true }
chacha20poly1305 = { workspace = true }
reqwest = { workspace = true }
pyo3 = { workspace = true }
gltf = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
nix = "0.29"
async-stripe = { workspace = true }
sha2 = { workspace = true }
rand = { workspace = true }
```

### nurture-api

```toml
[package]
name = "nurture-api"
version = "0.1.0"
edition = "2021"
license = "BUSL-1.1"
publish = false

[dependencies]
commerce-protocol = { path = "../../libs/commerce-protocol" }
nurture-core = { path = "../../libs/nurture-core" }
nurture-infra = { path = "../../libs/nurture-infra" }
aiome-core = { path = "../../../modular-open-claw/libs/core" }
axum = { workspace = true }
oauth2 = { workspace = true }
jsonwebtoken = { workspace = true }
tower = { workspace = true }
utoipa = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
uuid = { workspace = true }
```

---

## Aiome コード流用マトリクス

| Aiome ファイル | 流用先 | 流用方法 |
|-------------|-------|---------|
| `libs/core/error.rs` | `nurture-core` | `AiomeError` をそのまま `use`（path依存）|
| `libs/core/traits.rs` | `nurture-core` | トレイト設計パターンを参考に `CommerceEngine` を設計 |
| `libs/core/budget.rs` | `nurture-core/coin.rs` | AtomicU64 パターンを参考に通貨操作を実装 |
| `libs/infrastructure/security.rs` | `nurture-infra` | BastionGuard パターンを参考にサンドボックスポリシーを設計 |
| `specs/*.tla` | `specs/` | TLA+ 仕様のテンプレート構造を参考 |
| `.github/workflows/ci.yml` | `.github/workflows/` | CI ジョブ構成を流用・調整 |
| `deny.toml` | `deny.toml` | ライセンス許可リスト + BSL-1.1 追加 |
| `CLA.md` | `CLA.md` | 再ライセンス条項を BSL 1.1 に変更 |
| `.gitignore` | `.gitignore` | そのまま流用 |
| `docker/production.Dockerfile` | `docker/` | ビルドターゲット変更のみ |

---

## 実行計画サマリ

| Step | 内容 | 推定時間 |
|------|------|---------|
| 1 | ディレクトリ構造一括作成 | 1分 |
| 2 | ワークスペース Cargo.toml | 1分 |
| 3 | LICENSE + .gitignore + ヘッダースクリプト | 2分 |
| 4 | CLA.md (BSL 1.1版) | 1分 |
| 5 | CONTRIBUTING.md + CODE_OF_CONDUCT.md | 2分 |
| 6 | deny.toml | 1分 |
| 7 | CI/CD ワークフロー (2ファイル) | 3分 |
| 8 | GitHub テンプレート (PR + ISSUE) | 2分 |
| 9 | TLA+ 初期仕様 + OpenAPI 雛形 | 3分 |
| 10 | Dockerfile | 1分 |
| 11 | .env.example | 1分 |
| 12 | テストスクリプト | 1分 |
| 13 | SECURITY.md | 1分 |
| 14 | README.md | 2分 |
| 15 | 各 Crate の Cargo.toml + lib.rs | 5分 |
| 16 | `cargo check --workspace` 検証 | 2分 |
| 17 | git init + 初回コミット | 1分 |
| **合計** | | **約30分** |
