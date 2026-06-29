# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-06-29 (v9.0 — 全面再スキャン・ホットスポット検証・God Module 再評価・commune_ws 新規負債特定)
**前回監査日**: 2026-06-29 (v8.5)
**対象コードベース**: **175k LOC** (Rust ~143k + TypeScript ~33k)
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh --ci`, Git hotspot analysis (3ヶ月), grep-based deep scan
**分析コミット**: `18f8501f`

---

## 1. Executive Summary

v9.0 では、Git ホットスポット分析（過去3ヶ月で変更頻度の高いファイル Top 20）を起点に、前バージョンで報告した全項目を **実コード検証** で再評価しました。

### 主要な発見・修正

1. **P4 (God Module `discovery.rs`) は解消済み**: 以前 1,113行と報告していた `skills/discovery.rs` は現在 **305行** に縮小されていました。P4 は `[RESOLVED]` に変更します。
2. **God Module 数の訂正 (3件 → 1件)**: 1,000行超のファイルを全数調査した結果、テストコードを含むファイル（`workflow/mod.rs` 99%テスト, `immune_system.rs` 69%テスト, `stripe/mod.rs` 98%テスト）は God Module に該当しません。**唯一の真の God Module は `skills/mod.rs` (1,134行, テスト0%)** です。
3. **`commune_ws.rs` に新規負債 3件を特定 (Dimension 7)**: ユーザーが開いていた `commune_ws.rs` において、DB クエリエラーの silent suppression（`.unwrap_or(None)`）2箇所、Lamport Clock 同期結果の `let _ =` 破棄1箇所を発見しました。
4. **`cargo audit` 新規アドバイザリ**: `memmap2` に `RUSTSEC-2026-0186`（未チェックのポインタオフセット）が新たに報告されています。
5. **LOC 数・テスト定義数の正確な再計測**: Rust ~143k LOC, TypeScript ~33k LOC。テスト定義数は `#[test]` / `#[tokio::test]` 属性ベースで **1,347件**、`cargo test` 実行ベースで **4,524件**（パラメタライズ含む）。

---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **WebGL カラーコードと tokens.css のブリッジ化 (U-002)** | 🔴 | `BiomeCellGrid.tsx:35-44` および `BiomeGame.tsx:248-255` における 8元素カラーの HEX ハードコード。テーマ切替が WebGL 描画に反映されない。 | 5h | — |
| **P2** | **フロントエンド型安全性 (as any) の解消** | 🔴 | `WorkflowBuilder.tsx:101,234,271` / `workflowConverter.ts:139` / `api_resolver.ts:25` の計5箇所。型チェックが無効化され、ランタイムエラーの温床。 | 4h | — |
| **P3** | **Tauri IPC 構造体の TypeScript 自動生成同期化** | 🟡 | `src-tauri/src/lib.rs` の Rust 構造体と `management-console/src/types/` の TypeScript interface が手動同期。`ts-rs` 等による自動生成パイプライン未導入。 | 5h | — |
| **P4** | **`skills/mod.rs` (1,134行) God Module の分解** | 🟡 | テストを除いた純粋な本番コードが 1,134行。スキル登録、正規表現マッチング、ディスパッチが1ファイルに密結合。 | 4h | `[NEW]` |
| **P5** | **Error 型の統一 (10種類 → 3階層)** | 🟡 | `thiserror` 7ファイル vs `anyhow` 47ファイルの混在。ただし `error.rs` の変換層 (22テスト) は模範的な設計。 | 6h | — |

> **`[RESOLVED]` — 旧P4 (`discovery.rs` God Module)**: 以前 1,113行と報告していた `skills/discovery.rs` は現在 305行に縮小されています。

---

