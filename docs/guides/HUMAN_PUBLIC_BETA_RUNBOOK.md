# Human Public Beta 実行ランブック（NT-1〜7）

**対象読者**: コードを書かない人・初めて触る人  
**目的**: このファイルを上から読み、書いてあるコマンドとチェックをそのまま実行すれば Public Beta の Human 作業が終わる  
**作成日**: 2026-07-11  
**改訂**: **v1.8**（2026-08-04）— OP-099 `prompt_evaluation_log` route_* 列（sqlite migration `20260801000000_prompt_eval_route_fields`）は sqlx がトランザクション内で一括適用。手動 ALTER する場合は 3 列すべて完了するか一括ロールバック（部分適用禁止）。v1.7: NT-1 Step A 直前ホスト outbound 監視（OP-095）。本線防衛は OP-096。

**タスク正本**: [`OPEN.md`](../../OPEN.md)  
**技術詳細**: [`remaining_work_foolproof_plan.md`](../roadmaps/remaining_work_foolproof_plan.md) §2  
**食い違い時**: 各 NT 冒頭の正本 → 本ランブック → foolproof §2

> ### まずこれ（ランブックが長いとき）
>
> Agent に **`/nt-assist`** または「NT-1 をアシストして」と言う。  
> **今の1ステップだけ**が進む。機械判定: `python3 scripts/nt_gate.py step0` / `self-test`。  
> 進捗: `states/nt_progress.json`（雛形 [`nt_progress.example.json`](nt_progress.example.json)）。  
> **このファイルは正本のまま**。薄くしない・Agent 用に全文をコピーしない。

---

## 検証ログ

### v1.1
Activity 内タブ / Vault delete / 公開前 NT-5 / grep 代替 / down -v / ja ナビ / 本番 Vault 注意 / test·live 脚注 / ツール表 / §10 抄録

### v1.2（CRITICAL）

| # | 発見 | 修正 |
|---|------|------|
| C1 | ホスト CLI Vault ≠ key-proxy DB | **正本は MC「まもる・整える→設定→Abyss Vault」** |
| C2 | Step B 常時 live と D2 方針 A 矛盾 | 方針 A/B を Step B に統合 |
| C3 | D1/D3 操作なし・LP 混同 | Checkout/PlanBadge + §0.6 |
| C4 | quickstart 固定 `container_name` | `docker ps` 検知 + 1420 Negative |
| C5 | preflight 0/7/8 不完全 | ロールバック・Website・バッジ・実行場所・R5-2..4 |
| I* | KEY_PROXY / VAULT_MASTER / whsec env / OPEN 更新 | 追記 |

### v1.3（HIGH）

| # | 発見 | 修正 |
|---|------|------|
| H1 | 「まもる」≠ UI | **まもる・整える → 設定** |
| H2 | NT-3/5 が cockpit 専用なのに未記載 | cockpit 必須を明記 |
| H3 | VITE 再ビルドと Docker が噛み合わない | `price_gold_monthly` エイリアス + 任意 VITE |
| H4 | preflight ステップ6 閾値欠落 | vendor除外≤2500 / ≤75MB |
| H5 | Part E に R5-3 なし | 表に追加 |
| H6 | 表の NT-5→6 が誤解を招く | 並列可・公開直前必須に統一 |
| M* | Checkout 導線 / 本番 MC URL / 再起動対象 / ステップ番号 | 追記 |

### v1.4（残ギャップ）

| # | 発見 | 修正 |
|---|------|------|
| C1 | compose の `production.Dockerfile` は MC dist 未同梱 | **distroless 必須**を NT-1 に明記 |
| H1 | LICENSE grep が Apache を先に拾い誤 FAIL | BUSL 1行目 + README バッジ照合 |
| H2 | OPERATIONS §8 R2-1 が CLI/VITE 必須のまま | §8 を GUI+エイリアスに同期 |
| M1 | NT-1 表の VITE「必須」と Step B 矛盾 | 表を任意に修正 |
| M2 | cockpit 切替が抽象 | **インターフェース複雑度→コックピット** |
| M3 | QUICK_START 正本に container 衝突なし | 正本へ追記 |
| L* | 付録 B 本番配信経路 / api_key_rotation beginner | 追記・修正 |

### v1.5（compose コード変更）

| # | 内容 |
|---|------|
| IMPL | `docker-compose.production.yml` の **api-server** を `docker/distroless.Dockerfile` に変更（実施済） |
| KEEP | key-proxy / samsara-hub / shadow-worker は `production.Dockerfile` 維持（`nc`/`curl`/`CMD-SHELL` healthcheck） |
| UID | api-server `user: "65532:65532"`（distroless nonroot）。旧 1001 とは異なる |
| HC | api-server `healthcheck.disable: true`（distroless に curl 無し） |
| DATA | `./data/api` は UID 65532 で書き込み可能であること |

