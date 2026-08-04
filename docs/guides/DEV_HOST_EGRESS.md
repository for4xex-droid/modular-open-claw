# 開発ホスト Egress 衛生（macOS）

**最終更新: 2026-08-01**  
**正本計画**: [`dev_host_egress_hygiene_plan.md`](../roadmaps/dev_host_egress_hygiene_plan.md) **v1.2**（OP-095）  
**対象**: Vault / Stripe Live / 本番操作を行う **macOS 開発機**  
**非対象**: 製品ランタイムの SSRF・Immune・MCP・Bastion / seatbelt（それらは [`SECURITY_DESIGN.md`](../architecture/SECURITY_DESIGN.md) と計画 §1.2。本ガイドに転記しない）

> **本線防衛は OP-096**（製品内 `host_permitted` / Bastion Fail-Closed。正本: [`autonomous_egress_defense_plan.md`](../roadmaps/autonomous_egress_defense_plan.md)）。本ガイドの LuLu/Little Snitch（H1）は **任意の個人衛生**であり、OP-095 クローズの必須条件ではない。

---

## 1. なぜやるか

製品内の制御は既にある（OP-096）。本ガイドは **ホスト上の未知プロセスの外向き**を人間が見えるようにするだけで、サプライチェーン対策や Vault の代替ではない。

---

## 2. 導入（H1）

| 順 | 作業 | 完了条件 |
|---|---|---|
| 1 | [LuLu](https://objective-see.org/products/lulu.html)（無料・最低線）または [Little Snitch](https://www.obdev.at/products/littlesnitch/) をインストール | アプリが起動する |
| 2 | 既知の開発ツールはカテゴリ Allow、**未知バイナリは Ask** | ルールを固定した |
| 3 | 初回セッションで「驚いた外向き」をメモ（プロセス名 + ホスト名のみ） | 秘密値・API キーは書かない |

### 許可疲れプロトコル

連続で Allow しそうになったら **10 分止める**。同じカテゴリならアプリ単位の許可へ昇格し、見知らぬバイナリは Ask のままにする。

### よく出るプロセス（初回学習の目安）

Docker Desktop / Colima、Cursor（または IDE）、`cargo` / `rustup`、`node` / `npm`、ブラウザ、`ollama`（Local LLM 利用時）。

### allowlist カテゴリ（ホスト名の例・IP 固定禁止）

| カテゴリ | 例 | 既定 |
|---|---|---|
| SCM | `github.com`, `api.github.com` | Allow |
| Rust | `crates.io`, `static.crates.io`, `index.crates.io` | Allow |
| JS | `registry.npmjs.org` | 使用時 Allow |
| コンテナ | Docker Hub / ghcr.io、Docker Desktop | 開発時 Allow |
| 決済（Human） | `api.stripe.com`, Stripe Dashboard | 秘密作業時 |
| LLM | 利用プロバイダの公式 API ホストのみ | 設定時 |
| ローカル | `localhost`, `host.docker.internal`, Ollama | Allow |
| 未知バイナリ | 上記外 | **Ask** |

---

## 3. 検証（必須・Verification Protocol）

### Positive

次が **恒常ブロックされず**完了すること。

- `cargo fetch`（または普段使う同等）
- ローカル `api-server` 起動、または `docker compose` 開発起動
- 使う場合のみ `gh`

### Negative

未知バイナリ相当（一時的な未許可アプリでも可）を起動し、**プロンプトまたは拒否**が出ることを確認する。

### Revert

Negative 用に足した一時ルールを削除し、通常プロファイルに戻す。

---

## 4. 危険作業の前（1 チェック）

Vault / Live 鍵 / 本番反映の前に:

- [ ] ホスト outbound 監視が ON（LuLu または Little Snitch）

入口（リマインダのみ・手順の正本は各ファイル）:

- ランブック NT-1 Step A 直前: [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](HUMAN_PUBLIC_BETA_RUNBOOK.md)
- Stripe 本番チェック: [`stripe-production-setup.md`](../operations/stripe-production-setup.md)

**禁止**: ルール名・スクショ・Issue・チャットに `sk_` / `whsec_` / Vault 値 / パスワードを貼ること。

---

## 5. 製品側との境界（混同防止・1 文）

製品の URL 検証は系統が分かれる（例: `SecurityPolicy::validate_url` と workflow の `assert_resolved_url_safe`）。詳細・アンカーは計画 §1.2。本ガイドで仕様を複製しない。macOS seatbelt / BastionGuard のプロファイルもここに書かない。

---

## 6. やらないこと

- 製品への LuLu / Little Snitch 同梱
- CI や `nt_gate.py` でのホスト FW 有無ゲート
- Linux / Windows 向け長手順の追加（本 OP の必須外）
