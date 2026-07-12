# 残存ワーク Foolproof 実行計画（v1.4）

> **作成**: 2026-07-10（v1.0）  
> **改訂**: v1.1 → **v1.2**（Human NT 拡充）→ **v1.3**（NT-2 実装ゲート分離）→ **v1.4**（2026-07-13: NT-2=done・§8 R-A〜R-D 閉じ / OPEN OP-078・077・079）→ **v1.5**（2026-07-13: Local LLM /reflexion 残リスク LL-A〜D → OP-080〜082）  
> **根拠**: `OPEN.md` + `near_term_public_beta_plan.md` v5.1 + 運用正本（stripe-production-setup / QUICK_START_VERIFICATION / MESSAGING §8 / OPERATIONS_MANUAL §8 / release-preflight）  
> **タスク正本**: [`OPEN.md`](../../OPEN.md)（本計画は手順のみ。ID の二重管理をしない）  
> **ステータス**: Wave A1/A2/A3 ✅。NT-1 ✅。NT-2 ✅（§8 閉じ）。**いま NT-3**（OP-002 Biome 目視・human-only / LL-C）。残: NT-5/6/7 / Gate / OP-059-UI / LL follow（OP-080〜082・NT-6 ブロッカー外）
> **Human 実行の超詳細版（推奨）**: [`docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md)（**v1.6**）— Human コピペ正本。進行は **`/nt-assist`** + `scripts/nt_gate.py`（1ステップ・秘密禁止）。§2 H-1 は要約。

---

## 0a. v1.1 → v1.2（Human 拡充）

| # | v1.1 の問題 | v1.2 の固定 |
|---|-------------|------------|
| 1 | Wave H が要約のみ（「正本を見ろ」止まり） | NT-1〜7 を **Step / コマンド / DoD / Negative / 記録テンプレ** まで記載 |
| 2 | NT-1 の `abyss-vault` 起動形が曖昧 | ランブック v1.2: **GUI 正本**（MC Vault マネージャ→key-proxy）。CLI は同一 DB 時のみ（stripe-production-setup / api_key_rotation §B） |
| 3 | NT-3 パスのみで「何を見るか」不足 | タブ操作・合格/不合格の目視基準を固定 |
| 4 | NT-5 が「§8 参照」のみ | 7 ショット + GIF の保存先・禁止事項・チェック表 |
| 5 | NT-6 が文書名のみ | OPERATIONS §8 スキップ表 + preflight ステップ 0〜8 の実行順 |
| 6 | NT-7 テンプレ MISSING | 本計画内に獲得・記録テンプレを内蔵 |

### v1.0 → v1.1 で潰した抜け（要約・詳細は git 履歴）

B5 実行前ゲート / `String` 戻り、B3=`Failed`、対象 5 箇所、napi 純関数テスト、App ≤520、ADR-012→OP-062 等。

---

## 0. 30 秒でわかる全体像

```
[Human 律速]  Public Beta 閉ループ（NT-1/2/3/5/6/7）
      │
      │  完了後 or 並行で Agent 可
      ▼
[Agent Wave A]  A1 docs → A2 App.tsx → A3 OP-075-B（B1→B5）
      │
      ▼
[Gate]  ADR-054 Accepted → OP-051
[Gate]  ADR-012 Accepted +「OP-062 を実装しろ」→ Tauri
[Watch] Upstream / CF Monetization Gateway（x402）
```

| レーン | 内容 | 誰 |
|--------|------|-----|
| **H** | Vault・Quick Start・Biome 目視・撮影・release・任意ベータ | **Human** |
| **A** | OP-059 docs / App.tsx 分割 / OP-075-B | Agent（明示承認後） |
| **G** | OP-051 / OP-062 | 追加ゲート必須 |
| **W** | Upstream / x402 | 監視のみ |

### やらない（再発明・スコープ外）

| 禁止 | 理由 |
|------|------|
| compose に `STRIPE_API_KEY` | Zero-Trust（near_term v5 撤回） |
| OP-011 封印解除 | Public Beta 外 |
| Nurture `STRIPE_SECRET_KEY` リネーム | 別系統 |
| OP-051 一括置換 | ADR-054 **Proposed** |
| OP-054-B / JobQueue ISP | ADR-031 |
| OP-075 本体の再実装 | 2026-07-10 完了 |
| CF Monetization Gateway 組み込み | waitlist・観察のみ |
| commerce/webhook/auth/key-proxy ロジック | Safety-Critical |
| Tauri `lib.rs` を OP-062 以外で変更 | Safety-Critical Zone |

---

## 1. 残存タスク棚卸し（OPEN × 実コード・再検証済）

### 1.1 Human — 正本 [`near_term_public_beta_plan.md`](near_term_public_beta_plan.md) v5.1

| NT | OPEN | コード状態 | 次アクション |
|----|------|------------|--------------|
| **NT-1** | OP-057-R (1) | 手順 ✅ / 反映 ❌ | Vault |
| **NT-2** | G1 / OP-078 ✅ | API+MC proxy DoD + §8 R-A〜R-D ✅（2026-07-13） | [`nt2_quickstart_unblock_plan.md`](nt2_quickstart_unblock_plan.md) §8（完了） |
| **LL follow** | OP-080〜082 | Pattern A ✅ / B 実機❌ / git 未コミット | [`local_llm_ab_reflexion_plan.md`](local_llm_ab_reflexion_plan.md) §2（**NT-6 必須ブロッカー外**） |

| **NT-3** | OP-002 | `BiomeCanvas.tsx:99` / `BiomeRenderer.tsx:187` DONE | 目視（**LL-C** = 同一） |
| NT-4 | OP-013 | ✅ | 再実行不要 |
| **NT-5** | OP-063 | OGP ✅ / 撮影 ❌ | 7+GIF |
| **NT-6** | R5 | WF ✅ | 「実行しろ」後 |
| **NT-7** | OP-064 | 本計画 §2.H-6 にテンプレ内蔵 | 任意（5名+） |

### 1.2 Agent（承認後）

| ID | 要約 | アンカー | 工数 |
|----|------|----------|------|
| **A1 / OP-059-C** | MESSAGING L179 + OPEN クローズ | ✅ 2026-07-10 | — |
| **A2 / UI-SHELL** | App.tsx 786→456 + 6 ファイル分割 | ✅ 2026-07-11 | — |
| **A3 / OP-075-B** | Fail-Closed **5** 箇所 | ✅ 2026-07-11 | — |
| **OP-059-UI** | Settings `pro_monthly_kc_allowance` | 未着手 | 別承認後 |

### 1.3 ゲート付き

| ID | ゲート | アンカー |
|----|--------|----------|
| OP-051 | ADR-054 → **Accepted** +「実装しろ」 | `054-error-hierarchy.md:3` Proposed |
| OP-062 | **ADR-012 Accepted** +「OP-062 を実装しろ」 | `src-tauri/src/lib.rs:529–548`（`InProcess` なし） |

### 1.4 Watch

| ID | 方法 | 実装しない |
|----|------|------------|
| OP-030–034 / OP-068 | `scripts/watch_upstream_blockers.py` | Upstream |
| X402 本番鍵 | `aiome-commerce/src/x402.rs:52–58` | Federation 後 |
| CF Gateway | [blog](https://blog.cloudflare.com/monetization-gateway/) | waitlist |

---

## 2. Wave H — Human Public Beta（Foolproof）

> **実行はこちらを優先してよい**: [`docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md)（画面操作・判定表・コピペをさらに噛み砕いた版）。  
> **Agent の役割**: コード・Vault 値・compose への秘密追加はしない。ユーザーが「NT-N を進める」と言ったら、ランブックまたは本節の該当チェックリストをそのまま読み上げ・進捗を記録する。  
> **Human の役割**: 下記 Step を上から実行し、各 DoD を満たしたら OPEN / 本計画の記録欄に結果を残す。