### v1.6（実行抜け潰し）

| # | 発見 | 修正 |
|---|------|------|
| G1 | compose 変更後も `restart` のみ → 旧イメージのまま | **Step 0: pull → chown → build → up** |
| G2 | P6 がファイル `rg` のみ | **稼働コンテナの Image/Labels 確認** |
| G3 | foolproof H-1 が v1.5 未同期 | H-1 はランブック Step 0 へ委譲（重複排除） |

---

## 0. 最初に読む（ここを飛ばすと事故る）

### 0.0 事前に揃えるもの

| ツール / 権限 | NT | 確認 |
|---------------|-----|------|
| Docker Desktop | 2 | 起動できる |
| Git | 2 | `git --version` |
| Rust + Cargo（開発機・preflight） | 6 | `cargo --version` |
| `gitleaks` | 6 | `gitleaks version` |
| Stripe Dashboard | 1 | https://dashboard.stripe.com |
| 本番 MC 管理者ログイン | 1 | Vault GUI 用 |
| 本番ホスト SSH（env/再起動） | 1, 6 | 秘密はチャット禁止 |
| 本番ドメイン | 1 | Webhook URL |

`rg` が無ければ本文の `rg` を `grep -n` に読み替え。

### 0.1 一覧

| 順 | 名前 | 一言 | 必須 | 目安 |
|----|------|------|------|------|
| ① | NT-1 | Stripe 秘密を **本番 Vault** に入れ Webhook | 必須 | 30–60分 |
| ② | NT-2 | クリーン Quick Start | 必須 | 15–30分 |
| ③ | NT-3 | Biome（ワールド）目視 | 必須 | 5–10分 |
| ④ | NT-4 | Stripe 自動テスト | 済 | — |
| ⑤ | NT-5 | 証拠 6枚+GIF | **R5-5 公開直前に必須**（NT-6 開始前は不要） | 1–2時間 |
| ⑥ | NT-6 | preflight→公開承認 | 必須 | 1–3時間 |
| ⑦ | NT-7 | ベータ5人 | 任意 | 数日〜 |

```
NT-1 ──┐
NT-2 ──┼──→ NT-6 preflight（開発機）──┐
NT-3 ──┘                              ├──→ NT-5=7/7 ──→ 「公開してよい」(R5-5)
NT-5 ∥ NT-1〜3 と並行可 ──────────────┘
NT-7 任意 / NT-4 済
```

**順序の正**: NT-5 は NT-6 の preflight **開始条件ではない**。**「公開してよい」の直前**に 7/7 が必要。

### 0.2 絶対禁止

1. `sk_` / `whsec_` / パスワード / Vault 値をチャット・Issue・CHANGELOG に貼る  
2. `docker-compose.production.yml` に `STRIPE_API_KEY=...`  
3. コードのついで修正（commerce_webhook / auth / key-proxy）  
4. 「だいたい OK」で公開  
5. 「NT-6 を実行しろ」なしに Agent が preflight  
6. `abyss-vault get` の出力を共有  
7. Nurture の `STRIPE_SECRET_KEY` を本手順で触る  

### 0.3 用語

| 言葉 | 意味 |
|------|------|
| PASS/FAIL | 合格/不合格 |
| Vault | 秘密金庫。GUI 正本。CLI は上級 |
| DoD | 完了条件 |
| Negative | わざと壊して拒否を確認 |
| cockpit | ログイン後の**コックピット**表示モード（設定で切替）。simple ではない |
| アクティビティ | 「ようすを見る」内。監査/使用量は**内部タブ** |

### 0.4 進捗シート

```
日付開始:
事前ツール: [ ]
NT-1..3: 未/PASS/FAIL
NT-4: [x] 済
NT-5..7: 未/PASS/FAIL/DEFER
公開: まだ/承認/完了
```

### 0.5 困ったとき

- 秘密 → 「NT-? Step ? で詰まった」だけ（値なし）  
- 正本優先  
- UI 名: **まもる・整える** / **そだてる** / **ようすを見る** / **ひろげる** / **AIとはなす** / **ワールド** / **アクティビティ** / **SNS承認** / **コインとポイント** / **ワークフロー**
- **cockpit モード**: NT-3・NT-5 の大半の画面は cockpit 専用。切替: **まもる・整える → 設定 → インターフェース複雑度 →「コックピット」**（「シンプル」では Biome / アクティビティ / SNS承認 等が開けない）。**Consumer/Agency トグルとは別軸**（persona 切替ではサイドバーは出ない）。旧ビルドでは切替後にハードリロードが必要な場合あり（`ViewModeProvider` 導入後は即反映）

### 0.6 課金導線の正本（混同禁止）