## 3. Quick Wins（1時間以内で修正可能）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-7** | `BiomeEventToast.tsx` のインライン styles から HEX フォールバックを排除 | `BiomeEventToast.tsx:48-53` | tokens.css 準拠 | `[RESOLVED]` |
| **QW-8** | `api_resolver.ts` 内の `window as any` をグローバル宣言 or `typeof window` 型ガードへ | `api_resolver.ts:25` | 型安全性 | `[RESOLVED]` |
| **QW-9** | `dispatcher.rs` の `.ok()` エラー抑制に警告ログ出力を追加 | `dispatcher.rs:134` | デバッグアビリティ | `[RESOLVED]` |
| **QW-10** | `auth.rs` の管理者ハッシュパース失敗時に `warn!` ログを出力 | [auth.rs:142](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L142) | 認証失敗の可観測性確保 | — |
| **QW-11** | `commune_ws.rs` の `.unwrap_or(None)` 2箇所を `match` + `warn!` に置換 | [commune_ws.rs:93](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L93), [commune_ws.rs:112](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L112) | DB クエリ失敗時の可観測性確保 | `[NEW]` |
| **QW-12** | `commune_ws.rs` の `sync_local_clock` 結果の `let _ =` を `if let Err(e) = ... { warn!(...) }` に | [commune_ws.rs:271](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L271) | クロック同期失敗時のログ出力 | `[NEW]` |
| **QW-13** | `heartbeat.rs` の設定取得 `.ok().flatten()` を `match` + `warn!` に置換 | [heartbeat.rs:115](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/heartbeat.rs#L115) | 設定DB接続失敗の可観測性確保 | `[NEW]` |
| **QW-14** | `expression.rs` の TTS voice 設定取得 `.unwrap_or(None)` にログ出力追加 | [expression.rs:162](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/expression.rs#L162) | TTS 設定フォールバック時の透過性 | `[NEW]` |

---

## 4. Findings Table（モジュール別・12次元スキャン）

### 4.1. `libs/infrastructure` (インフラ・共通コア)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 |
|---|---|---|---|---|
| **Dim 1: Architectural decay** | `skills/mod.rs` が 1,134行の God Module。登録・マッチング・ディスパッチの3責務が密結合。 | [skills/mod.rs:1-1134](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/skills/mod.rs) | 🟡 Medium | 4h |
| **Dim 3: Type & Contract Debt** | `JobQueue` トレイト定義と `UniversalJobQueue` の API 乖離。 | [traits.rs:300-370](file:///Users/motista/Desktop/antigravity/aiome/libs/aiome-core-contracts/src/traits.rs#L300) | 🟡 Medium | 3h |
| **Dim 4: Test debt** | `immune_system.rs` 内にテスト専用の巨大 `MockJQ`（約700行）がインラインで定義。他モジュールのテストでも再利用可能な共通テストユーティリティとして切り出すべき。 | [immune_system.rs:313-1015](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/immune_system.rs#L313) | 🟡 Medium | 2h |
| **Dim 5: Dependency debt** | `cargo audit`: `memmap2` に `RUSTSEC-2026-0186`（未チェックのポインタオフセット）。`proc-macro-error2` に `RUSTSEC-2026-0173`（unmaintained）。いずれも `allowed` 扱いだが定期的なレビューが必要。 | `Cargo.lock` (transitive) | 🟡 Medium | 1h |
| **Dim 7: Error handling** | `dispatcher.rs:134` の `.ok()` によるログなきエラー抑制。 | [dispatcher.rs:134](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/task_orchestrator/dispatcher.rs#L134) | 🟡 Medium | 0.5h |
| **Dim 9: Documentation drift** | 未ドキュメントの `pub fn` が **181件** (`deep-scan.sh` レポート)。公開 API のうちドキュメントコメントが欠損。 | `deep-scan.sh` レポート | 🟢 Low | 4h |

### 4.2. `apps/api-server` (API バックエンド)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 |
|---|---|---|---|---|
| **Dim 7: Error handling** | 管理者パスワードハッシュパースエラー時の警告ロギング欠落（静かな失敗）。 | [auth.rs:142](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L142) | 🟡 Medium | 0.5h |
| **Dim 7: Error handling** | `commune_ws.rs`: DB クエリ(`sql_fetch_optional!`)結果の `.unwrap_or(None)` による silent error suppression（2箇所）。チャネルID 取得失敗やノード秘密鍵取得のDB エラーが握り潰される。 | [commune_ws.rs:93](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L93), [commune_ws.rs:112](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L112) | 🟡 Medium | 0.5h |
| **Dim 7: Error handling** | `commune_ws.rs`: Lamport Clock 同期結果の `let _ =` 破棄。同期失敗時に警告なし。 | [commune_ws.rs:271](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L271) | 🟢 Low | 0.3h |
| **Dim 7: Error handling** | `heartbeat.rs`: 設定DB 取得の `.ok().flatten()` によるエラー抑制。 | [heartbeat.rs:115](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/heartbeat.rs#L115) | 🟢 Low | 0.3h |
| **Dim 7: Error handling** | `expression.rs`: TTS voice 設定取得の `.unwrap_or(None)` によるエラー抑制。 | [expression.rs:162](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/expression.rs#L162) | 🟢 Low | 0.3h |
| **Dim 8: Security hygiene** | `vault.rs` への `Authenticated` 型制約追加。 | `vault.rs` | 🟢 Resolved | — |

### 4.3. `apps/management-console` (フロントエンド)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 |
|---|---|---|---|---|
| **Dim 3: Type & Contract Debt** | `WorkflowBuilder.tsx` の `as any` キャスト 3箇所。 | [WorkflowBuilder.tsx:101, 234, 271](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/components/WorkflowBuilder.tsx#L101) | 🟡 Medium | 1.5h |
| **Dim 3: Type & Contract Debt** | `workflowConverter.ts` の動的プロパティアクセスに対する `as any` キャスト。 | [workflowConverter.ts:139](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/workflowConverter.ts#L139) | 🟡 Medium | 1h |
| **Dim 11: Tauri IPC 型安全性** | `window` グローバル拡張オブジェクトアクセス時の `as any` キャスト。 | [api_resolver.ts:25](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/api_resolver.ts#L25) | 🟡 Medium | 0.5h |
| **Dim 11: Tauri IPC 型安全性** | Rust 構造体との共有型定義（TypeScript interface）の手動同期。`ts-rs` 等による自動型ブリッジ同期の欠如。 | `management-console/src/types/` | 🟡 Medium | 5h |
| **Dim 12: tokens.css 遵守度** | 8元素カラーの WebGL `THREE.Color` 内での HEX ハードコード。 | [BiomeCellGrid.tsx:35-44](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeCellGrid.tsx#L35) | 🔴 High | 3h |
| **Dim 12: tokens.css 遵守度** | 元素カラーマッピング用 HEX リテラルの直書き。 | [BiomeGame.tsx:248-255](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeGame.tsx#L248) | 🔴 High | 2h |
| **Dim 12: tokens.css 遵守度** | HUD ネオングロー用のインライン HEX フォールバック。 | [BiomeHUD.tsx:98](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeHUD.tsx#L98) | 🟡 Medium | 0.5h |

---

## 5. Things that look bad but are actually fine

- **`allow-anti-pattern` による expect / unwrap の使用**:
  - [secret_redactor.rs:30](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/security/secret_redactor.rs#L30): 静的正規表現リテラルの `Regex::new().expect()`。入力依存ではないため許容。
  - [cortex_ingester.rs:210](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/cortex_ingester.rs#L210): タイトルパース用正規表現。同上。
  - [http.rs:23](file:///Users/motista/Desktop/antigravity/aiome/libs/core/src/http.rs#L23): Reqwest グローバルクライアントビルド失敗。TLS 環境の致命的な起動時エラー。
- **テスト用モジュールにおける unwrap() / expect()**:
  - [validator.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/validator.rs), [workflow/mod.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/workflow/mod.rs): `deep-scan.sh` が警告を出力するが、テスト関数内でのみ使用。本番 Zero-Panic 規則の安全除外対象。
- **非同期ライフサイクル監視における tokio::spawn の使用**:
  - [supervisor.rs:41](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/supervisor.rs#L41): `CancellationToken` と統合された自動再起動・Fail-Closed シャットダウン設計により、リーク・ゾンビリスクは排除。
- **`heartbeat.rs` の `let _ = event_sender.send(...)` (3箇所)**:
  - [heartbeat.rs:76, 99, 141](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/heartbeat.rs#L76): `broadcast::Sender::send()` の戻り値（受信者不在時の `SendError`）の意図的な破棄。受信者がいない場合でもイベント送信側に影響がないため許容。
- **大ファイルに見えるが実はテストが大半**:
  - `workflow/mod.rs` (1,501行): 本番コード約12行、99%がテスト。
  - `stripe/mod.rs` (1,273行): 本番コード約16行、98%がテスト。
  - `immune_system.rs` (1,015行): 本番コード約312行、69%がテスト。ただし MockJQ の共有化は検討価値あり (→ Dim 4)。
  - `society_of_thought.rs` (1,330行): 本番コード約765行、42%がテスト。分解の閾値未満。

---

## 6. Open Questions

1. **JobQueue トレイトの API 乖離**:
   `UniversalJobQueue` にのみ定義されている多数のパブリック補助メソッドについて、トレイト側 (`traits.rs`) に引き上げるか、`crate` プライベート化で封じるか？
2. **WebGL / Canvas テーマカラー同期方法**:
   CSS 変数を JS 上で読み取って `THREE.Color` を動的に生成する memoized bridge クラスを新規作成する方針で進めてよいか？
3. **Tauri IPC 構造体の自動型共有**:
   `ts-rs` を用いて Rust 構造体から TypeScript 定義をビルド時に自動出力するパイプラインを構築してよいか？
4. **`immune_system.rs` 内の巨大 MockJQ の共有化**:
   テスト用 `MockJQ` (約700行) を `tests/common/` や `libs/test-utils` クレートとして切り出し、他モジュールのテストから再利用可能にすべきか？

---

## 7. メトリクス推移

| 指標 | v7.0 | v8.0 | v8.1 | v8.5 | v9.0 (2026-06-29) | トレンド |
|---|---|---|---|---|---|
| 総 LOC | 152k | 152k | 152k | 152k | **175k** (Rust 143k + TS 33k) | ↑ 再計測で正確化 |
| Rust テスト定義数 (`#[test]`) | — | — | — | — | **1,347** | 新規指標 |
| `cargo test` 実行パス数 | 4,459 | 4,524 | 4,524 | 4,524 | **4,524** | → |
| U-002 違反 (TSX/WebGL) | 0 | 0 | 12 | 12 | **12** | → |
| `as any` 本番使用 (TS) | 1 | 1 | 5 | 5 | **5** | → |
| CC-6 違反 (Auth) | 0 | 6 | 0 | 0 | **0** | ✅ 完全解消維持 |
| Silent error suppression | 0 | 1 | 1 | 1 | **6** (詳細特定) | ↑ 深掘りで正確化 |
| God Module (本番1k+行) | 3 | 3 | 3 | 3 | **1** (`skills/mod.rs`) | ↓ 訂正・改善 |
| `cargo audit` allowed warnings | — | — | — | — | **2** | 新規指標 |
| 未ドキュメント pub fn | — | — | — | — | **181** | 新規指標 |

> **メトリクス訂正について**: 
> - 総 LOC は前バージョンまで ~152k と記載していましたが、TypeScript を正確にカウントした結果 175k でした。
> - God Module 数は前バージョンまで 3件と記載していましたが、テスト比率を精査した結果、本番コード 1,000行超は `skills/mod.rs` の **1件のみ** でした。
> - Silent error suppression は前バージョンまで 1件 (dispatcher.rs) のみ記載していましたが、`commune_ws.rs`, `heartbeat.rs`, `expression.rs` を含む **6件** を正確に特定しました。

---

## 8. Git ホットスポット（過去3ヶ月）

参考として、変更頻度の高い上位10ファイルを記録します。

| # | ファイル | コミット数 | LOC | リスク評価 |
|---|---|---|---|---|
| 1 | `api_integration_tests.rs` | 70 | — (分割済) | テスト充実。低リスク。 |
| 2 | `bootstrap.rs` | 59 | — (分割済) | 起動時初期化。リファクタ完了。 |
| 3 | `router.rs` | 49 | 857 | 154ルート定義。列挙のみのためリスク低。 |
| 4 | `app_state.rs` | 47 | 247 | アプリ状態管理。適切なサイズ。 |
| 5 | `infrastructure/lib.rs` | 36 | 206 | モジュール re-export。低リスク。 |
| 6 | `samsara-hub/main.rs` | 36 | 455 | Federation Hub エントリポイント。 |
| 7 | `App.tsx` | 34 | 798 | フロントエンド最大ファイル。分割検討余地あり。 |
| 8 | `api-server/main.rs` | 33 | 147 | エントリポイント。低リスク。 |
| 9 | `stripe/mod.rs` | 32 | 1,273 (98%テスト) | テスト充実。低リスク。 |
| 10 | `api.rs` | 32 | 354 | API モジュール定義。低リスク。 |

---

*Generated by `/tech-debt-audit` workflow — 2026-06-29 v9.0 (全面再スキャン・ホットスポット検証・God Module 再評価完了)*
