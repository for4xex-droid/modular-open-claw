# 直近アクション開発計画 — Public Beta 閉ループ（v5.1）

- **作成日**: 2026-07-10（v1） / **改訂**: v2→v3→v4→v5→**v5.1**（/reflexion ×4 確認 PASS）
- **根拠**: `/biz-value` + `OPEN.md` + `release_master_plan.md` + 実コード照合（`/perfect-plan` ×5）
- **正本関係**: タスク ID は `OPEN.md`。実行順序・DoD は本計画。二重管理しない。
- **スコープ宣言**: **市場接触と本番課金ゲートのみ**。Wave 3・メタバース・CCI News・OP-011 は本計画外。

---

## 0. 結論（何をやるか / やらないか）

### やる（優先順）

| 順 | ID | 内容 | 担当 | コード状態 |
|---|---|---|---|---|
| 1 | **NT-1** = R2-1 = OP-057-R (1) | 本番 Stripe 秘密情報 + 非秘密設定の反映 | **Human** | 手順書 ✅ Vault 正本化済み・**反映は Human** |
| 2 | **NT-2** = R3-4 = G1 | Quick Start 実走 | **Human** | チェックリスト DONE |
| 3 | **NT-3** = OP-002 = R1-16 | Biome `alpha:false` 目視 | **Human** | コード DONE |
| 4 | **NT-4** = R2-4 = OP-013 | Stripe E2E 実行・台帳クローズ | Main | ✅ **2026-07-10 完了**（28+2+65 PASS） |
| 5 | **NT-5** = R4-1 = OP-063 | 証拠ビジュアル 7点 + GIF | **Human** | OGP DONE / 撮影 MISSING |
| 6 | **NT-6** = R5 | リリースゲート一式 | Main + Human | ワークフロー DONE |
| 7 | **NT-7** = R4-3 = OP-064 | ベータ 5–10 人（任意） | **Human** | テンプレ MISSING |
| — | **NT-0b** | compose に非シークレット 2 変数のみパススルー | Main | ✅ **2026-07-10 完了** |

### 後続（NT にしない）

| ID | 内容 | 再発明防止 |
|---|---|---|
| R4-2 | LP へ撮影素材組込 | 既存 LP 配置のみ |
| R4-4 | GitHub About | ≡ release-preflight ステップ 7 |
| R4-5 | Show HN / PH 文案 | 公開ブロッカー外 |

### やらない（再発明・誤診の撤回含む）

| 除外 | 理由 |
|---|---|
| **~~NT-0: compose に `STRIPE_API_KEY` を追加~~** | **v4 誤診。撤回。** 本番は `fetch_and_inject_secrets` + AbyssVault（`ALLOWED_VAULT_SECRETS` に `STRIPE_API_KEY` 既存）。compose コメントも「No API keys in environment」。**Zero-Trust 違反の再発明** |
| OP-057-R (2) コード再実装 | `stripe.rs` L250–270 完了 |
| ProUpgradeModal / checkout 新設 | `App.tsx` L753 + checkout-session API 既存 |
| 新規リリース checklist | 既存 3 文書の実行順のみ |
| `cargo test --lib`（api-server） | bin のみ |
| OP-027 / Wave 3 / メタバース / OP-011 / waitlist / OGP 再生成 | スコープ外 or 完了 |
| Nurture `STRIPE_SECRET_KEY` リネーム | 別系統 |

### ID 重複マップ

```
OP-057-R (1)  ≡  R2-1  ≡  NT-1
OP-013        ≡  R2-4  ≡  NT-4
OP-002        ≡  R1-16 ≡  NT-3
OP-063        ≡  R4-1  ≡  NT-5
OP-064        ≡  R4-3  ≡  NT-7
G1            ≡  R3-4  ≡  NT-2
R5            ≡  NT-6
R4-4          ≡  release-preflight ステップ 7
（旧 NT-0 の STRIPE_API_KEY compose 追加は廃止）
```

---

## 1. 実コード照合サマリ（2026-07-10 v5）