| 導線 | NT-1 D3 合格根拠？ |
|------|-------------------|
| MC 内 Checkout + Webhook | **使える（本線）** |
| LP Payment Link（MESSAGING §9） | **使えない** |

---

## NT-1 — Stripe 本番反映（OP-057-R / R2-1）

**正本**: [`docs/operations/stripe-production-setup.md`](../operations/stripe-production-setup.md)  
**補助**: [`docs/operations/api_key_rotation.md`](../operations/api_key_rotation.md) §B GUI / §C CLI  
**Human コピペ正本（本節）**: 下の **実行順 0→A→B→C→D→Negative** を上から実行する。foolproof §2 H-1 は要約（コマンド再掲なし）。食い違い時は **本節優先**。  
**終わる状態**: 稼働 api-server が distroless（MC 配信可）＋ Vault に秘密 ＋ Webhook ＋ アプリ Checkout で Pro。compose に API キー代入なし。

### 実行順（この順を守る）

| 順 | Step | 内容 | スキップ条件 |
|----|------|------|--------------|
| **0** | Step 0.1〜0.5 | distroless を本番に載せる（pull / chown / **build** / **up -d** / 稼働確認） | 既に Step 0 DoD を証明できるときのみ |
| A | Step A | 秘密を Vault へ（GUI 推奨） | — |
| B | Step B | 非秘密 env（TEST_MODE + Price） | — |
| C | Step C | Webhook 7 + whsec + **restart**（注入再読込） | — |
| D | Step D | Positive（Checkout → Pro） | — |
| N | Negative | キー削除→拒否→復元 | — |

> [!IMPORTANT]
> **`restart` では Docker イメージは更新されない。**  
> compose を distroless に変えた直後・旧イメージのままのときは、**Step 0.3 の `build` ＋ `up -d` が必須**。  
> Step C の `restart` は **Vault 秘密の再注入**専用（イメージ差し替えではない）。

### どこで作業するか

| 作業 | 場所 | 対応 Step |
|------|------|-----------|
| イメージ再ビルド | 本番ホスト SSH（リポジトリ） | **0** |
| Stripe 秘密 | 本番 Vault（GUI 推奨） | **A** |
| TEST_MODE / Price | 本番ホスト env | **B** |
| VITE_STRIPE_PRICE_ID | MC ビルド（**任意**・未設定時エイリアス） | B 補足 |
| Webhook | Stripe Dashboard | **C** |

### 方針（必ず1つ・Step B で使う）

| 方針 | Dashboard | TEST_MODE | キー | Price |
|------|-----------|-----------|------|-------|
| **A** soft-launch | Test mode | `true` | `sk_test_` | test price |
| **B** go-live | Live mode | `false` | `sk_live_` | live price（例 `price_1TpXFpBcUTwo5TwLmK9SQbKL`） |

選んだ方針: [ ] A / [ ] B

### 事前チェック（Step 0 の前）

| # | 内容 | Yes |
|---|------|-----|
| P1 | Dashboard（Test/Live が方針どおり） | [ ] |
| P2 | 方針に合う Price ID を控えた（チャットに全文禁止） | [ ] |
| P3 | 本番ホストに `VAULT_SECRET` + **`VAULT_MASTER_PASSWORD`** | [ ] |
| P4 | compose に `KEY_PROXY_URL=http://key-proxy:9999` | [ ] |
| P5 | compose に `STRIPE_API_KEY=` **代入なし** | [ ] |
| P6 | compose の api-server が `docker/distroless.Dockerfile`（**ファイル**確認） | [ ] |

```bash
cd /path/to/aiome   # 本番ホスト上のリポジトリ
rg "STRIPE_API_KEY\s*=" docker-compose.production.yml || echo "OK: no assignment"
rg -n "dockerfile:.*distroless" docker-compose.production.yml
# 期待: api-server ブロック付近に docker/distroless.Dockerfile
```

`STRIPE_WEBHOOK_SECRET=${…}`（compose）は **空推奨**。値は Vault へ。

### Compose 方針（リポジトリ側は v1.5 で実施済・参照のみ）

| サービス | dockerfile | 理由 |
|----------|------------|------|
| **api-server** | `docker/distroless.Dockerfile` | MC `dist` を static 同梱 |
| key-proxy | `production.Dockerfile` | healthcheck = `nc` |
| samsara-hub | `production.Dockerfile` | healthcheck = `curl` |
| shadow-worker | `production.Dockerfile` | healthcheck = `CMD-SHELL` |

付帯（compose 記載済）: api-server `user: "65532:65532"` / `healthcheck.disable: true`。  
別系統（本 NT 対象外）: `docker-compose.cell.yml` / `commercial.yml` / `docker-publish.yml`。

---

