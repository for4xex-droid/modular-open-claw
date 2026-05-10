# TECH_DEBT_AUDIT

**Date**: 2026-05-11
**Target**: Aiome
**Audit Engine**: `/tech-debt-audit` (12 Dimensions)
**Status**: 4th Iteration (Diff Update)

## 1. Executive Summary

Aiome リポジトリの第4回 統合技術的負債監査（Tech Debt Audit）を実行しました。
今回のスキャンでは、新たに Git コミット履歴から変更頻度の高い「ホットスポット」を特定し、構造的な負債を分析しました。また、`enforce_unwrap_deny.py` を用いた全自社製コード（`apps libs packages tools tests`）の検証により、**未承認のパニック（Zero-Panic Policy違反）が完全に 0件（数学的証明済）** であることが確認されました。

新たに特定された課題は、巨大化する結合テストファイル（Test Debt）および一部の未実装スタブ（TODO）です。

## 2. Top Priorities (最優先解消事項)

1. **[RESOLVED] [Test Debt] 巨大化した統合テストファイルの分割**
   - 状態: 4,100行に及ぶ `api_integration_tests.rs` モノリスを解体し、`api_integration_tests/` 配下のドメイン別モジュールに分割しました。スコープと `#[serial]` の修復を行い、175個のテストが全てGreen（Zero-Panic）であることを証明しました。これにより最大のホットスポットが解消されました。
2. **[RESOLVED] [Aiome] Zero-Panic アノテーションのフォーマット修正とスクリプト改修**
   - 状態: スクリプトによる全ディレクトリ走査で Exit code 0 が証明され、完全に解決しました。
3. **[RESOLVED] [Aiome] Error 型の統合**
   - 状態: 階層型エラー設計により既に解決済み。
4. **[PENDING] [Aiome] 未保守の外部クレート使用**
   - 状態: `adler`, `paste`, Gtk系バインディング等は OS ネイティブ機能のため許容していますが、代替クレートの調査は継続推奨です。

## 3. Quick Wins (1時間以内で解消可能な負債)

- **[NEW] `[Good First Issue]` タグのついた TODO の消化**:
  - `libs/infrastructure/src/intent/affiliate_adapter.rs:84` (Amazon/Rakuten API スタブ)
  - `libs/shared/src/mcp_constants.rs:24` (MCP Namespace 拡張)
  これらは明確なタスクとして切り出されており、コントリビューターが短時間で解消可能です。

## 4. Findings Table (12次元監査結果)

| 次元 | 対象 | 深刻度 | 該当ファイル / 詳細 | 見積もり |
|---|---|---|---|---|
| **1. Arch. Decay** | Aiome | ✅ 健全 | 未実装スタブ (`bootstrap.rs:1173` の SLM 注入待ち) が残存していますが、設計上の破綻はありません。 | - |
| **2. Consistency** | Aiome | ✅ 完了 | **[RESOLVED]** Error 型マッピング完了。 | - |
| **3. Type & Contract** | Aiome | ✅ 健全 | Type-Driven Security (Auth Extractor Enforcement) 120/120 エンドポイントで遵守確認済。 | - |
| **4. Test Debt** | Aiome | ✅ 完了 | **[RESOLVED]** `api_integration_tests.rs` モノリスを解体し、ドメイン別のモジュールディレクトリ構造に移行完了。 | - |
| **5. Dependency** | Aiome | ⚠️ 中 | `adler`, `paste`, Gtk系バインディング等の使用。 | 2 Days |
| **6. Performance** | Aiome | ✅ 健全 | メモリリークや N+1 を示す明確なスキャン結果なし。 | - |
| **7. Error Handling** | Aiome | ⚠️ 低 | 複数ファイルでの silent `.ok()` は意図的な MPSC チャンネルの送信エラー無視などが主。 | 1 Day |
| **8. Security** | Aiome | ✅ 健全 | `cargo audit` の結果、Rust 依存関係の脆弱性は 0件。 | - |
| **9. Docs Drift** | Aiome | ✅ 健全 | - | - |
| **10. Zero-Panic** | Aiome | 🟢 完璧 | **[RESOLVED]** 全コード走査にて違反 0件。完全なクリーン状態を達成。 | - |
| **11. Tauri IPC** | Aiome | ✅ 健全 | 型乖離なし。 | - |
| **12. tokens.css** | Aiome | ✅ 健全 | HEX / RGBA 違反なし。 | - |

## 5. Things that look bad but are actually fine

- **68件の .unwrap() / .expect() 警告 (deep-scan.sh)**: `deep-scan.sh` の単純 grep によって多数検出されますが、これは `// allow-anti-pattern` や `#[cfg(test)]` を無視しているためです。正式な CI ゲート（`enforce_unwrap_deny.py`）では 0件が証明されています。
- **ハードコードされた URL**: `127.0.0.1` などのローカルIPが複数検出されますが、これらは単体テストのモックサーバーやフォールバック設定であり、SSRF の原因にはなりません。
- **Turso / libSQL への未移行**: SQLite の単一ライター制約回避のため Turso の導入が提案されましたが、ADR-005 により「Sovereign Verifier 思想との衝突」を理由に意図的に却下されています。

## 6. Open Questions

（現在、未解決の Open Question はありません）