| 資産 | パス | 状態 | 注記 |
|---|---|---|---|
| Vault 注入 | `libs/shared/src/security.rs` L296–315, L318– | ✅ **本番正経路** | `STRIPE_API_KEY` / `STRIPE_WEBHOOK_SECRET` が whitelist |
| Vault CLI | `docs/operations/api_key_rotation.md` | ✅ | `abyss-vault set …` |
| compose api-server | `docker-compose.production.yml` L73–106 | ✅ | 「No API keys」+ `KEY_PROXY_URL` / `VAULT_SECRET`。Webhook env フォールバック維持 |
| compose 非シークレット Stripe | 同 L104–105 | ✅ **NT-0b 完了** | `STRIPE_TEST_MODE` / `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` パススルー済み（未設定時は `""` → preflight が本番起動拒否） |
| `stripe-production-setup.md` | docs/operations | ✅ **Vault 正本化済み** | 秘密=Vault / 非秘密=env。`.env` 直書きは非推奨フォールバック |
| preflight scrub | `preflight.rs` L124–125 | ✅ | 注入後に `STRIPE_API_KEY` を scrub |
| Commerce | `core_services.rs` / `factory.rs` | ✅ | 注入済みキーで Stripe engine |
| ProUpgradeModal / webhook unlock / Biome / OGP / テスト | 各所 | ✅ | v3–v4 どおり |
| NT-4 コマンド | bin フィルタ | ✅ | `--list` 確認済み |

### 1.1 秘密情報の正しい到達経路（車輪の再開発防止）

```
Human: abyss-vault set STRIPE_API_KEY / STRIPE_WEBHOOK_SECRET
        ↓
key-proxy（Vault）
        ↓ 起動時 fetch_and_inject_secrets()
api-server プロセス環境（その後 scrub_env）
```

**compose の `api-server.environment` に `STRIPE_API_KEY` を足さない。**  
（v4 NT-0 はこれを推奨していたが、既存 Zero-Trust と矛盾 → **廃止**）

### 1.2 非シークレット（Vault に無いもの）

| 変数 | Vault? | 反映方法 |
|---|---|---|
| `STRIPE_API_KEY` | ✅ | `abyss-vault set` |
| `STRIPE_WEBHOOK_SECRET` | ✅ | `abyss-vault set`（compose にもフォールバックあり） |
| `STRIPE_TEST_MODE` | ❌ | ホスト env / compose パススルー（NT-0b 済み） |
| `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` | ❌ | 同上（未設定=`""` → preflight が production で起動拒否） |
| `VITE_STRIPE_PRICE_ID` | ❌ | MC ビルド時 | 

### 1.3 テストモード vs 本番

| 目的 | 秘密 | 非秘密 |
|---|---|---|
| ローカル test | `.env` の `sk_test_` 可 | `STRIPE_TEST_MODE=true` |
| 本番 compose | **Vault のみ** | `false` + Price ID（NT-0b 済みパススルー） |

---

## 2. フェーズ実行計画

### Phase A — Human（NT-1 を Vault 正本で）

#### NT-1 DoD（Human）— 手順書の正本を二段に

**A. 秘密（必須）** — `docs/operations/api_key_rotation.md` 準拠  
1. `abyss-vault set STRIPE_API_KEY`（live）  
2. `abyss-vault set STRIPE_WEBHOOK_SECRET`  
3. api-server **再起動**（注入は起動時のみ）  
4. 起動ログで key-proxy 注入成功を確認（値は出さない）

**B. 非秘密（必須）**  
5. `STRIPE_TEST_MODE=false`  
6. `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` + MC `VITE_STRIPE_PRICE_ID`（同一）  
7. Dashboard Webhook 登録  
8. 本番 API が実 Price ID を返す + **テスト決済 1 件で Pro unlock**

**C. ドキュメント衛生**  
9. `stripe-production-setup.md` Vault 正本化 → ✅ 2026-07-10 完了

> 🔐 commerce/webhook **コード変更禁止**。compose に API キーを足さない。

#### NT-2 / NT-3 / NT-5 / NT-7
v4 と同じ（Quick Start / Biome 目視 / ショット / 任意ベータ）。

```
NT-1 ──┬──→ NT-6
NT-2 ──┤
NT-3 ──┘
NT-5 → R4-2
NT-7（任意）
```

---

### Phase A0b — NT-0b（✅ 2026-07-10 完了）

`docker-compose.production.yml` api-server に `STRIPE_TEST_MODE` / `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` パススルー済み。`STRIPE_API_KEY` は追加していない。

---

### Phase B — Agent

#### NT-4（✅ 2026-07-10 完了・再実行不要）