### Step 0 — distroless イメージを本番に載せる（Vault GUI の前提）

**目的**: 稼働中の api-server が **MC 付き distroless** であること。これ無しでは Step A の GUI が開けない／白い画面になる。  
**場所**: 本番ホスト SSH。秘密は出さない。

> **FE / static 更新**: 本番 compose は `apps/api-server/static` を bind-mount する（ADR-055 実行前）。イメージ rebuild（Path A）だけでは UI は変わらない。ホスト static 同期（Path B）は [`MC_STATIC_DEPLOY.md`](MC_STATIC_DEPLOY.md) / [`mc_static_deploy_plan.md`](../roadmaps/mc_static_deploy_plan.md)（OP-087）。`static/` は gitignore 全無視。Human 許可後に `./scripts/sync_mc_static.sh`。

#### Step 0.1 git pull + compose の distroless 確認

```bash
cd /path/to/aiome
git status
git pull   # またはデプロイ手順どおりの取得。v1.5 以降の compose（distroless）が含まれること
rg -n "dockerfile:.*distroless" docker-compose.production.yml
```

- [ ] pull 完了  
- [ ] ファイル上 api-server = distroless（P6 再確認）

#### Step 0.2 chown -R 65532:65532 data/api

旧イメージは `1001:1001` だった。distroless nonroot は **65532**。書き込めないと起動後に DB/データエラーになる。

```bash
mkdir -p data/api
# 初回、または以前 1001 で動かしていた場合:
sudo chown -R 65532:65532 data/api
ls -ld data/api
# 期待: 所有者 uid/gid が 65532（表示は数字または nonroot）
```

- [ ] `data/api` が 65532 で**回帰可能な書き込み権限**を持つ  

#### Step 0.3 build api-server → up -d（必須）

`restart` では不十分。イメージを作り直すために **`build` ＋ `up -d` を必ず実行**する。

```bash
# 所要: 初回は長い（Rust + npm）。ログに秘密を貼らない
docker compose -f docker-compose.production.yml build api-server
docker compose -f docker-compose.production.yml up -d api-server
docker compose -f docker-compose.production.yml ps api-server
```

- [ ] `build` 成功（exit 0）  
- [ ] `ps` で api-server が Up（Restarting でない）  

Caddy 経由で外から見る場合、Caddy も既に Up であること。api-server だけ載せ替えたあとにブラウザをハードリロードする。

#### Step 0.4 稼働 Labels security.distroless=true + ブラウザで MC

ファイルの `rg`（P6）だけでは不十分。**今動いているコンテナ**を見る。

```bash
docker compose -f docker-compose.production.yml ps -q api-server | xargs docker inspect \
  --format 'Image={{.Config.Image}} User={{.Config.User}} Labels={{json .Config.Labels}}'
```

**PASS 条件（推奨は Labels）**:

| 確認 | PASS |
|------|------|
| Labels | `security.distroless` が `true`（distroless.Dockerfile の LABEL） |
| User | `65532:65532` または `nonroot`（旧 `1001:1001` だけなら **FAIL → 0.3 やり直し**） |
| ブラウザ | `https://<YOUR_DOMAIN>/` で MC ログイン画面または cockpit（真っ白・404 連続は FAIL） |

```bash
docker compose -f docker-compose.production.yml ps -q api-server | xargs docker inspect \
  --format '{{index .Config.Labels "security.distroless"}}'
# 期待出力: true
```

- [ ] 稼働 Labels `security.distroless=true`（または同等の PASS）  
- [ ] ブラウザで本番 MC が開く  

#### Step 0.5 失敗時の症状→対処表

| 症状 | 対処 |
|------|------|
| build 失敗 | ログ末尾（秘密マスク）を保存。ディスク不足・npm/cargo エラーを確認 |
| Up するが MC が白い | 旧イメージの可能性 → **0.3 再実行**。Caddy upstream 確認 |
| permission denied on data | **0.2 `chown` 再実行** |
| `security.distroless` が空 | 別 Dockerfile でビルドされている → compose の dockerfile 行と build 対象サービス名を確認 |

- [ ] **Step 0 DoD PASS**（これ以降 Step A へ）

- [ ] **（推奨）ホスト outbound 監視 ON** — Vault/秘密操作前に LuLu または Little Snitch。手順: [`DEV_HOST_EGRESS.md`](DEV_HOST_EGRESS.md)（OP-095）

---

### Step A — 秘密（推奨 GUI）

**前提**: Step 0 DoD PASS。

#### 経路 A（推奨）MC GUI

