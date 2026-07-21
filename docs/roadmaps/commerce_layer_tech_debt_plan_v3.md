# Commerce レイヤー技術負債解消計画 v3.4（/perfect-plan 三度目検証済 + C/D ゲート再定義）

> **作成**: 2026-07-13（ユーザー案 v3）  
> **改訂**: v3.2 → **v3.3**（日次/月次 0 分離）→ **v3.4**（2026-07-20: C/D ゲートを Q2+SC に再定義。Federation transport=ADR-053 済みは非ブロッカー）  
> **ステータス**: B/A ✅。C/D 実行正本: [`op083_cd_x402_plan.md`](op083_cd_x402_plan.md)  
> **正本 ID**: OPEN **OP-083**（B→A 先行、C/D=Q2+SC）  
> **非目標（当時）**: compose への Stripe 秘密追加、Nurture `STRIPE_SECRET_KEY` リネーム、`keyring` 導入、AiomeCoin 置換、**月次上限の新機能再発明（ADR-050 / OP-059 済）**、**日次 0 を無制限に変えること**、**オンチェーン実送金 broadcast（C スコープ外）**。※ OP-011（無償 KC マーケット S2S）は **✅ 2026-07-22**（有償チャージは非対象のまま）

---

## 0. /perfect-plan 判定

### 0.1〜0.2（v3.1 / v3.2 要約）

構造・GiftEngine 兄弟 trait・ADR-050 硬化・verify→Fiat・CoinWallet 非変更は v3.2 までで確定。

### 0.3 v3.3（再々検証 MUST-FIX）

| # | v3.2 の欠陥 | v3.3 修正 |
|---|-------------|-----------|
| **MF-1** | §2.1 が単一 `effective_limit(…)` で **日次も 0=無制限**と書いており、interceptor の raw `min`（0=全拒否）・R4 と **計画内矛盾**。`wallet=0` や `policy=0` でリグレッション | **日次 / 月次で関数を分離**（§2.1）。日次は現行 raw `min` を維持 |
| SF-1 | 表示の 7 箇所目欠落 | `nurture-api` `daily-stats`（policy-only）を集約対象に追加 |
| SF-5 | `daily_spend_limit` の 0 意味が policy 未注釈 | Phase 2 で `EconomyPolicy` に日次 0=全拒否をコメント追記 |

**総合**: ✅ **PASS** — MF-1 反映後、実装可能。推奨着手 **OP-083-B**。

### 0.4 v3.4（C/D ゲート再定義 2026-07-20）

| # | 欠陥 | 修正 |
|---|------|------|
| **MF-F** | 「Federation 後」が ADR-053 transport・OP-020 P5・Q2 を混同 | C/D 着手条件 = **Q2（base-sepolia + `X402_RPC_URL`）+「OP-083-Cn を実装しろ」**。ADR-053 は前提済・非ブロッカー |
| — | 実行手順の正本が commerce v3 に埋もれる | [`op083_cd_x402_plan.md`](op083_cd_x402_plan.md) を C/D 実行正本とする |

---

## 1. 構造問題（D-1〜D-7）

| ID | 問題 | アンカー | 深刻度 | 照合 |
|----|------|----------|--------|------|
| **D-1** | `CommerceEngine` 肥大 | `libs/aiome-contracts/src/commerce.rs` L17–158 | 🟡 | ✅ 27 メソッド |
| **D-2** | x402 署名モック | `aiome-commerce/src/x402.rs` L84–86 | 🔴 | ✅ |
| **D-3** | `wallet/mod.rs` 空 | ヘッダのみ | 🔴 | ✅ |
| **D-4** | Negotiator 未配線 | Factory=Stripe/Polar/Mock；apps 呼出 0 | 🔴 | ✅ |
| **D-5** | u64 vs U256 | Engine vs x402 | 🟡 | ✅ |
| **D-6** | Nurture sealed 6 | checkout/stake/slash/sub×2/portal | 🟡 | ✅ |
| **D-7** | aiome-node スタブ | `StubCommerceEngine` | 🟡 | ✅ |

### 1.1 Phase 1 設計: supertrait（GiftEngine との差）

| パターン | 実例 | AppState | 採用理由 |
|----------|------|----------|----------|
| **兄弟 trait** | `GiftEngine` | `Arc<dyn GiftEngine>` **別フィールド** | ギフトは Commerce とライフサイクル分離 |
| **supertrait** | **本計画** | `Arc<dyn CommerceEngine>` **維持** | checkout/portal は既存ルートが同一 dyn を呼ぶ。AppState 二重化は波及大 |