記録用コマンド（再実行は回帰時のみ）:

```bash
cargo test -p api-server api_integration_tests::commerce -- --test-threads=1
cargo test -p api-server commerce_e2e_tests -- --test-threads=1
cargo test -p aiome-commerce -- --test-threads=1
```

結果: commerce 28 / e2e 2 / aiome-commerce 65 PASS → OPEN OP-013 ✅。

#### NT-6（未実施・明示「実行しろ」後）

| 順 | 正本 | 重複ルール |
|---|---|---|
| 1 | OPERATIONS_MANUAL §8 | NT-1/2 済みの R2-1/G1 行はスキップ |
| 2 | release-preflight | R4-4 ≡ ステップ 7 |
| 3 | release_master R5-2〜5 | |

`preflight.md`（コード変更前用）は使わない。

---

### Phase C — ドキュメント衛生

| 項目 | 状態 |
|---|---|
| v3: `--lib` 修正 / R4 非 NT | ✅ |
| v4: NT-6 スキップ規則 | ✅ 維持 |
| **v5: NT-0（API キー compose）撤回** | ✅ 本改訂 |
| `stripe-production-setup.md` Vault 正本化 | ✅ **2026-07-10 完了** |
| NT-0b（非秘密パススルー） | ✅ **2026-07-10 完了** |
| NT-4 OP-013 E2E | ✅ **2026-07-10 完了** |

---

## 3. Safety-Critical 境界

| 触ってよい | 触ってはいけない |
|---|---|
| `abyss-vault set`（Human） | `commerce_webhook/stripe.rs` |
| 既存テスト実行 | auth / key-proxy **ロジック**変更 |
| 非秘密の compose パススルー（NT-0b） | **compose への `STRIPE_API_KEY` 追加** |
| stripe-production-setup.md の Vault 追記 | Nurture キーリネーム / waitlist 復活 |

---

## 4. 成功基準

1. NT-1: Vault に Stripe 秘密あり + 再起動後に実 Price ID + テスト決済 Pro unlock  
2. NT-2→G1 / NT-3→OP-002 / NT-4→OP-013  
3. NT-5→証拠 / NT-6→公開可 /（任意）NT-7→V-3  

### P0/P1

| OPEN | 本計画 |
|---|---|
| OP-002 | NT-3 |
| OP-070 | NT-1/2/5/6 |
| OP-011 | 除外 |
| OP-013 | NT-4 |
| OP-057-R | NT-1（秘密=Vault、非秘密=env/NT-0b） |

---

## 5. /perfect-plan 検証結果（v5）

### Gate 1: 構造スキャン
- ✅ Vault whitelist / `fetch_and_inject_secrets` / compose「No API keys」が実在。
- ✅ v4 NT-0（compose へ API キー追加）は撤回済み。
- ✅ `stripe-production-setup.md` Vault 正本化・NT-0b 非秘密パススルー・NT-4 E2E 完了（2026-07-10）。

### Gate 2: 要件カバレッジ
- §2 経済: Vault + 既存コード。新規 commerce なし。
- §4 セキュリティ: 秘密を compose に戻さない方針で整合。

### Gate 3: 依存・波及
- ✅ 秘密経路は key-proxy 再起動のみ。
- ✅ NT-0b は非秘密 2 行に限定（完了）。

### Gate 4: 悪魔の弁護人
1. **最悪**: Vault 未設定のまま compose にキーを足して平文拡散 → 禁止を維持。
2. **誤前提（v4）**: 「compose に無い = バグ」→ 秘密は意図的除外。
3. **やらないメリット**: 追加機能・Wave 3 は V-3 を動かさない。

### Gate 5: 実行順序
- ✅ NT-1（Vault・Human）→ 再起動 → 決済確認 → NT-6。NT-2/3/5 並列。NT-4 ✅。

### 判定
- [x] ✅ **PASS** — Agent 実装分（docs / NT-0b / NT-4）完了。残は Human ゲート（NT-1/2/3/5/6/7）。

---

## 6. 次のユーザー指示テンプレ

```
NT-1 を進める（Vault に Stripe 秘密を入れる）
```
```
NT-2 を進める（Quick Start 実走）
```
```
NT-3 を進める（Biome 目視）
```
```
NT-6 を実行しろ（release ゲート）
```