1. ブラウザで `https://<YOUR_DOMAIN>/`（**1420 ではない**）  
2. 管理者ログイン  
3. **まもる・整える → 設定** → ページ下部 **Abyss Vault シークレットマネージャ**  
4. `STRIPE_API_KEY` を設定（方針 A なら `sk_test_` / B なら `sk_live_`。チャット禁止）  
5. `STRIPE_WEBHOOK_SECRET` は Webhook 作成前なら後回し可 → Step C で必ず入れる  
6. 設定済み表示を確認  

（API は key-proxy 経由 = 本番と同じ Vault）

#### 経路 B（上級）CLI

同一 `ABYSS_VAULT_PATH` + `VAULT_MASTER_PASSWORD` + `CELL_ID` 必須。迷ったら GUI のみ。

```bash
export CELL_ID=cell-0
export VAULT_MASTER_PASSWORD='…'
export ABYSS_VAULT_PATH='/path/to/SAME/abyss_vault.db'
cargo run --bin abyss-vault -- status
cargo run --bin abyss-vault -- set STRIPE_API_KEY
cargo run --bin abyss-vault -- set STRIPE_WEBHOOK_SECRET
```

- [ ] 秘密格納（GUI または CLI。whsec は Step C 後でも可とメモ）

### Step B — 非秘密（方針で分岐）

**方針 A:** `STRIPE_TEST_MODE=true` + test Price（ホスト `STRIPE_PRICE_SUBSCRIPTION_MONTHLY`）  
**方針 B:** `STRIPE_TEST_MODE=false` + live Price（同上）  

**フロントの Price（重要）**:
- MC の `VITE_STRIPE_PRICE_ID` 未設定時はデフォルト `price_gold_monthly`（`config.ts`）
- api-server は `price_gold_monthly` を **ホストの `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` に解決**する（エイリアス）
- よって本番 Docker で VITE を焼き込まなくても、**ホスト Price が正しければ Checkout は動く**
- 厳密に同一文字列を焼き込む場合のみ、MC 再ビルド時に `VITE_STRIPE_PRICE_ID` を渡す（任意）

方針 B でホスト Price 空 → 起動拒否（正しい）。

- [ ] TEST_MODE 一致  
- [ ] ホスト Price 設定済（エイリアス経路で可）  

### Step C — Webhook

1. Developers → Webhooks（A なら Test mode ON）  
2. URL: `https://<YOUR_DOMAIN>/api/v1/commerce/webhook`  
3. イベント7: `checkout.session.completed` / `invoice.paid` / `invoice.payment_failed` / `customer.subscription.deleted` / `customer.subscription.updated` / `charge.dispute.created` / `checkout.session.expired`  
4. whsec を Step A と同じ経路で Vault  
5. `docker compose -f docker-compose.production.yml restart api-server`（**秘密の再注入**用。イメージ変更ではない。key-proxy 再起動は通常不要）  
6. ログに注入成功（キー全文なし）  

- [ ] 完了  

### Step D — Positive

| # | 操作 | PASS |
|---|------|------|
| D1 | 次のいずれかで Checkout 開始: **ヘッダー PlanBadge** / **ひろげる→コインとポイント→Pro にアップグレード** / 402 モーダル。返る `price_id` が実 Price（またはエイリアス解決後の実 ID） | OK |
| D2 | **アプリ Checkout** 1件完了（LP Payment Link 単独は不可） | Stripe 完了 |
| D3 | PlanBadge / サブスク状態で Pro | セルフホスト Pro |
| D4 | compose | キー代入なし |

### Negative

GUI で API キー削除 → 再起動 → 拒否 → **すぐ復元** → 再起動。  
（CLI 経路 B のみ: `delete` → 復元）

- [ ] 拒否確認  
- [ ] 復元済  

### 補足 OP-057-R (2)

コード完了済。D3 = 人間レビュー記録。

### 完了メモ

```
NT-1: 日付 / Step0 distroless稼働=PASS / Vault=GUI|CLI / 方針=A|B / Price末尾4 / Webhook7 / Pro unlock / Negative復元
→ OPEN OP-057-R (1) 更新可
```

---

## NT-2 — Quick Start 実走（G1 / R3-4）

**実走正本**: [`QUICK_START_VERIFICATION.md`](QUICK_START_VERIFICATION.md)（手順・DoD・Negative はそちら。ここへコピペしない）  
**実装・残リスク正本**: [`nt2_quickstart_unblock_plan.md`](../roadmaps/nt2_quickstart_unblock_plan.md)（コア実装済。フォローは **§8**）  
**終わる状態**: 新規 clone → Setup → ログイン → チャット（**公式** `docker-compose.quickstart.yml`）。

> **2026-07-13**: 公式 compose DoD PASS（API + R-B 代理）。OPEN **OP-078/077/079** 閉じ。詳細は unblock **§8**。  
> **Local LLM /reflexion フォロー**（Pattern B 実機・git 整理）: [`local_llm_ab_reflexion_plan.md`](../roadmaps/local_llm_ab_reflexion_plan.md) — **OP-080〜082**（NT-6 必須ブロッカー外）。