### 2.0 Human 共通ルール

| ルール | 詳細 |
|--------|------|
| 秘密をチャットに貼らない | `sk_live_` / `whsec_` / Vault マスタパスワードは Agent 対話に出さない |
| compose に API キーを足さない | `docker-compose.production.yml` の api-server に `STRIPE_API_KEY` **禁止**（L83–105 は `KEY_PROXY_URL` + 非秘密のみ） |
| コード変更禁止 | `commerce_webhook/` / `auth.rs` / `key-proxy` ロジックは触らない |
| Nurture 別系統 | `STRIPE_SECRET_KEY`（Nurture）は本 Wave の対象外 |
| 依存 | NT-1・2・3 完了後に NT-6。NT-5∥可。NT-7 任意 |

```
NT-1 ──┬──→ NT-6 → 公開（R5-5）
NT-2 ──┤
NT-3 ──┘
NT-5 →（後続 R4-2 LP 組込は Agent/Sub）
NT-7（任意∥・公開ブロッカー外）
NT-4 ✅ 済み（再実行不要）
```

---

### H-1 NT-1 = R2-1 = OP-057-R (1) — Stripe 本番反映 🔐

**Human 実行の詳細正本（重複時はこちら）**: [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) **NT-1（v1.6+）** — Step 0→A→B→C→D。  
**技術正本**: [`docs/operations/stripe-production-setup.md`](../operations/stripe-production-setup.md)  
**補助**: [`docs/operations/api_key_rotation.md`](../operations/api_key_rotation.md) §B GUI  

> 本 H-1 はチェックリスト要約。**コマンドの全文コピペはランブック NT-1 を開いて実行**する（ここに再掲しない＝重複防止）。

#### 事前チェック（全部 Yes で開始）

- [ ] 方針 A（test）または B（live）を決めた  
- [ ] Price ID を控えた（チャットに全文禁止）  
- [ ] 本番ホストに `VAULT_SECRET` / `VAULT_MASTER_PASSWORD`  
- [ ] compose に `STRIPE_API_KEY=` 代入なし  
- [ ] compose の api-server が `docker/distroless.Dockerfile`（ファイル）

#### Step 0 — distroless を本番に載せる（必須・ランブック詳細へ委譲）

本番イメージが MC 付き distroless であることを保証するため、ホスト環境で `git pull`、`chown`、`build`、`up -d` を実行し、稼働 Labels を確認します。

