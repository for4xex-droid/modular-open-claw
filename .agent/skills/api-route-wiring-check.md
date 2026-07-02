---
name: api-route-wiring-check
description: apps/api-server に新 API エンドポイントや CommerceEngine 等のトレイトメソッドを追加するときに読む。router.rs 配線漏れ・OpenAPI だけの登録・Mock 未同期を防ぐチェックリスト。フロントエンドのみの変更では不要。
---

# API Route Wiring Check

新エンドポイント追加時、**ハンドラ実装・router 配線・OpenAPI 登録・Mock 同期**の4点セットを必ず確認します。

## 発動条件

- `apps/api-server/src/routes/*.rs` にハンドラを追加・変更したとき
- `CommerceEngine` 等のトレイトにメソッドを追加したとき
- 結合テストで新パスが 404 になる／コンパイルは通るが HTTP 到達不能なとき

## 手順

1. **ハンドラ**: `apps/api-server/src/routes/<domain>.rs` に `pub async fn` を実装する
2. **router 配線（必須）**: `apps/api-server/src/router.rs` の `build_app` 内で `.route(...)` を追加する
3. **OpenAPI**: `apps/api-server/src/api.rs` の `#[derive(OpenApi)]` → `paths(...)` にハンドラを登録する
4. **トレイト Mock 同期**: 本番実装に加え、テスト用 Mock（例: `apps/api-server/src/api_integration_tests/common.rs`）も同一シグネチャに更新する
5. **影響確認**: `python3 scripts/impact_query.py <Symbol>` で参照箇所を洗い出す

## 良い例 / 悪い例

```rust
// ✅ router.rs に HTTP パスを配線
.route("/api/v1/commerce/balance/:agent_id", get(routes::commerce::get_balance))

// ❌ api.rs の paths(...) だけ更新 — リクエストは 404 のまま
#[derive(OpenApi)]
#[openapi(paths(routes::commerce::get_balance))] // router.rs 未登録
```

```bash
# ✅ 参照漏れの事前確認
python3 scripts/impact_query.py get_balance
```

## 完了条件

- **Positive**: 結合テスト（`cargo test -p api-server`）で新エンドポイントが期待どおり 200 または 4xx を返す
- **Negative Test**: `router.rs` の該当 `.route(...)` を一時的にコメントアウト → 404 を確認 → 配線を戻して再テスト GREEN
- **Revert**: router 配線を必ず復元し、404 Negative Test 後も GREEN であること

> 出典: memory/2026-04-24.md Lessons「OpenAPI docs + api.rs 登録だけでは到達不能。router.rs 配線が必須」、memory/2026-04-27.md「トレイトメソッド追加時は Mock 全更新」