```rust
#[async_trait]
pub trait FiatPaymentRails: Send + Sync {
    async fn create_checkout_session(...) -> Result<String, AiomeError>;
    async fn create_portal_session(...) -> Result<String, AiomeError>;
    async fn create_subscription(...) -> Result<String, AiomeError>;
    async fn cancel_subscription(...) -> Result<(), AiomeError>;
    fn verify_signature(&self, payload: &str, sig_header: &str) -> Result<(), AiomeError>;
}

#[async_trait]
pub trait Web3PaymentRails: Send + Sync {
    async fn stake(...) -> Result<(), AiomeError>;
    async fn slash(...) -> Result<(), AiomeError>;
}

#[async_trait]
pub trait CommerceEngine: FiatPaymentRails + Web3PaymentRails + Send + Sync {
    // コア: balance / escrow / transfer / deduct / points / history / wishlist
    // get_subscription_status: デフォルト Ok(None) 可 — auth/expression 依存のためコア残留
}
```

**却下**: GiftEngine 流の `Arc<dyn FiatPaymentRails>` 追加 — `routes/commerce.rs`・webhook・bootstrap の書き換えが過大（車輪ではなくコスト過大）。

### 1.2 impl 波及 **8 型**

| # | 型 | ファイル |
|---|-----|----------|
| 1 | `MockCommerceEngine` | `libs/aiome-commerce/src/mock.rs` |
| 2 | `StripeCommerceEngine` | `libs/aiome-commerce/src/stripe/mod.rs` |
| 3 | `PolarCommerceEngine` | `libs/aiome-commerce/src/polar.rs` |
| 4 | `NurtureCommerceBridge` | `commercial/.../commerce_impl.rs` |
| 5 | `StubCommerceEngine` | `apps/aiome-node/src/main.rs` |
| 6 | `MockCommerceEngineForMarketplace` | `libs/infrastructure/src/lora_marketplace.rs` |
| 7 | browser red-team mock | `libs/infrastructure/tests/browser_red_team_tdd.rs` |
| 8 | api integration mock | `apps/api-server/src/api_integration_tests/common.rs` L114 |

`CommerceEngineFactory` は **impl ではない**（生成のみ）。

---

## 2. Phase 2 — spend_guard（OP-083-B）★ 最優先

### 2.0 位置づけ（重複排除）

- **ADR-050 / OP-059** で月次上限は **実装済**。本 Phase は **新機能ではない**。
- 目的: (1) `effective_spend_limit` **二重定義の排除** (2) escrow 日次の **wallet 無視バグ修正** (3) 表示 API と enforcement の式統一。

### 2.1 既存資産（触らない / 流用）

| 資産 | 扱い |
|------|------|
| `CoinWallet::check_daily_limit` / `check_monthly_limit` | **置換しない**。ウォレット単体。日次は limit=0 で拒否し得る（既存）。Phase 2 で変更 **禁止** |
| `AiomeCoin` / `CreatorPoints` | 触らない |
| `EconomyPolicy.daily_spend_limit` / `monthly_spend_limit` | 合成入力として使用（再発明しない） |
| `effective_spend_limit` ×2 | → `nurture_core::spend_guard` へ移動 |

**新規**: `commercial/libs/nurture-core/src/spend_guard.rs`（`lib.rs` に `pub mod spend_guard`）

```text
// 日次 — 現行 interceptor / commerce_impl の raw min を維持（0 ≠ 無制限）
effective_daily_limit(wallet_limit, policy_limit) -> u64
  // = wallet_limit.min(policy_limit)
  // 例: (10000, 0) → 0 → 正の支出拒否 / (0, 10000) → 0 → 拒否

// 月次 — 既存 effective_spend_limit と同一（0 = 無制限）
effective_monthly_limit(wallet_limit, policy_limit) -> u64
  // (0,0)→0 / (w,0)→w / (0,p)→p / (w,p)→min

check_spend_limits(wallet, policy, amount) -> Result<(), NurtureError>
  // 日次: effective_daily を超えれば DailyLimitExceeded（0 でも amount>0 なら拒否）
  // 月次: effective_monthly > 0 のときのみ検査
```

