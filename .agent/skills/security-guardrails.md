---
name: security-guardrails
description: 外部入力（RSS/Web/LLM出力/ユーザー入力）を扱うコードやサニタイズ処理を実装・変更するときに読む。リポジトリ内の検証済み実装（purge_entities / sanitize_for_prompt / Cleanroom）の所在マップ。auth.rs 等の Safety-Critical Zone 変更許可については AGENTS.md を参照。
---

# Security Guardrails

セキュリティ実装を行う際は、必ず本リポジトリの**既存の検証済み実装**を参照・再利用してください。
車輪の再発明を避け、検証済みのパターンを使用することで脆弱性を防ぎます。

## 検証済み実装の場所（車輪の再開発禁止）

| 用途 | 実装 |
|---|---|
| 外部入力の無害化（エンティティ除去） | `libs/core/src/security_impl.rs` の `purge_entities()` |
| プロンプトインジェクション防御 | `libs/shared/src/guardrails.rs` の `sanitize_for_prompt()` |
| 課金・レート系ガード | `libs/infrastructure/src/job_queue/guardrails.rs` |
| スキルの AI セキュリティ監査 | `libs/infrastructure/src/skills/cleanroom.rs`（Cleanroom, G-22） |

**職責分離**: 文字列の安全な切り詰めは `shared::strings`、プロンプトインジェクション防御は `shared::guardrails::sanitize_for_prompt` を使う（混同しない）。

## 実装手順

1. 上記テーブルから該当する既存実装を `view` で読み、再利用または同パターンで拡張する。
2. 新規のセキュリティ処理を書く場合、「SQLインジェクション対策」「XSS対策」「パス・トラバーサル対策」の3点が含まれているか確認する。
3. Negative Test（不正入力を投げて拒否されることの確認）を必ず書く（AGENTS.md Verification Protocol）。

## 実装時の注意点

- **入力はすべて疑う**: ユーザーからの外部入力（RSS, Web Search, LLM出力等）は、`aiome_core::security_impl::purge_entities()` を通すまで信頼してはならない。
- **機密情報のハードコード禁止**: Sentinelスキャンで検出されるようなコードを書かないこと。
- **スキルインポート時の監査**: 新しいスキルをインポートする際は、必ず `Cleanroom` による AI セキュリティ監査 (G-22) を通過させること。
