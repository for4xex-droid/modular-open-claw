# 🚨 エラーハンドリング設計・責務ドキュメント

本ドキュメントは、Aiome システムにおけるエラーハンドリング設計、エラー分類、および実装ルールを記述するものです。

## 1. エラーハンドリング設計の基本方針

Aiome は、エージェント自律運転システムとして以下の堅牢性を担保します：
1. **Zero-Panic Policy**: ライブラリ層およびインフラ層での不意な `panic!` を完全排除し、すべて `Result` または安全なフォールバックへ変換します。
2. **CWE-209/CWE-532 準拠（情報漏洩防止）**: 本番ビルド環境において、データベースエラーや通信詳細などの内部エラーメッセージをそのままフロントエンド（ユーザー）に返さず、UUID化された `Error ID` を用いてサニタイズします。
3. **HTTP セマンティクスとのマッピング**: API 境界では、例外が適切な HTTP ステータスコード（例: `400 Bad Request`, `403 Forbidden`, `500 Internal Server Error`）に変換されます。

---

## 2. 既存エラー型とそれぞれの責務

システム内には、各モジュールの独立性と意味的整合性を保つため、主に以下の 10 種類のエラーが定義されています。

| エラー型名 | 定義場所 | 責務・役割 |
|---|---|---|
| **AiomeError** | `libs/aiome-contracts/src/error.rs` | **システム全体の主要なドメインエラー**。APIサーバー、インフラ、LLM連携、ファイルシステムなどの例外を集約し、Axum の HTTP レスポンス変換（IntoResponse）および CWE-209 サニタイズ（UUIDマスク）を担当。 |
| **SoulError** | `libs/soul/src/error.rs` | **エージェント自律状態（SOUL.md）の整合性エラー**。状態遷移、ライフサイクル、バインドの不整合などを定義。 |
| **X402Error** | `libs/aiome-commerce/src/x402.rs` | **経済・ライセンス処理のエラー**。Stripe連携、トークン取引、バッジ付与、課金ステータスに関する異常を捕捉。 |
| **CsamError** | `libs/shared/src/csam/image_hash.rs` | **不適切コンテンツ検出時のエラー**。隔離（Quarantine）ディレクトリへの書き込み失敗や、ハッシュミスマッチを定義。 |
| **NurtureError** | `commercial/libs/commerce-protocol/src/error.rs` | **Nurture 連盟（Samsara Hub）連携のエラー**。P2Pノード間の通信や契約データのシリアライズに関するエラー。 |
| **ProcessError** | `libs/infrastructure/src/security_zombie.rs` | **セキュリティ隔離およびサブプロセス実行のエラー**。Dockerコンテナ/Shadow Workerのハンドリングに失敗した際のエラー。 |
| **ProportionError** | `libs/avatar-engine/src/proportions.rs` | **アバター画像生成・パラメータ妥当性エラー**。アバター構成比率の設定不整合を定義。 |
| **LoaderError** | `libs/avatar-engine/src/loader.rs` | **アセット・アバターロードのエラー**。ローダー初期化やアセットリソースの解決失敗。 |
| **FactoryResetError** | `libs/shared/src/bootstrap_detector.rs` | **ファクトリーリセット・クリーンアップのエラー**。ブートストラップ復旧処理が失敗した場合の致命的エラー。 |
| **BudgetExhaustedError** | `libs/aiome-contracts/src/error.rs` | **予算上限超過（JobBudget）エラー**。自律エージェントの処理予算オーバー時にスロー。 |

> **OP-051 / ADR-054（2026-07-10）**: 上記 10 種を「Domain / Subsystem / Internal」の 3 階層に整理する Decision は [`docs/decisions/054-error-hierarchy.md`](../decisions/054-error-hierarchy.md)（**Proposed**）。コード一括置換は ADR Accepted 後のみ。本節 §2 が現状の正本。

---

## 3. 新規エラー型追加の制限ルール

コードベースの保守性と一貫性を保護するため、開発エージェントを含むすべての開発者は以下の制限ルールを遵守しなければなりません：

1. **原則として新規エラー型の追加を禁止する**:
   新たに独自のエラー型（`struct SomeError` や `enum SomeError`）を定義することは原則として認められません。機能追加の際は、既存の `AiomeError` に新しいバリアントを追加することを最優先してください。
2. **例外的な新規追加の条件**:
   - 完全に他から独立した自己完結ライブラリ（他の aiome クレートに依存しないスタンドアロンユーティリティ）を新規作成する場合。
   - サンドボックス隔離されたクリティカルコンポーネントであり、専用のエラーハンドリング契約を厳格に要求する場合。
3. **新規エラー型を定義する場合の審査・実装手順**:
   - 変更前に必ず人間に設計上の必要性を説明し、承認を得ること。
   - `thiserror::Error` および適切な `#[derive(Debug)]` を用いて、可読性の高い文字列表記を定義すること。
   - API 境界（`api-server` 等）へ波及するエラーである場合、`AiomeError` への `From` 変換トレイトを `libs/aiome-contracts/src/error.rs` に必ず実装し、安全なサニタイズパスへマッピングすること。

*最終更新: 2026-07-10 — ADR-054（OP-051）リンク追加*