| セマンティクス | 日次 | 月次 |
|----------------|------|------|
| limit `0` の意味 | **上限ゼロ（正の支出拒否）** | **無制限（検査スキップ）** |
| `EconomyPolicy` default | `daily_spend_limit: 10_000` | `monthly_spend_limit: 0` |
| 根拠 | interceptor L162 raw `min` | interceptor L198 + `effective_spend_limit` |

- **適用範囲**: interceptor / commerce_impl / nurture-api daily-stats の **policy×wallet 合成**のみ。
- **非適用**: `CoinWallet::spend` 内部の check_*（変更禁止）。
- **禁止**: 日次を月次と同型の「0=無制限」に統一すること（リグレッション）。

### 2.2 集約 **7 箇所**（v3.2 の 6 + 表示ずれ）

| # | 場所 | 種別 | 現状 |
|---|------|------|------|
| 1 | `interceptor.rs` L162–198 | enforcement | 日次 raw min / 月次 effective |
| 2 | `commerce_impl` `escrow_create` L161 | enforcement | **日次=policy のみ** ⚠️ |
| 3 | `commerce_impl` `deduct_generation_cost` | enforcement | OK 寄り |
| 4 | `commerce_impl` `validate_activity` | enforcement | OK 寄り |
| 5 | `commerce_impl` `get_monthly_limit` L79–95 | 表示 | インライン重複 → `effective_monthly_limit` |
| 6 | `commerce_impl` `get_daily_limit` L46–55 | 表示 | raw min |
| 7 | `nurture-api` `daily-stats`（`balance.rs` L49） | 表示 | **policy のみ**（wallet 無視）→ Stripe `get_daily_limit` もここ経由 |

### 2.3 DoD

- [ ] `spend_guard` 単体: 日次 `(w,0)→0` / `(0,p)→0`；月次 `(w,0)→w`；overflow
- [ ] Negative: escrow が **wallet.daily_limit** を尊重
- [ ] Negative: `daily_spend_limit=0` または `wallet.daily_limit=0` で正の支出が **拒否**（無制限化していないこと）
- [ ] Positive: interceptor 月次回帰；`daily-stats` が `effective_daily_limit` を返す
- [ ] ローカル `fn effective_spend_limit` 削除（2 ファイル）
- [ ] `EconomyPolicy.daily_spend_limit` に「0 = 全拒否（月次の unlimited とは別）」コメント
- [ ] **`CoinWallet::check_*` の差分ゼロ**

---

## 3. Phase 1 — トレイト再設計（OP-083-A）

Phase 2 完了後。

1. §1.1 の supertrait + `verify_signature` → Fiat。
2. **8 impl** 更新（Nurture Fiat/Web3 は sealed Err 維持）。
3. `api_integration_tests/common.rs` + `commerce` 統合テストを DoD。
4. webhook（`verify_signature`）経路のコンパイル確認。
5. aiome-node ボイラープレート短縮（任意）。
6. `libs/core/src/commerce.rs` 再 export 確認。

**ゲート**: 「Commerce v3 Phase 1 を実装しろ」。

---

## 4. Phase 3 — x402（OP-083-C）⚠ Q2 + SC

**実行正本**: [`op083_cd_x402_plan.md`](op083_cd_x402_plan.md)

| 出典 | 方針 |
|------|------|
| ADR-053 | Federation **transport** は済み。C の技術前提ではない |
| Q2 | Network=`base-sepolia`、RPC=`X402_RPC_URL`（未設定 fail-closed） |
| 本計画 | B/A と独立。着手は Q2 確定 +「OP-083-C を実装しろ」 |

### 実装（ゲート解除後）

1. `AgentWallet`（空 `wallet/mod.rs`）。Nurture `CoinWallet` とは別レイヤー。
2. 秘密鍵: **Vault `X402_SIGNER_KEY`（ALLOWED_VAULT_SECRETS 追加）** → 注入後 env → macOS `get_keychain_secret`。`keyring` crate 禁止。
3. `negotiate` **実署名**（broadcast しない）。alloy 依存は既存。
4. `X402ClientFactory`（CommerceEngineFactory と分離）。
5. api-server に `Arc<dyn X402Negotiator>` DI 最低1箇所。
6. `.env.example` / ARCHITECTURE / SYNERGY / RIPPLE。
7. OP-011 と直交。OP-080〜082 とは偽依存なし。

**Q1**: ADR-052 政策 OK（オフランプなし）・法務メモ推奨。

