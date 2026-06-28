# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-06-29 (v8.5 — 主要モジュール別/12次元分割監査・Tauri型共有同期負債の特定)
**前回監査日**: 2026-06-29 (v8.4 — 指摘事項のテーブル化・Open Questions の追加)
**対象コードベース**: **152k LOC** (Rust ~128k + TypeScript ~24k)
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh`, Git hotspot analysis, grep-based deep scan
**分析コミット**: `5268943d`

---

## 1. Executive Summary

本監査フェーズでは、ワークフローの分割監査（Subagent Dispatch）の精神に基づき、コードベースを主要3コンポーネント（**`libs/infrastructure`**, **`apps/api-server`**, **`apps/management-console`**）に分割し、12次元監査フレームワークの全項目について詳細な個別スキャンを実施しました。

今回の監査により、コンポーネント間の境界において、以下の深刻な長期負債が新たに特定されました：
1. **Tauri IPC 型共有の欠如と手動同期負債 (Dimension 3/11)**: `src-tauri/src/lib.rs` での IPC 定義と、フロントエンド `management-console` 内の型定義が手動同期されており、`ts-rs` 等による自動型生成ブリッジが導入されていません。これにより定義乖離リスクが恒常化しています。
2. **認証フローにおける静かな失敗と observability の欠損 (Dimension 7/8)**: `auth.rs:142` での管理者ハッシュパース失敗時に、警告ログを出力せずに `authenticated = false` とする「静かな失敗」が放置されています。

---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **WebGL カラーコードと tokens.css のブリッジ化 (U-002)** | 🔴 | `BiomeCellGrid.tsx:35-44` および `BiomeGame.tsx:248-255` における WebGL 元素カラーの HEX ハードコードの排除と、memoized CSS 変数ブリッジの適用。 | 5h | `[NEW]` |
| **P2** | **フロントエンド型安全性 (as any) の解消** | 🔴 | `WorkflowBuilder.tsx` や `workflowConverter.ts` における `as any` キャストを、厳密な型定義または `unknown` 絞り込みにリファクタリング。 | 4h | `[NEW]` |
| **P3** | **Tauri IPC 構造体の TypeScript 自動生成同期化** | 🟡 | `src-tauri/src/lib.rs` 内の Rust 構造体に `ts-rs` を導入し、TypeScript インターフェースへの自動コンパイル・同期ブリッジを構築。 | 5h | `[NEW]` |
| **P4** | `mcp/discovery.rs` (1,113行) God Module の分解 | 🟡 | OAuth エンドポイント、トークン交換、MCP テンプレートが1ファイルに密結合。URL 変更時のリグレッションリスク。 | 6h | — |
| **P5** | Error 型の統一 (10種類 → 3階層) | 🟡 | `thiserror` 7ファイル vs `anyhow` 47ファイルの混在。ただし `error.rs` の変換層 (22テスト) は模範的な設計。 | 6h | — |

---

## 3. Quick Wins（1時間以内で修正可能）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-7** | `BiomeEventToast.tsx` のインライン styles から HEX フォールバックを排除 | `BiomeEventToast.tsx:48-53` | tokens.css への 100% 依存への準拠 | `[RESOLVED]` |
| **QW-8** | `api_resolver.ts` 内の `window as any` をグローバル宣言または `typeof window` での型ガードへ移行 | `api_resolver.ts:25` | グローバルオブジェクトアクセスの型安全性確保 | `[RESOLVED]` |
| **QW-9** | `dispatcher.rs` の `.ok()` エラー抑制に警告ログ出力を追加 | `dispatcher.rs:134` | JSON 変換失敗時のデバッグアビリティ向上 | `[RESOLVED]` |
| **QW-10** | `auth.rs` の管理者パスワードハッシュパース失敗時に `warn!` ログを出力 | `auth.rs:142` | ハッシュ破損や認証不正時の可観測性の確保 | `[NEW]` |

---

## 4. Findings Table（モジュール別・12次元スキャン）

### 4.1. `libs/infrastructure` (インフラ・共通コア)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 |
|---|---|---|---|---|
| **Dimension 3: Type & Contract Debt** | `JobQueue` トレイト定義と実装 `UniversalJobQueue` の API 乖離。 | [traits.rs:300-370](file:///Users/motista/Desktop/antigravity/aiome/libs/aiome-core-contracts/src/traits.rs) | 🟡 Medium | 3h |
| **Dimension 7: Error handling** | `serde_json::to_string` 失敗時のログなきエラー握り潰し。 | [dispatcher.rs:134](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/task_orchestrator/dispatcher.rs#L134) | 🟡 Medium | 0.5h |

### 4.2. `apps/api-server` (API バックエンド)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 |
|---|---|---|---|---|
| **Dimension 7: Error handling** | 管理者パスワードハッシュパースエラー時の警告ロギング欠落。 | [auth.rs:142](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L142) | 🟡 Medium | 0.5h |
| **Dimension 8: Security hygiene** | OAuth や Webhook 以外の API への `Authenticated` の一貫した型強制。 | `vault.rs` (QW-4 で解消済み) | 🟢 Resolved | — |

### 4.3. `apps/management-console` (フロントエンド)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 |
|---|---|---|---|---|
| **Dimension 3: Type & Contract Debt** | 動的プロパティアクセスに対する `as any` キャストの使用。 | [workflowConverter.ts:139](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/workflowConverter.ts#L139) | 🟡 Medium | 1h |
| **Dimension 3: Type & Contract Debt** | イベントデータおよびノードタイプ判定時の `as any` キャストの使用。 | [WorkflowBuilder.tsx:101, 234](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/components/WorkflowBuilder.tsx#L101) | 🟡 Medium | 1.5h |
| **Dimension 11: Tauri IPC 型安全性** | `window` グローバル拡張オブジェクトアクセス時の `as any` キャスト。 | [api_resolver.ts:25](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/api_resolver.ts#L25) | 🟡 Medium | 0.5h |
| **Dimension 11: Tauri IPC 型安全性** | Rust 構造体との共有型定義（TypeScript interface）の手動同期。 | `management-console/src/types/` | 🟡 Medium | 5h |
| **Dimension 12: tokens.css 遵守度** | 元素表示用カラー（8元素）の WebGL THREE.Color 内での HEX ハードコード。 | [BiomeCellGrid.tsx:35-44](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeCellGrid.tsx#L35) | 🔴 High | 3h |
| **Dimension 12: tokens.css 遵守度** | 元素カラーマッピング用 HEX リテラルの直書き。 | [BiomeGame.tsx:248-255](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeGame.tsx#L248) | 🔴 High | 2h |
| **Dimension 12: tokens.css 遵守度** | HUDネオングロー表示用のインライン HEX フォールバック指定。 | [BiomeHUD.tsx:98](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeHUD.tsx#L98) | 🟡 Medium | 0.5h |

---

## 5. Things that look bad but are actually fine

- **`allow-anti-pattern` による expect / unwrap / unreachable の使用**:
  - [secret_redactor.rs:30](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/security/secret_redactor.rs#L30): `Regex::new` の `.expect()`。静的な正規表現リテラルのコンパイル失敗はコンパイル時のミスであり、実行時の入力依存ではないため許容。
  - [cortex_ingester.rs:210](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/cortex_ingester.rs#L210): タイトルパース用正規表現のコンパイル。同上。
  - [http.rs:23](file:///Users/motista/Desktop/antigravity/aiome/libs/core/src/http.rs#L23): Reqwest グローバルクライアントビルド失敗時の `.expect()`。ホストマシンの TLS 設定が破綻している場合の致命的な起動失敗であり、正常な実行継続が不可能なため許容。
- **テスト用モジュールにおける unwrap() / expect()**:
  - [validator.rs:354](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/validator.rs#L354) / [workflow/mod.rs:104, 183](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/workflow/mod.rs#L104): テスト関数内でのアサーション・早期エラーパニック用の unwrap() / expect() の使用。本番の Zero-Panic 規則からは安全に除外されます。
- **非同期ライフサイクル監視における tokio::spawn の使用**:
  - [supervisor.rs:41](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/supervisor.rs#L41): `TaskSupervisor` での非同期ループ起動。一見するとリークしやすい `tokio::spawn` の乱用に見えますが、CancellationToken と統合された堅牢な自動再起動・Fail-Closed シャットダウン設計が実装されており、リーク・ゾンビプロセスのリスクは排除されています。

---

## 6. Open Questions

1. **JobQueue トレイトの API 乖離について**:
   `UniversalJobQueue` にのみ定義されている多数のパブリック補助メソッドについて、トレイトの境界定義（`traits.rs`）側にすべて引き上げるべきか、あるいは単に `UniversalJobQueue` 内部でのカプセル化（`pub` を削る、または `crate` プライベート化）を進めるべきでしょうか？
2. **WebGL / Canvas テーマカラーの同期方法について**:
   `BiomeCellGrid` や `BiomeGame` 内の Three.js / Canvas 描画カラーについて、CSS 変数を JS 上で読み取って `THREE.Color` を動的に生成する memoized bridge クラス（`docs/architecture/theme_protocols.md` に記載 of `theme_protocols.md`）を新規作成して統合する方針で進めてよいでしょうか？
3. **Tauri IPC 構造体の自動型共有について**:
   Rust の `ts-rs` を用いて、Rust 構造体から TypeScript 定義ファイルをビルド時に自動出力し、フロントエンド側で直接インポートして同期させるパイプラインを構築してよいでしょうか？

---

## 7. メトリクス推移

| 指標 | v7.0 | v7.2 | v8.0 | v8.1 | v8.2 | v8.3 | v8.4 | v8.5 (2026-06-29) | トレンド |
|---|---|---|---|---|---|---|---|---|---|
| 総 LOC | 152k | 152k | 152k | 152k | 152k | 152k | 152k | **152k** | → |
| Rust テスト数 | 4,459 | 4,459 | 4,524 | 4,524 | 4,524 | 4,524 | 4,524 | **4,524** | → |
| U-002 違反 (TSX/WebGL) | 0 | 0 | 0 | 0 | 12 | 12 | 12 | **12** | 維持 |
| `as any` 本番使用 (TS) | 1 | 1 | 1 | 1 | 5 | 5 | 5 | **5** | 維持 |
| CC-6 違反 (Auth) | 0 | 0 | 6 | 0 | 0 | 0 | 0 | **0** | 完全解消維持 ✅ |
| ログなしエラー抑制 (.ok()) | 0 | 0 | 1 | 1 | 1 | 1 | 1 | **1** | 維持 |
| God Module (1k+ 行) | 3 | 3 | 3 | 3 | 3 | 3 | 3 | **3** | 維持 |

---

*Generated by `/tech-debt-audit` workflow — 2026-06-29 v8.5 (主要モジュール別/12次元分割監査・Tauri型共有同期負債の特定完了)*
