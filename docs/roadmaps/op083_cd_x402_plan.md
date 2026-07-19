# OP-083-C/D x402 実行正本（v1.0）

- **ステータス**: **P0 + Q2 + C + D ✅ 2026-07-20**
- **正本 ID**: OPEN **OP-083-C** / **OP-083-D**
- **継承**: [`commerce_layer_tech_debt_plan_v3.md`](commerce_layer_tech_debt_plan_v3.md) Phase 3/4、ADR-052、ADR-053
- **作成**: 2026-07-20（/perfect-plan ブラッシュアップ）

## 0. 命名分離（必須）

| 用語 | 意味 | 状態 |
|------|------|------|
| **Federation transport (ADR-053)** | `FederationOps` export/import/peer sync | ✅ 完了（C/D 非ブロッカー） |
| **OP-083-C start gate** | Q2（chain/RPC 文書）+「OP-083-C を実装しろ」 | 本計画 |
| **Product P2P / Soul Sync (F-5)** | クロスデバイス人格同期 | 任意・別 OP（C/D 非依存） |

「Federation 後」を OP-083-C/D の前提に使わない。

## 1. Q2 既定値（G-Q2）

Human が別値を文書上書きするまでこれが正本:

| 項目 | 値 |
|------|-----|
| Network | `base-sepolia` |
| RPC | env `X402_RPC_URL`（未設定時 Client 構築 fail-closed） |
| Mainnet / broadcast | **本 OP スコープ外** |

## 2. ゲート

```
P0 台帳・ゲート再定義（docs）
  → G-Q2（本節の既定を台帳確定）
  → 「OP-083-C を実装しろ」
  → C 実装
  → 「OP-083-D を実装しろ」
  → D 実装
```

## 3. OP-083-C（Phase 3）

### 作業

1. `AgentWallet`（`libs/aiome-commerce/src/wallet/`）— Nurture `CoinWallet` と別レイヤー
2. Vault `X402_SIGNER_KEY` を `ALLOWED_VAULT_SECRETS` に追加（Vault → env → macOS `get_keychain_secret`）。`keyring` crate 禁止
3. `X402Client::new` release: 鍵欠落 fail-closed。debug モック鍵は test/debug のみ可
4. `negotiate` 実署名（alloy）。モック `0x_mock_tx_...` 廃止。**RPC broadcast しない**
5. `X402ClientFactory`（`CommerceEngineFactory` / `ProviderType` に X402 を混ぜない）
6. api-server bootstrap で `Arc<dyn X402Negotiator>` を最低1箇所 DI（HTTP 新設は必須としない）
7. `.env.example` / ARCHITECTURE / SYNERGY / RIPPLE / CHANGELOG

### 検証

- Positive: negotiate → 署名付き `PaymentProof`（`0x_mock_tx_` でない）
- Negative: 鍵なし / 予算超過 → 拒否。秘密・RPC 詳細がクライアントに出ない
- Safety: Stripe webhook / auth 差分ゼロ

### 禁止

- mainnet / 実送金 broadcast
- OP-011 解除、compose への秘密追加
- AiomeCoin 置換、月次上限再発明
- Soul Sync / samsara-hub 製品作業の同乗

## 4. OP-083-D（Phase 4）

1. `OnChainAmount` + `currency.rs`（`u64` ↔ `U256` ヘルパのみ）
2. x402 / Web3 境界はヘルパ経由
3. AiomeCoin / CoinWallet 置換禁止
4. Negative: オーバーフロー / 切り捨て

## 5. 台帳

- OPEN OP-083-C/D
- Wave C2: [`agentic_production_hardening_plan.md`](agentic_production_hardening_plan.md)
- commerce v3 参照: [`commerce_layer_tech_debt_plan_v3.md`](commerce_layer_tech_debt_plan_v3.md)