### 事前準備

- [ ] Docker 起動  
- [ ] 1420 空  
- [ ] 固定名コンテナなし  
- [ ] **実装ゲート I-1〜I-4 完了**（unblock 計画）または意図的に FAIL 記録  

```bash
lsof -i :1420 || echo "1420 OK"
docker ps --format '{{.Names}}' | grep -E '^aiome-(ollama|api-server|mc)$' || echo "OK: no name clash"
```

衝突時: `docker stop aiome-ollama aiome-api-server aiome-mc 2>/dev/null || true`

以降の Step 1〜6 / DoD / Negative / 記録は **QUICK_START_VERIFICATION.md** に従う。

---

## NT-3 — Biome 目視（OP-002）

**コードは触らない。**

| ファイル | 行 | 内容 |
|----------|-----|------|
| `BiomeCanvas.tsx` | 99 | `alpha: false` |
| `BiomeRenderer.tsx` | 187 | `alpha: false` |

### 前提

- [ ] **コックピット**モード: **まもる・整える → 設定 → インターフェース複雑度 →「コックピット」**（「シンプル」では「ワールド」が出ない）

### Step

1. MC を開く（ローカル or NT-2 quickstart）  
2. サイドバーセクション **そだてる** → **ワールド**（Biome）  
3. キャンバス背後の合成を見る  

| 判定 | 見た目 |
|------|--------|
| **PASS** | 不透明な灰色の板で全体が塗りつぶされず、下層の雰囲気が見える |
| **FAIL** | 灰色の長方形で下が完全に隠れる |

### Negative

DevTools で canvas を一時非表示 → 下層が見える → 元に戻す。

### 完了メモ

```
NT-3 / OP-002
日付:
結果: PASS / FAIL
ブラウザ:
```

PASS 後: Agent に「NT-3 PASS、OPEN の OP-002 を閉じて」→ OPEN 更新。

---

## NT-4 — やらなくてよい

2026-07-10 完了。回帰時のみ:

```bash
cd /path/to/aiome
cargo test -p api-server api_integration_tests::commerce -- --test-threads=1
cargo test -p api-server commerce_e2e_tests -- --test-threads=1
cargo test -p aiome-commerce -- --test-threads=1
```

---

## NT-5 — 証拠ビジュアル（OP-063）

**正本**: [`docs/marketing/MESSAGING.md`](../marketing/MESSAGING.md) §8  
**終わる状態**: 7 ファイル。秘密なし。**公開（R5-5）の前に必須。**

### 共通ルール

- [ ] 実データ  
- [ ] ダークテーマ  
- [ ] **1920×1080 以上**  
- [ ] 個人情報・キー・トークン・`.env` が映らない  
- [ ] OGP 再生成不要  

### 保存先例

```text
docs/assets/evidence/YYYY-MM-DD/
  01-quickstart.gif
  02-audit.png
  03-buzz-approval.png
  04-nurture-economy.png
  05-workflow-builder.png
  06-agent-diorama.png
  07-prompt-stats.png
```

### 画面の開き方（重要）

ログイン後 **コックピット必須**（切替: **まもる・整える → 設定 → インターフェース複雑度 →「コックピット」**。「シンプル」では #2〜5,#7 が開けない）。サイドバーはグループ付き。

| # | ファイル | 撮る内容 | **正確な開き方** |
|---|----------|----------|------------------|
| 1 | `01-quickstart.gif` | Setup→Playbook→Home 約30秒 | 初回 Wizard。無理なら同等フローを録画 |
| 2 | `02-audit.png` | 監査ログ | **ようすを見る → アクティビティ** を開く → 画面上部タブ **「監査ログ」**（`data-testid=activity-tab-audit`） |
| 3 | `03-buzz-approval.png` | 承認キュー | **ひろげる → SNS承認** |
| 4 | `04-nurture-economy.png` | エコノミー | **ひろげる → コインとポイント** |
| 5 | `05-workflow-builder.png` | ワークフロー | **ひろげる → ワークフロー** |
| 6 | `06-agent-diorama.png` | チャット+アバター | **ホーム**セクションの **AIとはなす**（Diorama が見える状態） |
| 7 | `07-prompt-stats.png` | LLM 統計 | **ようすを見る → アクティビティ** → 上部タブ **「使用量」**（`activity-tab-usage`） |

**やってはいけない誤解**:

- サイドバーに「監査ログ」「プロンプト統計」が**独立項目として無い**のは仕様（U6-5）。**アクティビティの中のタブ**で撮る。  
- `activeTab=audit` / `prompt-stats` は内部互換用。通常 Human は触らなくてよい。