> [!IMPORTANT]
> プロセスの `restart` だけでは Docker イメージは更新されません。必ず詳細手順とコマンドを [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-1 **Step 0** で確認し、実行してください（コマンドの重複を避けるため、本節でのコマンド全文は省略しランブックへ委譲します）。

- [ ] Step 0 DoD PASS  

#### Step A — 秘密（AbyssVault）

**推奨**: 本番 MC → **まもる・整える → 設定 → Abyss Vault**（Step 0 後）。CLI は同一 Vault DB 時のみ。

- [ ] `STRIPE_API_KEY` 格納  
- [ ] `STRIPE_WEBHOOK_SECRET` 格納（または Step C 後に完了とメモ）

#### Step B — 非秘密（ホスト env / compose パススルー）

**方針 A**: `STRIPE_TEST_MODE=true` + test Price  
**方針 B**: `STRIPE_TEST_MODE=false` + live Price  

`VITE_STRIPE_PRICE_ID` は任意（`price_gold_monthly` エイリアス可）。詳細はランブック / stripe-production-setup §2.1。

- [ ] TEST_MODE 一致  
- [ ] ホスト Price 設定済  
- [ ] 方針 B で Price 空起動しない  

#### Step C — Webhook（Stripe Dashboard）

ランブック NT-1 Step C（イベント 7・whsec・`restart api-server`＝注入再読込）。

- [ ] Webhook 登録 + 注入ログ OK（値は出ない）

#### Step D — DoD（Positive）

| # | 確認 | 合格条件 |
|---|------|----------|
| D0 | 稼働 distroless | Step 0 DoD |
| D1 | Checkout 開始 | PlanBadge / コインとポイント / 402 |
| D2 | アプリ Checkout 1 件 | LP Link 単独不可 |
| D3 | Pro unlock | PlanBadge 等 |
| D4 | compose 衛生 | キー代入なし |

#### Negative

GUI/CLI で API キー一時削除 → restart → 拒否 → **復元** → restart。

#### 完了記録

```
NT-1 / OP-057-R(1) / R2-1
日付: YYYY-MM-DD
Step0 distroless: PASS
Vault: GUI|CLI / 方針: A|B / Price末尾4: ____
Webhook7 / Pro / Negative復元
```

#### Agent 禁止

- compose に `STRIPE_API_KEY` 追加 / 秘密をチャットへ / webhook コードのついで修正

---

### H-2 NT-2 = G1 = R3-4 — Quick Start 実走

**現状（2026-07-13）**: 公式 compose DoD PASS + **§8 R-A〜R-D すべて閉じ**（NT-2=done / OP-078・077・079）。手順の再掲はしない。

| 正本 | 用途 |
|------|------|
| [`nt2_quickstart_unblock_plan.md`](nt2_quickstart_unblock_plan.md) | 実装履歴 + **§8 残リスク対応計画（R-A〜R-D）** |
| [`QUICK_START_VERIFICATION.md`](../guides/QUICK_START_VERIFICATION.md) | **Human / 代理実走**の唯一の手順・DoD・Negative |
| [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-2 | 入口 |

#### 実装ゲート（要約）

| ID | 状態 |
|----|------|
| B1–B6 / I-1–I-5 / V（API 代理） | ✅ |
| I-3 GHCR quickstart タグ | 任意・未 |
| §8 R-A イメージ再ビルド煙 | ✅ 2026-07-13（`--no-build` FATAL → `--build` healthy） |
| §8 R-B ブラウザ人手 | ✅ 2026-07-13（API+MC proxy 代理） |
| §8 R-C / R-D | ✅ 2026-07-13（OP-077 / OP-079） |

#### Human / Agent フォロー

手順・記録は **VERIFICATION**。残リスク **§8 R-A〜R-D すべて ✅**（2026-07-13）。`nt_gate` browser=PASS。次は NT-3。

---

### H-2.5 Local LLM A/B /reflexion フォロー（OP-080〜082）

**正本**: [`local_llm_ab_reflexion_plan.md`](local_llm_ab_reflexion_plan.md)（手順の再掲はしない）。

| ID | OPEN | 要約 | ブロッカー |
|----|------|------|------------|
| **LL-A** | OP-080 | Pattern B 実機: `pattern-b-up` → API 煙 → `pattern-a-up` 復帰。Negative: 11434 競合 | **いいえ**（macOS quickstart は Pattern A で可） |
| **LL-B** | OP-081 | 未コミット diff の論理分割コミット（`.env` 除外） | いいえ（ユーザー承認後） |
| **LL-C** | OP-002 | NT-3 Biome 目視 — **H-3 と同一** | **はい**（Public Beta Human ゲート） |
| **LL-D** | OP-082 | Linux `extra_hosts`（需要ゲート後） | いいえ |

**Agent 禁止**: Pattern B を本番 compose 既定にする / `.env` コミット / LL-C を代理 PASS。

---

### H-3 NT-3 = OP-002 = R1-16 = LL-C — Biome `alpha:false` 目視

**コード（変更不要・確認のみ）**:

| ファイル | 行 | 内容 |
|----------|-----|------|
| `apps/management-console/src/lib/biome/BiomeCanvas.tsx` | 99 | `alpha: false,` |
| `apps/management-console/src/lib/biome/BiomeRenderer.tsx` | 187 | `alpha: false,` |

#### Step

1. ローカルまたは quickstart で Management Console を開く  
2. **コックピット**に切替（**まもる・整える → 設定 → インターフェース複雑度 →「コックピット」**）  
3. サイドバー **そだてる → ワールド**（Biome）を開く  
4. キャンバス背後の合成を観察  

#### 合格 / 不合格

| 判定 | 見た目 |
|------|--------|
| **PASS** | 背景が不自然な不透明グレー板にならず、意図した透明合成（下層 UI / 雰囲気が見える） |
| **FAIL** | キャンバス全体が灰色の矩形で塗りつぶされ、下層が完全に隠れる |

参考: `FluidBackground.tsx:43` も `alpha: false`（本 NT の主対象は Biome 2 ファイル）。

#### Negative

- 一時的に DevTools で canvas を非表示 → 下層が見えること（合成の前提確認）。元に戻す。

#### 記録

```
NT-3 / OP-002
日付: YYYY-MM-DD
結果: PASS / FAIL
ブラウザ: Chrome / Safari / …
スクショ: （任意・個人情報なし）
```

完了時: OPEN OP-002 を `[x]`。

---

### H-4 NT-5 = OP-063 = R4-1 — 証拠ビジュアル

**正本**: [`docs/marketing/MESSAGING.md`](../marketing/MESSAGING.md) **§8**（L129–141）

#### 要件（全ショット共通）

- [ ] 実データ（ダミープレースホルダ画面だけは不可）  
- [ ] ダークテーマ  
- [ ] **1920×1080 以上**  
- [ ] 個人情報・API キー・トークンが映らない  
- [ ] OGP の再生成は不要（既存 `docs/landing/public/ogp.png` 等）

#### 推奨保存先（リポジトリ外でも可。入れるならパスを統一）

```text
docs/assets/evidence/YYYY-MM-DD/
  01-quickstart.gif          # 約30秒
  02-audit.png
  03-buzz-approval.png
  04-nurture-economy.png
  05-workflow-builder.png
  06-agent-diorama.png
  07-prompt-stats.png
```

#### ショットチェックリスト

| # | 内容 | UI 到達 | ファイル | 済 |
|---|------|---------|----------|----|
| 1 | SetupWizard → Playbook → Home（GIF ~30s） | 初回 or 設定リセット相当 | `01-quickstart.gif` | [ ] |
| 2 | 監査ログ | **アクティビティ**（`karma`）を開き内部タブ「監査ログ」 | `02-audit.png` | [ ] |
| 3 | 承認キュー | **SNS承認**（`buzz-approval`） | `03-buzz-approval.png` | [ ] |
| 4 | エコノミー | **コインとポイント**（`nurture`） | `04-nurture-economy.png` | [ ] |
| 5 | ワークフロー | **ワークフロー**（`workflow-builder`） | `05-workflow-builder.png` | [ ] |
| 6 | チャット+アバター | **AIとはなす**（`agent`）+ Diorama | `06-agent-diorama.png` | [ ] |
| 7 | LLM 統計 | **アクティビティ** → 内部タブ「使用量」 | `07-prompt-stats.png` | [ ] |

> **注意（2026-07-11）**: `audit` / `prompt-stats` はサイドバー独立項目ではない（U6-5）。詳細は [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-5。

#### DoD

- [ ] 静止画 6 + GIF 1（上表 7 点）が揃う  
- [ ] 各ファイルが 1920×1080 以上（GIF はフレーム解像度）  
- [ ] 秘密の映り込みなし（自分で目視）

#### Negative

- キーや `.env` が写ったショットは **破棄して撮り直し**（リポジトリに入れない）

#### 後続（本 NT の外）

- R4-2: LP / README への組込は Agent/Sub（「R4-2 を実装しろ」後）

#### 記録

```
NT-5 / OP-063
日付: YYYY-MM-DD
パス: docs/assets/evidence/…
結果: 7/7 PASS / 欠番: …
```

完了時: OPEN OP-063 を `[x]`（組込未なら「撮影完了・R4-2 待ち」と注記）。

---

### H-5 NT-6 = R5 — リリースゲート（「NT-6 を実行しろ」必須）

> Agent はユーザーが **「NT-6 を実行しろ」** と言うまで preflight を走らせない（Scope Lock）。

#### 実行順

| 順 | 正本 | 内容 |
|----|------|------|
| 1 | [`OPERATIONS_MANUAL.md`](../guides/OPERATIONS_MANUAL.md) **§8** | 本番チェックリスト |
| 2 | [`.agent/workflows/release-preflight.md`](../../.agent/workflows/release-preflight.md) | ステップ 0〜8 |
| 3 | [`release_master_plan.md`](release_master_plan.md) R5-2〜5 | 切り出し・ロールバック・docs・公開 |

#### §8 スキップ規則（二重作業防止）

| OPERATIONS §8 行 | NT で済んでいれば |
|------------------|-------------------|
| Quick Start 実走 (G1) | **NT-2 PASS ならスキップ**（記録を添付） |
| Stripe 本番反映 (R2-1) | **NT-1 PASS ならスキップ** |
| PostgreSQL / Keychain スクリプト | OP-012 / OP-014 完了済みなら結果再利用可。疑わしいときだけ再実行 |

その他の §8 項目（`VAULT_SECRET` / `NURTURE_*` / `A2A_NODE_TOKEN` 等）は **本番ホスト向けに残チェック**。

#### release-preflight コピペ順

```bash
# 0.5 DAG
python3 scripts/enforce_dag.py

# 1 gitleaks
gitleaks detect -v 2>&1 | tail -5

# 2 衛生（ワークフロー記載の一括 echo コマンド）
# 3 ローカルパス
git ls-files | grep -v node_modules | xargs grep -rl "/Users/" 2>/dev/null || echo "OK: No local paths found"

# 4 誤 URL
grep -rn "google/antigravity" README.md README_en.md 2>/dev/null || echo "OK: No wrong URLs"

# 5 ビルド
cargo check --workspace 2>&1 | tail -3

# 5.5 ignored ゲート
cargo test --workspace -- --ignored --skip sandbox --skip vendor 2>&1 | tail -10

# 6 サイズ — PASS: ≤2500 files かつ ≤75MB
echo "Tracked files:" && git ls-files | wc -l && echo "Estimated size:" && git ls-files -z | xargs -0 du -ch 2>/dev/null | tail -1

# 7.5 CHANGELOG
awk '/^## \[Unreleased\]/{f=1;next} /^## \[/{f=0} f' CHANGELOG.md | wc -l
# 200 超なら R5-2 でバージョン切り出し必須

# 8 LICENSE（1行目 BUSL。Apache は Change License なので grep -o Apache は使わない）
head -1 LICENSE
grep -n "BUSL\|Business Source\|License-BUSL\|License-BSL" README.md | head -5
```

**ステップ 0（Human）**: ロールバック手順（Feature Flag / `git revert` / DB 復元の**実リンク**）を Issue かリリースノート草案に書く（空欄は FAIL）。  
**ステップ 7（Human）**: GitHub About — Description（MESSAGING §7）+ **Website** + Topics + Social preview。R4-4 と同一作業ならここでまとめて実施。  
**実行場所**: preflight は**開発機の clone**（本番 SSH ではない）。詳細コピペは [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-6。

#### R5-2〜5

| ID | 担当 | DoD |
|----|------|-----|
| R5-2 | Sub/Agent（承認後） | `[Unreleased]` ≤200 行へ切り出し |
| R5-3 | Main/Human | ロールバック明文化 |
| R5-4 | Sub/Agent | `/docs-sync` 相当 |
| R5-5 | **Human 承認** → Main | GitHub Release + タグ |

#### DoD（NT-6）

- [ ] preflight 全ステップ OK（または NG を修正して再実行）  
- [ ] §8 の未スキップ項目がチェック済み  
- [ ] **NT-5 = 7/7**（R5-5 直前）  
- [ ] R5-5 まで進む場合は Human が公開を明示承認  

#### Negative

- gitleaks / DAG / ignored テストが 1 件でも NG → **公開中止**（「だいたい OK」で進めない）

#### 記録

```
NT-6 / R5
日付: YYYY-MM-DD
preflight: PASS / FAIL（失敗ステップ: ）
§8 残チェック: __ 項目
R5-5 公開: 未 / 承認済 / 完了
```

---

### H-6 NT-7 = OP-064 = R4-3 — ベータ獲得（任意・公開ブロッカー外）

**release_master**: 5 名以上。テスティモニアルなしでも公開可（後追い掲載）。

#### Step

1. 招待チャネルを決める（Discord / X / 知人 等）  
2. 招待文は MESSAGING 禁止表現リスト（§10）を守る  
3. 下記台帳に 5〜10 名を記録  
4. 実名掲載の許可が取れたら testimonial 文を 1〜3 件確保  

#### 記録テンプレ（本計画が正本・別ファイル不要）

```markdown
# Beta roster (OP-064) — リポジトリ外または private メモ推奨

| # | 表示名 | 連絡先（非公開） | 許諾 (Y/N) | Testimonial 要約 | 日付 |
|---|--------|------------------|------------|------------------|------|
| 1 |        |                  |            |                  |      |
| 2 |        |                  |            |                  |      |
…最大 10
```

#### DoD

- [ ] 5 名以上が実利用（ログイン〜チャット相当）  
- [ ] 公開用に使える実名+一言が **0 でも公開可**（あれば LP 後追い）

#### 記録（OPEN）

```
NT-7 / OP-064
日付: YYYY-MM-DD
人数: N
Testimonial: 0 / K
結果: PASS（任意完了） / DEFER（公開後）
```

---

### H-7（参考）NT-4 — 再実行不要

2026-07-10 完了。回帰時のみ:

```bash
cargo test -p api-server api_integration_tests::commerce -- --test-threads=1
cargo test -p api-server commerce_e2e_tests -- --test-threads=1
cargo test -p aiome-commerce -- --test-threads=1
```

---

## 3. Wave A1 — OP-059 docs クローズ

### 事実

| 項目 | 状態 | アンカー |
|------|------|----------|
| KC 月次付与 | ✅ | `commerce_webhook/stripe.rs:346–381` |
| 月間支出上限 | ✅ | ADR-050 + `interceptor.rs` + Settings `economy.monthly_spend_limit` |
| ALLOWED キー | ✅ | `settings.rs:103` `pro_monthly_kc_allowance` |
| Settings UI allowance | ❌ | **作らない**（OP-059-UI） |
| stale 文 | ❌ | `MESSAGING.md:179` |

### 手順（確定文面）

**1. `MESSAGING.md` L179 を次に置換**（L106 の「含み枠数値を LP に書かない」は維持。L175 の設計表「例: 1,000 KC」は**変更しない**）:

```markdown
**【採否決定 2026-07-03 / 実装状況 2026-07-10】**: ハイブリッド案は採用済み。
バックエンドは実装済み（月次 KC 付与: Stripe webhook / 月間支出上限: ADR-050 + interceptor + Settings）。
Settings に `pro_monthly_kc_allowance` 入力 UI は未着手（OP-059-UI）。
**対外文書（LP・README）に含み枠の具体数値を書かないこと**は継続（L106・禁止表現 #1）。
```

**2. `OPEN.md` L60** OP-059 を `[x]` にし末尾に:

```text
→ **2026-07-10 docs クローズ**（コードは先行完了。残: OP-059-UI = Settings allowance 入力）
```

必要なら同日 OPEN に `- [ ] **OP-059-UI**: ...` を 1 行追加。

**3. CHANGELOG** `[Unreleased]` に docs クローズ 1 行。

### 検証

| 段階 | コマンド / 確認 |
|------|-----------------|
| Positive | `rg "バックエンドは未実装" docs/marketing/MESSAGING.md` → 0 |
| Negative | `rg "含み枠" docs/landing README.md README_en.md` に新規数値が増えていない |
| Revert | 不要 |

### ゲート文言

```
現在は Wave A1（OP-059 docs クローズ）です。commerce / interceptor / SettingsPage には触れません。
```

---

## 4. Wave A2 — App.tsx シェル分割

### 現状（再計測）

| 項目 | 値 |
|------|-----|
| ファイル | `apps/management-console/src/App.tsx` |
| 行数 | **786** |
| 型 + `NAV_GROUPS` | L97–157（`NavItemDef` L97–105 / groups L106–157） |
| `App()` | L159–758 |
| vitality switch | L245–357 |
| `renderStatusBadge` | L359–413 |
| Sidebar | **L530–617**（`aside` 本体 L538–617） |
| Header | L621–682 |
| タブ switch | L715–741 |
| `NavItem` | L760–784 |
| 既存分割ファイル | `navConfig` / `AppSidebar` / `NavItem.tsx` 等は **未作成**（二重実装なし） |
| 同期コメント | `lib/a2uiTabs.ts:9` |

### 分割順序（1 コミット = 1 Step 推奨）

| Step | 新規 | 切り出し | App.tsx 側 |
|------|------|----------|------------|
| **S1** | `src/navConfig.tsx` | L97–157（interface + `NAV_GROUPS`）+ 必要な lucide import | `import { NAV_GROUPS, ... } from './navConfig'` |
| **S2** | `src/components/NavItem.tsx` | L760–784 | import |
| **S3** | `src/components/AppSidebar.tsx` | L530–617 | props: `viewMode`, `isMobileNav`, `isSidebarOpen`, setters, `workspacePersona`, `isVisible`, `activeTab`, `setActiveTab`, `t`, `navContainerRef` |
| **S4** | `StatusBadge.tsx` + `AppHeader.tsx` | L359–413 + L621–682 | タイトル map は Header 内に同居可 |
| **S5** | `src/AppRoutes.tsx` | L715–741 | props: `activeTab`, `stats`, … 既存 JSX をそのまま |
| **S6**（任意・≤400 用） | `hooks/useVitalityEventProcessor.ts` | L245–357 | |

### S1 確定形（抜粋）

```tsx
// apps/management-console/src/navConfig.tsx
import { Home, MessageSquare, /* 既存 NAV で使う lucide のみ */ } from 'lucide-react';

export interface NavItemDef { tab: string; labelKey: string; icon: React.ReactNode; }
export interface NavGroupDef { sectionKey: string; items: NavItemDef[]; }

export const NAV_GROUPS: NavGroupDef[] = [ /* App.tsx から一字一句移動 */ ];
```

### 禁止

| 禁止 | 理由 |
|------|------|
| タブ ID リネーム | `a2uiTabs.ts` / i18n / Jest |
| ProUpgradeModal / TaskApproval 変更 | 課金・承認 |
| `as any` / HEX 直書き | OP-028 / OP-029 |

### 検証

```bash
cd apps/management-console
npm test -- --watchAll=false
python3 ../../scripts/test_ui_hex_violations.py
wc -l src/App.tsx
# S1–S5 後: ≤520 / S6 後: ≤400

# Positive: NAV_GROUPS の tab 集合 ⊆ a2uiTabs whitelist（手動 diff）
# Negative: navConfig から 'settings' を一時削除 → Settings ナビ消滅を確認し戻す
```

### ゲート文言

```
現在は Wave A2（App.tsx 分割）です。S1 から着手します。
api-server / commerce / Tauri には触れません。
```

---

## 5. Wave A3 — OP-075-B Immune Fail-Closed（5 箇所）

### 参照正本（変更禁止・コピー元）

`apps/api-server/src/tool_call_router.rs:64–98` — `match` で `Err` → deny。

### 対象一覧

| # | ファイル | 関数 / 箇所 | 行 | Fail-Open の意味 |
|---|----------|-------------|-----|------------------|
| **B1** | `libs/napi-bridge/src/lib.rs` | `immune_check_tool` | 294–315 | `Err` → `blocked: false` |
| **B2** | 同 | `immune_scan_input` | 390–397 | `Err` → `Ok(())` |
| **B3** | `libs/infrastructure/src/task_orchestrator/goal_processor.rs` | `process_goal_job` 内 `verify_tool_call` | 146–148 | `Err` → 計画続行・enqueue |
| **B4** | `commercial/apps/nurture-api/src/mcp/server.rs` | `tools/call` | 233–254 | `Err` → ツール実行 |
| **B5** | `apps/api-server/src/skill_handler.rs` | `execute_wasm_skill` | 実行 L230+ / fetch L291–296 | 実行後 `unwrap_or_default` |

触らない: L184 low severity proceed / L132–134 `IMMUNE_BYPASS_APPROVED` / OP-075 本体。

---

### 5.1 共通: napi 用純関数（B1/B2 テスト可能化）

`libs/napi-bridge/src/lib.rs` に（`#[napi]` の外・`pub(crate)` 可）:

```rust
pub(crate) enum ImmuneGate<R> {
    Allow,
    Block(R),
}

/// verify_intent / 同等 Result を Fail-Closed で解釈する（ユニットテスト用）
pub(crate) fn gate_immune_result<E: std::fmt::Display, R>(
    result: Result<Option<R>, E>,
    on_rule: impl FnOnce(R) -> ImmuneGate<String>,
    deny_msg: &str,
) -> ImmuneGate<String> {
    match result {
        Ok(Some(rule)) => on_rule(rule),
        Ok(None) => ImmuneGate::Allow,
        Err(e) => {
            tracing::error!(error = %e, "[Security] immune verify failed; deny");
            ImmuneGate::Block(deny_msg.to_string())
        }
    }
}
```

テスト（既存 `mod tests` L514 に追加）:

```rust
#[test]
fn test_gate_immune_result_err_is_deny() {
    let g = gate_immune_result::<&str, ()>(Err("db down"), |_| ImmuneGate::Allow, "DENIED");
    match g {
        ImmuneGate::Block(m) => assert!(m.contains("DENIED")),
        _ => panic!("must deny"),
    }
}

#[test]
fn test_gate_immune_result_ok_none_allow() {
    let g = gate_immune_result::<&str, ()>(Ok(None), |_| ImmuneGate::Block("x".into()), "DENIED");
    assert!(matches!(g, ImmuneGate::Allow));
}
```

---

### 5.2 B1 — `immune_check_tool` 確定形

`if let Ok(Some(rule)) = ...`（L294–309）を削除し:

```rust
let intent = format!("{} with params: {}", context_topic, params);
match gate_immune_result(
    immune.verify_intent(&intent, db.as_ref()).await,
    |rule| {
        ImmuneGate::Block(format!(
            "[SENTINEL] Adaptive Block: {} (Pattern: {})",
            rule.action, rule.pattern
        ))
    },
    "[SENTINEL] Unable to verify immune status. Request denied.",
) {
    ImmuneGate::Block(reason) => {
        return Ok(ToolCheckResponse {
            blocked: true,
            reason: Some(reason),
            new_params: None,
        });
    }
    ImmuneGate::Allow => {}
}
```

（`gate_immune_result` を使わず直接 `match` でも可。その場合も **同一テスト可能な match 腕**を維持。）

---

### 5.3 B2 — `immune_scan_input` 確定形

```rust
match gate_immune_result(
    immune.verify_intent(&prompt, db.as_ref()).await,
    |rule| {
        ImmuneGate::Block(format!(
            "[SENTINEL] Blocked by Rule: {} -> action: {}",
            rule.pattern, rule.action
        ))
    },
    "[SENTINEL] Unable to verify immune status. Request denied.",
) {
    ImmuneGate::Block(msg) => Err(napi::Error::from_reason(msg)),
    ImmuneGate::Allow => Ok(()),
}
```

---

### 5.4 B3 — `goal_processor` 確定形

`process_goal_job` 内 L146–148 の `if let Ok(Some(rule))` を `match` に変更。

**Err 腕（確定）** — 高 severity の `AwaitingInput` を流用しない:

```rust
Err(e) => {
    error!(
        error = %e,
        "[Security] verify_tool_call failed; failing goal {} (fail-closed)",
        job.id
    );
    let reason = "Unable to verify immune status. Request denied.".to_string();
    if let Err(fe) = self.job_queue.fail_job(&job.id, &reason).await {
        error!("fail_job after immune Err: {:?}", fe);
    }
    let _ = self
        .job_queue
        .update_job_status(&job.id, aiome_core_contracts::traits::JobStatus::Failed)
        .await;
    return Ok(()); // サブジョブ enqueue 禁止（L192 以降に進まない）
}
```

`Ok(Some(rule))` の既存 severity 分岐（L152–185）は**維持**。  
`IMMUNE_BYPASS_APPROVED`（L132–134）は**維持**。

**テスト追加先**: `libs/infrastructure/src/task_orchestrator/tests.rs`  
既存免疫ブロックテスト（`active_rules` + `network_sender` 付近 L633–）を参考に、**JobQueue の `verify_tool_call` が Err になるモック**、または `fetch_active_immune_rules` が Err を返すモックを追加。

`MockJQ::fetch_active_immune_rules`（`testing/mock_jq.rs:409`）は常に `Ok` — **GlobalMockJobQueue 側に `fail_immune_fetch: bool` を足す**か、テスト専用 stub を tests.rs 内に定義。既存 `GlobalMockJobQueue` のフィールドを壊さないこと。

**N-B3 DoD**: immune Err 後、`store_trajectory_step` / enqueue が呼ばれていない（モックカウンタ or job が `Failed`）。

---

### 5.5 B4 — nurture-api MCP 確定形

`commercial/apps/nurture-api/src/mcp/server.rs` L233–254 を `match`:

```rust
match state
    .immune_system
    .verify_intent(&args_str, state.job_queue.as_ref())
    .await
{
    Ok(Some(rule)) => {
        // 既存 block レスポンス（L238–252）をそのまま
        ...
    }
    Err(e) => {
        tracing::error!(error = %e, "[Nurture-MCP] immune verify_intent failed; deny");
        return JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Unable to verify immune status. Request blocked by AdaptiveImmuneSystem."
                    .to_string(),
                data: None,
            }),
        };
    }
    Ok(None) => {}
}
```

**テスト**: 同ファイル `mod tests` L522+ に追加。既存 `test_verify_intent_blocks_injection`（L568）の隣。

Err 注入が困難な場合: `handle_mcp_request` 前に immune を差し替えられるなら差し替え。できなければ **B4 用に `fn map_mcp_immune(...)` を抽出しユニットテスト**（B1 と同型）。「テストなしでマージ」は禁止。

---

### 5.6 B5 — `skill_handler` 確定形（実行前ゲート）

**現状バグある手順を禁止**: L294 の `unwrap_or_default` だけ直しても、スキルは L242–246 で**既に実行済み**。

**確定手順**:

1. `execute_wasm_skill` 冒頭（`UnverifiedSkill` 構築の直後、**`unverified.verify` より前** ≈ L200 手前）に:

```rust
let immune_rules = match state.job_queue.fetch_active_immune_rules().await {
    Ok(r) => r,
    Err(e) => {
        tracing::error!(
            error = %e,
            "[Security] fetch_active_immune_rules failed; deny skill {}",
            skill_name
        );
        return format!(
            "[{} Error: Unable to verify immune status. Request denied.]",
            skill_name
        );
    }
};
```

2. 後段 L291–296 を:

```rust
let checker = infrastructure::constraint_checker::ConstraintChecker::new(
    immune_rules,
    state.wasm_skill_manager.get_metadata(skill_name).map(|m| m.permissions).unwrap_or_default(),
);
```

3. **`unwrap_or_default()` 削除**。

**N-B5** ✅: `test_execute_wasm_skill_immune_db_error_fail_closed` — `immune_rules` DROP で Err → 実行前 return・`Unable to verify immune status`（`call_skill` 成功経路に到達しない）。

---

### 5.7 検証コマンド（コピペ）

```bash
# B1/B2
cargo test -p napi-bridge gate_immune_result -- --nocapture

# B3
cargo test -p infrastructure --test '*' 2>/dev/null; \
cargo test -p infrastructure immune -- --nocapture
# 追加テスト名に合わせて: cargo test -p infrastructure test_goal_immune_db_error_fail_closed

# B4（commercial workspace）
cd commercial && cargo test -p nurture-api test_verify_intent_db_error_fail_closed --lib -- --nocapture

# B5
cargo test -p api-server test_execute_wasm_skill_immune_db_error_fail_closed -- --nocapture
# B3 正本:
cargo test -p infrastructure --lib test_goal_immune_db_error_fail_closed -- --nocapture

cargo clippy -p napi-bridge -p infrastructure -p nurture-api -p api-server -- -D warnings
cargo fmt -p napi-bridge -p infrastructure -p nurture-api -p api-server -- --check
```

| ID | Positive | Negative |
|----|----------|----------|
| N-B1/B2 | `Ok(None)` → allow / `Ok(Some)` → block | `Err` → deny |
| N-B3 | ルールヒット既存 | Err → Failed・enqueue なし |
| N-B4 | 既存 injection テスト維持 | Err → JsonRpc error・sandbox 未実行 |
| N-B5 | 正常スキル | Err → 実行前 return・文言 |

### OPEN / CHANGELOG

- OPEN L54 OP-075-B → 完了時 `[x]` + 日付  
- 運用注意: immune/DB 障害時は napi / goal / nurture MCP / wasm skill も拒否（チャット OP-075 と同型）

### ゲート文言

```
現在は Wave A3（OP-075-B）です。B1 から着手します。
evaluate_security（OP-075）・BeggingSupervisor・commerce webhook には触れません。
```

---

## 6. Wave G1 — OP-051（Accepted 後）

前提（1 つでも No → 中止）:

- [ ] `054-error-hierarchy.md` Status = **Accepted**
- [ ] 「OP-051 を実装しろ」
- [ ] 一括置換スクリプト禁止

Accepted 後に**別計画**を書く（anyhow 概数: infrastructure 66 / api-server 65 / aiome-commerce 22）。本 v1.1 にコード手順は書かない。

---

## 7. Wave G2 — OP-062 🔐

### 前提（1 つでも No → 中止）

- [ ] ADR-012 Accepted（二重 Hook / KarmaForge 論点）
- [ ] 「OP-062 を実装しろ」
- [ ] api-server `NURTURE_IN_PROCESS` 経路は既存（`bootstrap/plugins.rs:20–34`）— **再発明しない**

### 現状

| 層 | アンカー |
|----|----------|
| Tauri enum | `lib.rs:529–533` `{ Local, Cloud(String), Disabled }` |
| resolve | `lib.rs:535–548` Cloud → Disabled → Local |
| sidecar | `lib.rs:192–222` Local のみ spawn |
| api-server | `plugins.rs:29–34` env で in-process |

### 確定優先順位

```text
1. NURTURE_DISABLED=true|1     → Disabled
2. NURTURE_CLOUD_URL 非空      → Cloud(url)
3. NURTURE_IN_PROCESS=true|1   → InProcess   ← 新規（Local より優先）
4. else                        → Local
```

### `start_sidecars` InProcess 腕

- nurture-api sidecar **spawn しない**
- `state.nurture_status = "in_process"`（または `"disabled"` と区別する文字列を 1 つに固定し `get_nurture_status` も更新）
- api-server sidecar の env に `NURTURE_IN_PROCESS=true` を渡す（既存 plugins と整合）

### 検証

| 段階 | 内容 |
|------|------|
| Positive | InProcess で nurture 子プロセスなし + api-server に env |
| Negative | InProcess+Local 相当の曖昧指定でも sidecar 二重起動しない |
| feature | `cargo check -p management-console`（Tauri） |

---

## 8. Wave W — 監視のみ

```bash
python3 scripts/watch_upstream_blockers.py
```

x402: `X402Negotiator` は買い手側資産。Stripe HTTP 402（Pro）と**混同禁止**。CF Gateway は実装しない。

---

## 9. 実行順序

```
H: NT-1 ∥ NT-2 ∥ NT-3 ∥ NT-5  →  NT-6 → 公開
A1 ∥ A2（Beta と並列可）
A3（PR 分離推奨）
G1 / G2（各ゲート後のみ）
W（常時）
```

**推奨 Agent 順**: A1 → A2 → A3。

---

## 10. /perfect-plan 検証結果（v1.2）

## 検証対象
`docs/roadmaps/remaining_work_foolproof_plan.md` v1.2（Human 拡充）

### Gate 1: 構造スキャン
- ✅ Human 正本実在: stripe-production-setup / QUICK_START_VERIFICATION / MESSAGING §8 / OPERATIONS §8 / release-preflight / compose L104–105。
- ✅ Agent 側 v1.1 修正（B5 実行前等）を維持。
- ✅ NT-7 テンプレを本計画に内蔵（外部 MISSING 解消）。

### Gate 2: 要件カバレッジ
- §2 経済: NT-1 Vault + Webhook 7 イベント。コード非変更。  
- §4 セキュリティ: 秘密を compose に戻さない・チャット非開示。  
- Human 手順に Positive/Negative/記録を追加。

### Gate 3: 依存
- NT-6 は NT-1/2 スキップ規則で二重作業を防止。  
- NT-5 → R4-2 は後続（本 Wave 外）。

### Gate 4: 悪魔の弁護人
1. **最悪**: Human が急いで compose に live キーを書く → **禁止を §2.0 / H-1 に再掲**。  
2. **誤前提**: 「Quick Start は開発 DB の固定パスワードでよい」→ **クリーン clone 必須**と明記。  
3. **やらないメリット**: NT-7 を公開ブロッカーにしない（release_master どおり）。

### Gate 5: 実行順序
- ✅ NT-1∥2∥3∥5 → NT-6 → R5-5。NT-4 再実行不要。

### 判定
- [x] ✅ **PASS（v1.2）** — Human / Agent ともコピペ実行可能。実装・実走は明示指示後。

---

## 11. ユーザー指示テンプレ

```
NT-1 を進める
```
```
NT-2 を進める
```
```
NT-3 を進める
```
```
NT-5 を進める
```
```
NT-6 を実行しろ
```
```
NT-7 を進める（任意）
```
```
Wave A1 を実装しろ
```
```
Wave A2 を実装しろ（S1 から）
```
```
Wave A3 を実装しろ（OP-075-B・B1 から）
```
```
ADR-054 を Accepted にした。OP-051 の実装計画を書け
```
```
ADR-012 を Accepted にした。OP-062 を実装しろ
```

---

## 12. 関連正本

| 文書 | 役割 |
|------|------|
| [`OPEN.md`](../../OPEN.md) | ID 台帳 |
| [`near_term_public_beta_plan.md`](near_term_public_beta_plan.md) | Human NT-* |
| [`tech_debt_top5_plan.md`](tech_debt_top5_plan.md) | Top5 完了 |
| [`remaining_tasks_implementation_plan.md`](remaining_tasks_implementation_plan.md) | Wave 1–3 |
| [`054-error-hierarchy.md`](../decisions/054-error-hierarchy.md) | OP-051 |
| Cloudflare [Monetization Gateway](https://blog.cloudflare.com/monetization-gateway/) | W 観察 |
