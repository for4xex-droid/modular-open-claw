---
name: architecture-rules
description: Rust バックエンド（libs/・apps/）のコードを書く・変更する前に必読の Aiome 固有アーキテクチャルール（R-001〜R-008）。JobQueue/SQLite/async 再帰/JSON構築/unwrap 禁止等。UI のみの変更では不要。
---

# Aiome Architecture Rules — AI Agent Must-Read

> **Status**: Accepted  
> **Created**: 2026-03-18  
> **Purpose**: AIエージェントが遵守すべきアーキテクチャルール。  
> コーディング前に必ず読むこと。違反はCIで自動検出される。

---

## R-001: SwarmOps Direct Call [CRITICAL]

**ルール**: `karma.rs`, `guardrails.rs` 等の `do_*` メソッド内から swarm 関連操作を呼ぶ場合、`JobQueue` トレイトメソッド経由ではなく `SwarmOps::do_*` を直接使う。

```rust
// ✅ 正しい
use super::swarm::SwarmOps;
let node_id = self.do_get_node_id().await?;
let clock = self.do_tick_local_clock().await?;
let sig = self.do_sign_swarm_payload(&target).await?;

// ❌ 禁止 — SQLite デッドロックを引き起こす
let node_id = self.get_node_id().await?;  // JobQueue trait dispatch
```

**理由**: `do_get_node_id` は SQLite トランザクションを使用してキーを生成する。`do_sign_swarm_payload` がキー未生成時に再帰的に `do_get_node_id` を呼ぶと、ネストしたトランザクション → SQLite 単一 writer 制限 → デッドロック。

**発見日**: 2026-03-18（karma テスト 8 件が無限ハング）

---

## R-002: Box::pin for JobQueue Delegation [CRITICAL]

**ルール**: `impl JobQueue for SqliteJobQueue` の全委譲メソッドは `Box::pin()` で必ずラップする。

```rust
// ✅ 正しい
async fn store_karma(...) -> Result<(), AiomeError> {
    Box::pin(self.do_store_karma(...)).await
}

// ❌ 禁止 — スタックオーバーフローを引き起こす
async fn store_karma(...) -> Result<(), AiomeError> {
    self.do_store_karma(...).await
}
```

**理由**: 60+ メソッドの async state machine が合計数百 KB のスタックを消費し、debug ビルドでスタックオーバーフローが発生する。

---

## R-003: JSON Construction [HIGH]

**ルール**: `format!()` で JSON 文字列を組み立てない。必ず `serde_json::json!` マクロを使う。

```rust
// ✅ 正しい
let payload = serde_json::json!({
    "user_input": user_text,
    "timestamp": now
});

// ❌ 禁止 — JSON injection 脆弱性
let payload = format!(r#"{{"input": "{}"}}"#, user_text);
```

**理由**: ユーザー入力に `"` や `}` が含まれると JSON 構造が破壊される。

---

## R-004: No Silent Error Suppression [HIGH]

**ルール**: `.ok()` や `let _ = ...` でエラーを握りつぶさない。最低限 `warn!` でログ出力する。

```rust
// ✅ 正しい
if let Err(e) = sqlx::query("...").execute(&pool).await {
    warn!("Non-critical operation failed: {}", e);
}

// ❌ 禁止
let _ = sqlx::query("...").execute(&pool).await;
sqlx::query("...").execute(&pool).await.ok();
```

**理由**: 失敗が見えないと原因調査に時間がかかる。FTS5 トリガーの問題で実際に発生。

---

## R-005: Panic-Free Production Code [HIGH]

**ルール**: テストコード以外で `.unwrap()` / `.expect()` を使わない。

```rust
// ✅ 正しい
let value = config.get("key").unwrap_or_else(|| {
    error!("Missing required config: key");
    std::process::exit(1);
});

// ❌ 禁止（テスト以外）
let value = config.get("key").unwrap();
let value = config.get("key").expect("key not found");

// ❌ 禁止 — パニック検出回避のための無限ループも同罪
// NG実例: skills/mod.rs L163 の unwrap_or_else(|_| loop {}) は CPU 100% で沈黙する
// （出典: REMAINING_TASKS.md 2026-07-02 / OPEN.md OP-053）
let value = init().unwrap_or_else(|_| loop {});
```

---

## R-006: No Hardcoded URLs [MEDIUM]

**ルール**: `http://127.0.0.1`, `http://localhost` 等の URL リテラルをプロダクションコードに書かない。環境変数 or 設定ファイル経由で取得する。

---

## R-007: Test Quality Standards [MEDIUM]

**ルール**: テストを通すために以下を行わない:
- assert の条件を緩める
- assert を削除する
- テスト自体を `#[ignore]` にする

テストが通らない場合は、テストではなくプロダクションコードを修正すること。

---

## R-008: No Recursive Async Calls [CRITICAL]

**ルール**: async 関数から同じ async 関数を再帰的に呼び出さない。特に SQLite トランザクションを含む関数は絶対禁止。

```rust
// ❌ 致命的 — SQLite デッドロック
async fn do_sign(&self, payload: &str) -> Result<String, Error> {
    if key_missing {
        self.do_init_keys().await?;       // トランザクション開始
        Box::pin(self.do_sign(payload)).await  // 再帰 → 2つ目のトランザクション → DEADLOCK
    }
}

// ✅ リニアフローで解決
async fn do_sign(&self, payload: &str) -> Result<String, Error> {
    self.do_init_keys().await?;           // 先にキーを確保
    // キーが確実に存在する状態で署名（再帰なし）
    let key = sqlx::query("SELECT...").fetch_one(&pool).await?;
    Ok(sign_with_key(key, payload))
}
```

**理由**: R-001 と同根。SQLite は同一接続内で同時に1つのトランザクションしか保持できない。