| # | 撮った | 1920+ | 秘密なし |
|---|--------|-------|----------|
| 1 GIF | [ ] | [ ] | [ ] |
| 2〜7 | [ ]×6 | [ ] | [ ] |

### Negative

キーが映ったファイルは **破棄**。push しない。

### 完了メモ

```
NT-5 / OP-063
日付:
パス:
結果: 7/7 PASS / 欠番:
※ R4-2（LP 埋込）は別。撮影完了で本 NT PASS 可
→ OPEN OP-063 を「撮影完了・R4-2 待ち」に更新可
```

---

## NT-6 — リリースゲート（R5）

**前提**: NT-1・2・3 PASS。**公開直前に NT-5=7/7。**  
**実行場所**: preflight は **開発マシンの clone**（本番 SSH ではない）。  
**Agent**: 「**NT-6 を実行しろ**」

正本: [`OPERATIONS_MANUAL.md`](OPERATIONS_MANUAL.md) §8 / [`.agent/workflows/release-preflight.md`](../../.agent/workflows/release-preflight.md) / [`release_master_plan.md`](../roadmaps/release_master_plan.md) R5

### パート A — ステップ 0（必須）

Issue またはリリース草案に実体を書く（「リンク」空欄は FAIL）:

```
ロールバック:
- Feature Flag / 課金停止:
- git revert:
- DB: docs/operations/BACKUP.md 等の復元手順:
```

- [ ] 記載済  

### パート B — §8

スキップ可: G1←NT-2 / R2-1←NT-1 / OP-012・014 は過去 PASS 信頼時。  
スキップしない代表: `VAULT_SECRET` / `VAULT_MASTER_PASSWORD` / `NURTURE_API_URL`+`NURTURE_INTERNAL_SECRET` / `A2A_NODE_TOKEN` / 該当する Polar・Tauri 等は §8 全文で確認。

### パート C — preflight

`cargo` / `gitleaks` / `python3` 確認。1件 FAIL → 公開中止。  
番号は [release-preflight.md](../../.agent/workflows/release-preflight.md) と対応（0 / 0.5 / 1… / 5.5 / 6 / 7 / 7.5 / 8）。

```bash
cd /path/to/aiome
# 0 = パート A（ロールバック文を目視）
# 0.5 DAG
python3 scripts/enforce_dag.py
# 1 gitleaks
gitleaks detect -v 2>&1 | tail -5
# 2 衛生
echo "=== .DS_Store ===" && (git ls-files | grep -i DS_Store || echo "OK") && \
echo "=== node_modules ===" && (git ls-files | grep node_modules/ | head -3 || echo "OK") && \
echo "=== .env files ===" && (git ls-files | grep -E '\.env$' || echo "OK") && \
echo "=== memory files ===" && (git ls-files | grep -E '^memory/|MEMORY\.md|\.agent.*/memory/' || echo "OK") && \
echo "=== database files ===" && (git ls-files | grep -E '\.(sqlite|sqlite3|db)$' || echo "OK") && \
echo "=== build artifacts ===" && (git ls-files | grep -E '\.(dylib|so|dll|node|tgz)$' || echo "OK") && \
echo "=== strategy docs ===" && (git ls-files | grep -iE 'master_blueprint|vision_manifesto|pitch_deck|buyout|valuation' | grep -vi 'evaluation' || echo "OK") && \
echo "=== backup files ===" && (git ls-files | grep -E '\.bak$|\.orig$|\.swp$' || echo "OK") && \
echo "=== states/logs ===" && (git ls-files | grep -E '^states/|^logs/' || echo "OK")
# 3 ローカルパス
git ls-files | grep -v node_modules | xargs grep -rl "/Users/" 2>/dev/null || echo "OK: No local paths found"
# 4 誤 URL
grep -rn "google/antigravity" README.md README_en.md 2>/dev/null || echo "OK: No wrong URLs"
# 5 cargo check
cargo check --workspace 2>&1 | tail -3
# 5.5 ignored
cargo test --workspace -- --ignored --skip sandbox --skip vendor 2>&1 | tail -10
# 6 サイズ — PASS: vendor除外≤2500 かつ size≤75MB
echo "Tracked files (excl. vendor/):" && git ls-files | grep -vc '^vendor/' && \
echo "Tracked files (all):" && git ls-files | wc -l && \
echo "Estimated size:" && git ls-files -z | xargs -0 du -ch 2>/dev/null | tail -1
# 7 = パート D（手動・GitHub About）
# 7.5 Unreleased 行数 — 200超なら R5-2
awk '/^## \[Unreleased\]/{f=1;next} /^## \[/{f=0} f' CHANGELOG.md | wc -l
# 8 LICENSE ↔ README バッジ一致（※ LICENSE 本文中の "Apache" は Change License。1行目の BUSL を見る）
head -1 LICENSE
grep -n "BUSL\|Business Source\|License-BUSL\|License-BSL" README.md | head -5
```

