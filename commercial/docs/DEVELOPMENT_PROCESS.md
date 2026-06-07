# 🔒 NURTURE 開発プロセス戦略 — 安定・安全・効率化

> **お金を扱うプラットフォームは、1つのバグが信用崩壊を招く。**
> このドキュメントは「絶対に事故を起こさない」ための、金融グレードの開発プロセスを定義する。
>
> 最終更新: 2026-03-14

---

## 目次

1. [Git ワークフロー（ブランチ戦略）](#1-git-ワークフロー)
2. [コミット規約](#2-コミット規約)
3. [CI/CD パイプライン](#3-cicd-パイプライン)
4. [テストピラミッド](#4-テストピラミッド)
5. [セキュリティ防衛線](#5-セキュリティ防衛線)
6. [コード品質ゲート](#6-コード品質ゲート)
7. [リリース管理](#7-リリース管理)
8. [インシデント対応](#8-インシデント対応)

---

## 1. Git ワークフロー

### ブランチ戦略: GitHub Flow + Release Branch

```
main ─────────────────────────────────── (常にデプロイ可能)
  │
  ├── feature/economy-refund ─────────── (機能開発)
  ├── feature/typestate-cancel ──────── (機能開発)
  ├── fix/daily-limit-overflow ──────── (バグ修正)
  ├── docs/legal-strategy ───────────── (ドキュメント)
  │
  └── release/v0.1.0 ────────────────── (リリース候補)
```

### ルール

| ルール | 内容 |
|--------|------|
| **main への直接 push 禁止** | すべての変更は PR 経由 |
| **PR マージ条件** | CI 全 Green + コードレビュー 1 名以上 |
| **ブランチ命名** | `feature/`, `fix/`, `docs/`, `refactor/`, `test/` + 短い説明 |
| **マージ戦略** | Squash merge（履歴をクリーンに保つ） |
| **ブランチ寿命** | 最大 1 週間（長期化したら分割） |

### GitHub Branch Protection (Recommended Settings)

To protect the `main` branch, configure the following in GitHub (Settings > Branches > Add rule):

- **Branch name pattern**: `main`
- **Require a pull request before merging**: Checked
  - **Required approvals**: 1
- **Require status checks to pass before merging**: Checked
  - `Compile Check`
  - `Unit Tests`
  - `Lint (Clippy)`
  - `Security Audit`
  - `BSL 1.1 Header Check`
- **Require conversation resolution before merging**: Checked
- **Enforce admins**: Checked (protects against accidental direct pushes by admins)

### Pre-commit Guardrails

A pre-commit script is provided to catch errors locally before they hit CI.

**To enable locally:**
```bash
ln -sf ../../scripts/pre_commit.sh .git/hooks/pre-commit
```

---

## 2. コミット規約

### Conventional Commits

```
<type>(<scope>): <description>

[body]

[footer]
```

| Type | 用途 | 例 |
|------|------|-----|
| `feat` | 新機能 | `feat(transaction): add Refunded typestate` |
| `fix` | バグ修正 | `fix(coin): prevent daily_limit overflow` |
| `test` | テスト追加 | `test(policy): add proptest for balance invariant` |
| `docs` | ドキュメント | `docs(legal): add AML policy` |
| `refactor` | リファクタリング | `refactor(ledger): extract entry validation` |
| `ci` | CI/CD 変更 | `ci: add cargo-audit to pipeline` |
| `chore` | その他 | `chore: update dependencies` |
| `security` | セキュリティ修正 | `security(auth): fix token validation` |

### Scope の一覧

`transaction`, `wallet`, `coin`, `points`, `ledger`, `policy`, `interceptor`, `settlement`, `mcp`, `auth`, `api`, `infra`, `tla`, `ci`, `legal`

---

## 3. CI/CD パイプライン

### GitHub Actions ワークフロー

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"
  PYO3_USE_ABI3_FORWARD_COMPATIBILITY: "1"

jobs:
  # ── Layer 1: 高速フィードバック（< 2分）──
  fmt:
    name: Format Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt }
      - run: cargo fmt --all -- --check

  check:
    name: Compile Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets

  clippy:
    name: Lint (Clippy)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  # ── Layer 2: テスト（< 5分）──
  test:
    name: Tests
    runs-on: ubuntu-latest
    needs: [check]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-targets
      - run: cargo test --workspace --doc   # ドキュメントテスト

  # ── Layer 3: セキュリティ監査（< 3分）──
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-deny --locked
      - run: cargo deny check                # ライセンス + 脆弱性
      - run: cargo install cargo-audit --locked
      - run: cargo audit                     # RustSec Advisory DB

  # ── Layer 4: ライセンスヘッダー検証 ──
  license-header:
    name: BSL 1.1 Header Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check BSL headers
        run: |
          MISSING=$(find . -name "*.rs" -not -path "*/target/*" \
            -exec grep -L "Business Source License 1.1" {} \;)
          if [ -n "$MISSING" ]; then
            echo "❌ Missing BSL 1.1 header in:"
            echo "$MISSING"
            exit 1
          fi
```

### パイプライン構成図

```
PR 作成
  │
  ├─ Layer 1 (並列, < 2分)
  │   ├── fmt ──────── コードフォーマット
  │   ├── check ────── コンパイル
  │   └── clippy ───── Lint
  │
  ├─ Layer 2 (check 依存, < 5分)
  │   └── test ─────── 単体 + proptest + ドキュメント
  │
  ├─ Layer 3 (並列, < 3分)
  │   └── audit ────── cargo-deny + cargo-audit
  │
  └─ Layer 4 (並列, < 1分)
      └── license ──── BSL 1.1 ヘッダー検証
```

> すべて Green → PR マージ可能

---

## 4. テストピラミッド

```
          ╱╲
         ╱  ╲        L5: TLA+ 形式検証（将来）
        ╱    ╲           → デッドロック、二重支払い不可能性
       ╱──────╲
      ╱        ╲     L4: AWS Kani / Z3（将来）
     ╱          ╲        → transfer() パニック不可能性
    ╱────────────╲
   ╱              ╲   L3: Proptest（プロパティベース）
  ╱                ╲      → 台帳の合計保存則、CRDT 可換性
 ╱──────────────────╲
╱                    ╲ L2: 統合テスト
╲                    ╱     → Crate 間の連携（API → Core → Protocol）
 ╲──────────────────╱
  ╲                ╱   L1: 単体テスト ← 【最優先で拡充】
   ╲──────────────╱        → 各モジュールの正常系・異常系
```

### L1: 単体テスト（即座に実装）

| モジュール | テスト項目 | 優先度 |
|-----------|-----------|:------:|
| `coin.rs` | charge/spend/daily_limit の正常系・異常系・境界値 | 🔴 |
| `policy.rs` | validate_transaction の全ポリシー条件 | 🔴 |
| `transaction.rs` | Typestate 遷移の正当性（コンパイル時安全性含む） | 🔴 |
| `interceptor.rs` | 暴走ストッパーのブロック条件 | 🔴 |
| `commodity.rs` | Serialize/Deserialize 往復 | 🟡 |
| `identity.rs` | ActorId 一意性 | 🟡 |
| `settlement.rs` | SettlementReceipt 生成 | 🟡 |
| `points.rs` | earn/withdraw の正常系 | 🟡 |
| `ledger.rs` | LedgerEntry Serialize + 合計保存則 | 🟡 |

### L3: Proptest（Phase 1 Week 5-6 で導入）

```rust
// Cargo.toml に追加
[dev-dependencies]
proptest = "1.4"

// 例: 台帳の合計保存則
proptest! {
    #[test]
    fn balance_conservation(
        charge_amount in 1u64..10000,
        spend_amount in 1u64..10000,
    ) {
        let mut wallet = CoinWallet::new(actor_id, 50000);
        wallet.charge(charge_amount).unwrap();

        if spend_amount <= charge_amount {
            wallet.spend(spend_amount).unwrap();
            prop_assert_eq!(
                wallet.balance(),
                charge_amount - spend_amount
            );
        }
    }
}
```

---

## 5. セキュリティ防衛線

### 5層の防御

| 層 | 対策 | ツール | タイミング |
|:--:|------|-------|:----------:|
| L1 | **依存関係の脆弱性スキャン** | `cargo-audit` | CI 毎回 |
| L2 | **ライセンス監査** | `cargo-deny` | CI 毎回 |
| L3 | **シークレット検出** | `gitleaks` | pre-commit |
| L4 | **unsafe コード禁止** | Clippy + `#![forbid(unsafe_code)]` | CI 毎回 |
| L5 | **依存関係の定期更新** | Dependabot | 週次自動 PR |

### pre-commit フック

```bash
#!/bin/bash
# .git/hooks/pre-commit

# 1. シークレットの漏洩チェック
if command -v gitleaks &> /dev/null; then
    gitleaks detect --source . --verbose
    if [ $? -ne 0 ]; then
        echo "❌ シークレットが検出されました！"
        exit 1
    fi
fi

# 2. フォーマットチェック
cargo fmt --all -- --check
if [ $? -ne 0 ]; then
    echo "❌ cargo fmt を実行してください"
    exit 1
fi

# 3. BSL ヘッダーチェック
MISSING=$(find . -name "*.rs" -not -path "*/target/*" \
  -exec grep -L "Business Source License 1.1" {} \;)
if [ -n "$MISSING" ]; then
    echo "❌ BSL 1.1 ヘッダーが不足: $MISSING"
    exit 1
fi
```

### Dependabot 設定

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    reviewers:
      - "for4xex-droid"
    labels:
      - "dependencies"
    open-pull-requests-limit: 5
```

### Crate 全体で unsafe を禁止

```rust
// 各 Crate の lib.rs 先頭に追加
#![forbid(unsafe_code)]
```

---

## 6. コード品質ゲート

### PR マージの必須条件

| ゲート | 基準 | 自動/手動 |
|--------|------|:---------:|
| `cargo fmt` | フォーマット差分ゼロ | 自動 |
| `cargo clippy` | 警告ゼロ（`-D warnings`） | 自動 |
| `cargo test` | 全テスト Green | 自動 |
| `cargo deny check` | ライセンス違反ゼロ | 自動 |
| `cargo audit` | 重大脆弱性ゼロ | 自動 |
| BSL ヘッダー | 全 `.rs` ファイルに存在 | 自動 |
| コードレビュー | 1 名以上の Approve | 手動 |

### Clippy の追加ルール（金融グレード）

```toml
# clippy.toml または Cargo.toml の [workspace.metadata.clippy]
# 以下を .cargo/config.toml に追加

[target.'cfg(all())']
rustflags = [
    "-W", "clippy::unwrap_used",         # unwrap() を警告
    "-W", "clippy::expect_used",         # expect() を警告
    "-W", "clippy::panic",               # panic!() を警告
    "-W", "clippy::integer_arithmetic",  # 整数オーバーフロー注意喚起
]
```

### ドキュメント品質

```toml
# 各 Crate の lib.rs に追加
#![warn(missing_docs)]     # pub API にドキュメント必須
#![warn(rustdoc::all)]     # ドキュメントの整合性チェック
```

---

## 7. リリース管理

### セマンティックバージョニング

```
v{MAJOR}.{MINOR}.{PATCH}

MAJOR: 破壊的変更（API 互換性なし）
MINOR: 後方互換の新機能
PATCH: バグ修正
```

### リリースフロー

```
1. main から release/vX.Y.Z ブランチを切る
2. バージョン番号を Cargo.toml で更新
3. CHANGELOG.md を更新
4. CI が全 Green を確認
5. Git tag を打つ（v0.1.0）
6. main にマージ
7. GitHub Release を作成
```

### CHANGELOG 管理

```markdown
# Changelog

## [Unreleased]

### Added
- Typestate パターンによる Transaction の型安全な状態遷移
- 法務・税務・規制戦略ドキュメント

### Changed
- `validate_transaction` をジェネリック化（全 TxState に対応）

### Fixed
- (なし)

### Security
- (なし)
```

---

## 8. インシデント対応

### 重大度レベル

| レベル | 定義 | 対応時間 |
|:------:|------|:--------:|
| **P0** | 金銭的損害が発生 / データ漏洩 | 即時（1時間以内） |
| **P1** | 決済機能停止 / セキュリティ脆弱性 | 4 時間以内 |
| **P2** | 機能の一部が使用不可 | 24 時間以内 |
| **P3** | 表示崩れ / パフォーマンス劣化 | 次のスプリント |

### 対応フロー

```
検知 → 初期分析（影響範囲特定）→ 対策実施 → 検証 → ポストモーテム（振り返り）
```

### ポストモーテム テンプレート

```markdown
## インシデントレポート: [タイトル]
- **日時**: YYYY-MM-DD HH:MM
- **重大度**: P0 / P1 / P2 / P3
- **影響範囲**: [ユーザー数、金額等]
- **根本原因**: [技術的原因]
- **対策**: [実施した修正]
- **再発防止策**: [CI追加、テスト追加、設計変更等]
```

---

## 現状からの導入ロードマップ

| 優先度 | タスク | 期限 |
|:------:|--------|:----:|
| 🔴 即時 | GitHub Branch Protection を設定 | 今日 |
| 🔴 即時 | `.github/workflows/ci.yml` を作成 | 今日 |
| 🔴 即時 | `#![forbid(unsafe_code)]` を全 Crate に追加 | 今日 |
| 🟡 今週 | pre-commit フック設定 | 3日以内 |
| 🟡 今週 | Dependabot 設定 | 3日以内 |
| 🟡 今週 | Conventional Commits の運用開始 | 3日以内 |
| 🟢 来週 | CHANGELOG.md の作成 | 1週間 |
| 🟢 来週 | L1 単体テストの完全実装 | 1週間 |
| 🔵 将来 | Proptest（L3）導入 | Phase 1 後半 |
| 🔵 将来 | TLA+ CI 統合 | Phase 3 以降 |

---

## 9. Aiome OSS との並行開発

### リポジトリ構成

```
antigravity/
├── modular-open-claw/     ← Aiome OSS（GitHub Public）
└── Project-Nurture/      ← 商用拡張（GitHub Private）
```

**依存方向**: `NURTURE → OSS`（一方向のみ。絶対に逆依存を作らない）

### OSS 変更時の連鎖ビルド確認

> ⚠️ **最重要ルール**: OSS を触ったら、必ず NURTURE も `cargo check` する。
> OSS の型やトレイト変更が NURTURE をサイレントに壊す危険性がある。

```bash
# 両リポ横断ビルド確認（最も重要なコマンド）
cd ../../
cargo check --workspace && \
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo check \
  --manifest-path ../Project-Nurture/Cargo.toml --workspace && \
echo "✅ 両リポ正常"
```

### GitHub CI 連鎖

OSS の CI 成功時に NURTURE の CI を自動起動する。

**OSS 側** (`.github/workflows/ci.yml` に追加):
```yaml
- name: Trigger NURTURE CI
  if: success()
  uses: peter-evans/repository-dispatch@v3
  with:
    token: ${{ secrets.NURTURE_REPO_TOKEN }}
    repository: motivationstudio-llc/project-nurture
    event-type: oss-updated
```

**NURTURE 側** (`.github/workflows/ci.yml` に追加):
```yaml
on:
  repository_dispatch:
    types: [ oss-updated ]
```

### Antigravity（AI アシスタント）の運用

| 目的 | 方法 |
|------|------|
| NURTURE のコードを読む/書く | 絶対パスで指定 |
| NURTURE の `cargo check` | `cargo check --manifest-path ../Project-Nurture/Cargo.toml` |
| 進捗追跡 | `DEVELOPMENT_GUIDE.md` のモック一覧を毎セッション確認 |
| コンテキスト復元 | `SOUL.md`, `MEMORY.md`, `memory/` を参照 |

### MCP サーバ活用

| ツール | 活用場面 |
|--------|---------|
| `scout_web` | Stripe API 仕様変更、PyO3 最新バージョン、法規制の動向調査 |
| `read_web_content` | Stripe / Tremendous ドキュメントの深掘り |
| `grep_local_code` | **2リポ横断検索**（例: `NurtureError` の使用箇所を両リポ一括検索） |
| `ask_local_brain` | 設計判断の壁打ち |

### 日常チェックリスト

```
セッション開始
  ├── ① DEVELOPMENT_GUIDE.md のモック一覧を確認
  ├── ② 両リポの git status 確認
  ├── ③ NURTURE の cargo check 確認
  └── ④ Phase X / Week Y の作業対象を決定

セッション終了
  ├── ① git commit + push（両リポ）
  ├── ② モック一覧を最新化
  └── ③ memory/ に作業ログを記録
```
