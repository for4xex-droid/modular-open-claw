---
name: stripe-mock-centralization
description: CommerceEngine / Stripe モックの追加・変更、結合テストの Commerce スタブ修正時に読む。Mock の一元定義・cfg ガード・本番パスでの偽成功禁止。commerce.rs / StripeCommerceEngine 本体の変更は AGENTS.md Safety-Critical のためユーザー明示許可が必要。
---

# Stripe Mock Centralization

`MockCommerceEngine` は **`libs/aiome-commerce/src/mock.rs` のみ**に定義し、テスト側でローカル再定義しません。cfg ガードは golden-rules **B-004**（`.agent/skills/docs-ui-ux-golden-rules.md`）に従います。

## 発動条件

- Commerce 結合テスト用の Mock 振る舞いを追加・変更するとき
- `MockCommerceEngine` の二重定義やシグネチャドリフトを疑うとき
- `ProviderType::Mock` や `is_mock` 分岐を触るとき

## 手順

1. **一元定義**: ロジックは `libs/aiome-commerce/src/mock.rs` に集約し、`apps/api-server/src/api_integration_tests/common.rs` 等から `use aiome_commerce::mock::MockCommerceEngine` で参照する
2. **cfg ガード**: `#[cfg(any(test, debug_assertions))]` を Mock 構造体・impl に付与する
3. **本番パス**: `StripeCommerceEngine`（`libs/aiome-commerce/src/stripe/mod.rs`）で `is_mock == false` のとき、スタブが `Ok(...)` を返さず `AiomeError::Infrastructure` を返すことを確認する
4. **Safety-Critical**: `apps/api-server/src/routes/commerce.rs` および `StripeCommerceEngine` の**モック以外**の変更は、ユーザー明示許可なしに行わない（AGENTS.md）

## 良い例 / 悪い例

```rust
// ✅ mock.rs — cfg 付き一元定義
#[cfg(any(test, debug_assertions))]
pub struct MockCommerceEngine { /* ... */ }

// ❌ テストファイル内で struct MockCommerceEngine を再定義 — ドリフトの温床
struct MockCommerceEngine;
impl CommerceEngine for MockCommerceEngine { /* 別実装 */ }
```

```rust
// ✅ is_mock=false — Nurture 未設定なら Infrastructure エラー
if self.is_mock { return Ok("tx_mock".into()); }
Err(AiomeError::Infrastructure { reason: "Nurture S2S URL not configured".into() })

// ❌ 本番相当パスで常に Ok を返す — 決済成功の偽陽性
async fn execute_autonomous_purchase(...) -> Result<String, AiomeError> {
    Ok("tx_mock".into()) // is_mock 未確認
}
```

## 完了条件

- **Positive**: `cargo test -p api-server commerce` および `cargo test -p aiome-commerce` が GREEN
- **Negative Test**: `StripeCommerceEngine` で `is_mock=false`・Nurture 未設定の状態 → 決済系呼び出しが `Err(AiomeError::Infrastructure { .. })` を返すことを確認（偽成功でないこと）
- **Release 確認**: `cargo check --workspace --release` で Mock シンボルが参照されないこと（B-004）

> 出典: memory/2026-06-01.md Lessons「モック複数定義はドリフト。ライブラリ側集約+制御APIが有効」、CHANGELOG「MockCommerceEngine 二重定義排除」