---

## 5. Phase 4 — u64↔U256（OP-083-D）

`OnChainAmount` / `currency.rs` 変換ヘルパのみ。`AiomeCoin` 置換禁止。Phase 3 直後。正本: [`op083_cd_x402_plan.md`](op083_cd_x402_plan.md)。

---

## 6. 実施順

```
OP-083-B  spend_guard（日次/月次分離・7箇所）  ← ✅
    →
OP-083-A  supertrait + 8 impl                  ← ✅
    →
[Q2: base-sepolia + X402_RPC_URL] + 「OP-083-C を実装しろ」
OP-083-C  x402 + Vault + AgentWallet（署名まで・broadcast 禁止）
    →
「OP-083-D を実装しろ」
OP-083-D  OnChainAmount
```

---

## 7. Red Team

1. **最悪**: 独立 trait 分割で checkout 全滅 → **supertrait + 統合テスト DoD**。
2. **誤前提**: apps が x402 待ち → **呼出 0**。C を急がない。
3. **やらないメリット**: B だけで escrow バグ修正の価値。月次「新機能」は ADR-050 再発明になる。
4. **日次 0=無制限の単一関数化**: interceptor で `wallet=0` / `policy=0` 時に支出が通り始める → **v3.3 で日次/月次 API を分離して封鎖**。

---

## 8. NURTURE 要件

| セクション | 扱い |
|------------|------|
| §2 台帳 | Phase 2 = ADR-050 硬化 |
| §3 MCP | 非対象 |
| §4 秘密 | Phase 3 = Vault |
| §5 ADR-052 | x402 はオフランプなし |
| §6 VRM | 非対象 |
| §7 A2C | OP-011 ✅ 2026-07-22（無償 KC のみ。有償は凍結） |
| §8 P2P | `allow_p2p_transfer` 非変更 |

---

## 9. やらないこと

- `keyring` / `ProviderType::X402` 混在 / AiomeCoin 置換
- compose への API キー（OP-011 有償化は別ゲート）
- x402 オンチェーン実送金 broadcast（C は署名のみ）
- **月次上限の新機能**（ADR-050 再実装）
- **日次 0 を無制限に統一**（現行 raw min / CoinWallet と衝突）
- **`CoinWallet::check_daily_limit` のセマンティクス変更**
- GiftEngine 流の AppState 二重 Arc（Phase 1）
- Public Beta を OP-083-C でブロック
- commerce-protocol への spend_guard 重複実装（型定義のみ・ヘルパなし）

---

## 10. ARCHITECTURE 波及（Gate 1）

| コンポーネント | ARCHITECTURE.md | 計画上の同期 |
|----------------|-----------------|--------------|
| `aiome-commerce` / `nurture-core` | ✅ 記載済 | — |
| `x402` / `spend_guard` / trait 分割 | ❌ 未記載 | Phase 完了時に追記（§11） |

---

## 11. OPEN / チェックリスト

| ID | フェーズ | 内容 | 依存 |
|----|----------|------|------|
| **OP-083** | 親 | 本計画 | — |
| **OP-083-B** | Phase 2 | spend_guard（日次/月次分離・**7 箇所**）= ADR-050 硬化 | SC 承認 |
| **OP-083-A** | Phase 1 | supertrait + verify→Fiat + 8 impl | Phase 2 |
| **OP-083-C** | Phase 3 | x402 + Vault + wallet | **Q2 + SC**（[`op083_cd_x402_plan.md`](op083_cd_x402_plan.md)） |
| **OP-083-D** | Phase 4 | u64↔U256 | Phase 3 |

```
[x] Phase 2: daily/monthly 分離 API + 7 箇所 + escrow Negative + daily 0 拒否 Negative + CoinWallet 差分ゼロ — **2026-07-13**
[x] Phase 1: supertrait + verify_signature→Fiat + 8 impl + commerce/webhook テスト — **2026-07-13**
[x] Q2 既定確定（base-sepolia + X402_RPC_URL）— **2026-07-20**（op083_cd_x402_plan.md）
[x] Phase 3: ALLOWED_VAULT_SECRETS + 実署名 + X402ClientFactory + DI — **2026-07-20**
[x] Phase 4: OnChainAmount（AiomeCoin 非置換）— **2026-07-20**
[x] ARCHITECTURE / SYNERGY / .env.example / RIPPLE 同期 — **2026-07-20**
```
