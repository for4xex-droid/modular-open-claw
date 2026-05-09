# TECH_DEBT_AUDIT

**Date**: 2026-05-10
**Target**: Aiome & Project-Nurture
**Audit Engine**: `/tech-debt-audit` (12 Dimensions)
**Status**: 2nd Iteration (Diff Update)

## 1. Executive Summary

両リポジトリ（Aiome, Nurture）の統合技術的負債監査の第2回イテレーションを実行しました。
前回のセッションから TDD ベースでのフェーズ1〜3の実装が行われ、Aiome のエラーアーキテクチャの統一（階層型エラー設計）や環境変数の整備が完了しました。

Nurture における重大なセキュリティ負債については、Phase 1 (`cargo update` による SemVer パッチ) が完了し、一部の脆弱性が解消されましたが、依然として `wasmtime` 等のメジャーバージョンに依存する **22件** の脆弱性が残留しています。これらは破壊的変更を伴うため、次なる Targeted Major Upgrade (Phase 2) が急務です。

## 2. Top 5 Priorities (最優先解消事項)

1. **[Nurture] 深刻な CVE の解消 (メジャーアップデート)**
   - 状態: 22 件の脆弱性 (`wasmtime`, `rustls-webpki` 等) が残留。
   - 影響: サンドボックスエスケープ、ホストメモリ漏洩の危険性。
   - アクション: Phase 2 として、コードベースを伴う `wasmtime` 等のメジャーバージョンアップとアダプタ修正。
2. **[Aiome] `format!` による手動 JSON 構築の排除**
   - 状態: 複数ファイルで手動での JSON 構築を検出。
   - 影響: エスケープ漏れによる JSON インジェクションの危険。
   - アクション: 該当箇所を特定し、`serde_json::to_string` へ移行。
3. **[Aiome] サイレント `.ok()` の握り潰し解消**
   - 状態: 複数のインフラストラクチャ層で `.ok()` によるエラー無視が進行。
   - 影響: 将来的なバグ調査時のオブザーバビリティ（可観測性）の欠如。
   - アクション: 適切なログ出力、または `Result` エラーハンドリングへのリファクタリング。
4. **[Nurture] Mock への Zero-Panic 適用**
   - 状態: `libs/nurture-infra/src/storage/mod.rs:71` に不正な `.unwrap()`。
   - 影響: CI / 監査パイプラインの統一阻害。
   - アクション: `// allow-anti-pattern` アノテーションの手動追加。
5. **[RESOLVED] [Aiome] Error 型の統合**
   - 状態: `AppError` 境界を用いた階層型マッピングにより解消。

## 3. Quick Wins (1時間以内で解消可能な負債)

- **[RESOLVED] 環境変数の同期**: `Aiome` の `.env.example` に `OTHER_VAR` を追加。
- **[PENDING] Nurture の Mock Unwrap**: `libs/nurture-infra/src/storage/mod.rs:71` の `.unwrap()` アノテーション付与（前回は実行保留）。

## 4. Findings Table (12次元監査結果)

| 次元 | 対象 | 深刻度 | 該当ファイル / 詳細 | 見積もり |
|---|---|---|---|---|
| **1. Arch. Decay** | Aiome | ⚠️ 中 | `libs/infrastructure/src/job_queue/mod.rs` (57回の高頻度変更)。JobQueue Trait と impl のメソッド数不一致。 | 1 Day |
| **2. Consistency** | Aiome | ✅ 完了 | **[RESOLVED]** Error 型マッピング（階層型エラーアーキテクチャ）完了。 | - |
| **3. Type & Contract** | Nurture | 🔴 高 | `wasmtime`, `rustls-webpki` 等のメジャーバージョン脆弱性 (残22件)。 | 3 Hours |
| **4. Test Debt** | Nurture | ⚠️ 低 | `libs/nurture-infra/src/storage/mod.rs:71` モック内の `unwrap()`。 | 5 Min |
| **5. Dependency** | Aiome | ✅ 完了 | **[RESOLVED]** `.env.example` に `OTHER_VAR` 追加完了。 | - |
| **5. Dependency** | Aiome | ⚠️ 中 | 未保守の外部クレート使用（`adler`, `derivative`, `paste`, Gtk系バインディング）。 | 2 Days |
| **6. Performance** | 両方 | ✅ 健全 | メモリリークや N+1 を示す明確なスキャン結果なし。 | - |
| **7. Error Handling** | Aiome | 🔴 高 | 複数ファイルでの silent `.ok()` (握り潰し)。 | 1 Day |
| **8. Security** | Aiome | 🔴 高 | `format!` による JSON 構築の可能性（インジェクションリスク）。ハードコード URL の使用。 | 1 Day |
| **9. Docs Drift** | Nurture | ⚠️ 中 | `libs/nurture-infra/src/economy/bridge.rs` の変更に伴う ADR 同期確認。 | 2 Hours |
| **10. Zero-Panic** | Nurture | 🔴 高 | `libs/nurture-infra/src/storage/mod.rs:71` 未解消。 Aiome は 0件維持（PASS）。 | 5 Min |
| **11. Tauri IPC** | Aiome | ✅ 健全 | 型乖離なし。 | - |
| **12. tokens.css** | Aiome | ✅ 健全 | HEX / RGBA 違反なし（U-002 完全準拠）。 | - |

## 5. Things that look bad but are actually fine

- **Nurture の `MockAssetStorage` 内の `.unwrap()`**: `libs/nurture-infra/src/storage/mod.rs:71`。テスト専用の Mock 実装であるため本番稼働でのパニックリスクはありません。ただし、ポリシーの一貫性のために `// allow-anti-pattern` などの明示的なオプトアウトアノテーションが推奨されます（現在 Pending）。
- **Aiome の GTK 関連 Cargo Audit 警告**: `atk`, `gdk`, `gtk` など 16 個の Allowed warnings が検出されていますが、これは Aiome が Tauri/Webview 側で OS ネイティブ機能（System Tray 等）に依存しているためであり、現在の仕様上回避不可能です（許容された負債）。

## 6. Open Questions

1. **[RESOLVED] Error 型の統合**: 階層型エラー設計 (Composition Root) を採用し実装完了。
2. **[RESOLVED] Nurture の依存関係アップデート方針**: Phased Upgrade（段階的）を採用。Phase 1（パッチ）は完了し、次は Phase 2（コアエンジンのメジャー更新）。
3. **[NEW] Nurture Phase 2 メジャー更新のタイミング**:
   - 残留している 22 件の脆弱性 (`wasmtime` 関連) は API や実行モデルの破壊的変更（例: `Store` のライフサイクル変更など）を伴う可能性が高いです。これを「今すぐ行う」か、「別途の専用スプリントで慎重に行う」か、優先度はいかがでしょうか？