**ステップ 6 判定**: `vendor/` 除外の追跡ファイル ≤ **2500** かつ `du` 合計 ≤ **75MB**。`vendor/oxilean-kernel` は path 依存のため件数除外。超えたら公開中止（Agent に縮小を依頼）。  
**ステップ 8 判定**: `head -1 LICENSE` が `Business Source License` で、README バッジが BUSL/BSL。`grep -o Apache LICENSE | head -1` は **使わない**（Change License 行に誤ヒットする）。

### パート D — GitHub About（MESSAGING §7）

Description:
```text
The sovereign OS for autonomous AI agents — self-hosted, fully auditable, with a built-in agent economy. Own it, govern it, let it earn.
```
- Website（例 https://aiome.dev）  
- Topics: `ai-agents` `autonomous-agents` `self-hosted` `sovereign-ai` `mcp` `rust` `agent-economy` `local-first` `ai-os` `tauri`  
- Social preview: 既存 OGP  

- [ ] Description / Website / Topics / Preview  

### パート E — R5-2〜5

| ID | あなた | Agent |
|----|--------|-------|
| R5-2 | Unreleased>200 | 「R5-2 を実装しろ」 |
| R5-3 | パート A のロールバック文 | （Human 完了で足りる） |
| R5-4 | preflight PASS 後 | 「docs-sync を実行しろ」 |
| R5-5 | **「公開してよい」**（その直前に NT-5=7/7） | Release/タグ |

公開ゲート: C PASS（ステップ6閾値含む）+ §8 + **NT-5=7/7** + R5-3 + 明示承認。

### Negative

gitleaks/DAG/ignored が1件 NG → 公開中止 → 直して C から再実行。

```
NT-6: 日付 / 開発機 / preflight / NT-5 / R5-5
→ OPEN OP-070 更新可
```

---

## NT-7 — ベータ獲得（OP-064・任意）

公開ブロッカー外。

1. 招待チャネル決定  
2. 下の禁止を守る  
3. 名簿は **private**（連絡先を git に入れない）  
4. 5〜10 人がログイン〜チャット相当  

### 禁止表現（MESSAGING §10 抄録）

1. 未実装を実装済みと書く（Coming Soon なし）  
2. 架空の実績・ロゴ・推薦・ユーザー数  
3. 旧手数料 25%/10%  
4. 「世界初」「完全に安全」等  
5. 「146,000行」を主訴求  
6. 収益保証と読める表現  

```markdown
| # | 表示名 | 連絡先（非公開） | 実名許諾 | 一言 | 日付 |
|---|--------|------------------|----------|------|------|
| 1 |        |                  | Y/N     |      |      |
```

```
NT-7 / OP-064
日付:
人数:
Testimonial: 0 / K
結果: PASS / DEFER
```

---

→ OPEN OP-064 を更新可（任意完了 / DEFER）。

## 完了宣言テンプレ

```
Human Public Beta ランブック完了報告（v1.6）
NT-1..7: …
日付:
```

---

## 付録 A — 「今どれ？」

| 状況 | 次 |
|------|-----|
| Stripe 未 | ~~NT-1~~ ✅ 2026-07-14（方針 A・`app.aiome.dev`） |
| Quick Start 未 | ~~NT-2~~ ✅ |
| Biome 未 | ~~NT-3~~ ✅ |
| スクショ未 | ~~NT-5~~ ✅ 2026-07-14（**R4-2 組込済**） |
| NT-6 PASS・公開後 | 任意 NT-7 / Stripe 方針 B |
| 公開 | ~~NT-6 PASS + NT-5=7/7 +「公開してよい」~~ ✅ `v1.2.0` |

## 付録 B — 突合根拠

| 項目 | 根拠 |
|------|------|
| compose L101–106 | キー禁止 + whsec パススルー |
| KEY_PROXY / VAULT_MASTER | compose L49–52, L83 |
| Vault GUI → key-proxy | `routes/vault.rs` / `VaultSecretsManager.tsx` |
| Biome alpha | `BiomeCanvas.tsx:99` / `BiomeRenderer.tsx:187` |
| Activity タブ | `ActivityView.tsx` L29–33 |
| quickstart 固定名 | `aiome-ollama` / `aiome-api-server` / `aiome-mc` |
| Webhook 7 | stripe-production-setup §3 |
| preflight | release-preflight.md |
| 本番 MC 配信 | Caddy → api-server:3015。compose **api-server = distroless**（v1.5 実施済） |
| 本版 | **v1.6** |

---

*Human 実行専用（v1.6）。Agent は秘密入力・compose へのキー追加・勝手な公開をしない。*
