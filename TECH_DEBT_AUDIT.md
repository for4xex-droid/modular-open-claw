# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-07-02 (v10.0 — MCP Patterns 導入後スキャン・ホットスポット検証・God Module 再評価)
**前回監査日**: 2026-06-29 (v9.0)
**対象コードベース**: **176k LOC** (Rust ~144k + TypeScript ~32k)
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh --ci`, Git hotspot analysis (3ヶ月), grep-based deep scan
**分析コミット**: `78c4308a`

---

## 1. Executive Summary

v10.0 では、前回の監査結果から MCP Patterns 論文知見の適用完了（ツール予算制限、説明品質チェック）を経たリポジトリの状態をスキャンしました。

### 主要な発見・修正

1. **quick-xml の新規脆弱性検出 (RUSTSEC-2026-0195, RUSTSEC-2026-0194)**:
   `cargo audit` にて `quick-xml` (tauri -> plist 依存) に2件のDoS/脆弱性が報告されています。Cargo.lock のアップデートが必要です。
2. **`unwrap_or_else(|_| loop {})` による危険なパニック検出回避 (Dim 10 違反)**:
   `libs/infrastructure/src/skills/mod.rs:164` の静的正規表現初期化において、パニック検出ツールをすり抜けるために `unwrap_or_else(|_| loop {})` が用いられ、エラー時に意図的に CPU 100% 無限ループに陥る DoS 誘発設計が残存しているのを特定しました。
3. **`apps/watchtower` の構造的ドリフト**:
   `deep-scan.sh` のスキャン対象および `SYSTEM_PANORAMA.md` に `apps/watchtower` の記述が残存していますが、実ディレクトリは廃止（api-serverに統合）されています。
4. **`biome-popup-entry.tsx` における U-002 違反**:
   WebGL 関連 of HEX ハードコード解消が進んだ一方で、`biome-popup-entry.tsx:36` で `background: '#030712'` インラインスタイルが残存しているのを確認しました。
5. **テストコード由来のディープスキャン偽陽性**:
   `validator.rs` や `workflow/mod.rs` での unwrap/expect はすべてテストモジュール内のアサーションであり、本番 Zero-Panic 規則には違反していないことを実証しました。


---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **quick-xml 脆弱性の修正 (RUSTSEC-2026-0195/0194)** | 🔴 | `tauri` の依存である `plist` -> `quick-xml` の脆弱性による DoS 等のリスク。`cargo update -p quick-xml` の適用。 | 1h | `[NEW]` |
| **P2** | **フロントエンド型安全性 (as any) の解消** | 🔴 | `WorkflowBuilder.tsx:101,234,271` および `workflowConverter.ts:139` の計4箇所。型チェックが無効化され、ランタイムエラーの温床。 | 4h | `[NEW]` |
| **P3** | **`biome-popup-entry.tsx` の HEX ハードコード (U-002)** | 🔴 | `biome-popup-entry.tsx:36` での `#030712` 直書き。テーマ同期ブリッジへの適合漏れ。 | 0.5h | `[NEW]` |
| **P4** | **`skills/mod.rs` (1,134行) God Module の分解** | 🟡 | テストを除いた純粋な本番コードが 1,134行。スキル登録、正規表現マッチング、ディスパッチが1ファイルに密結合。 | 4h | `[NEW]` |
| **P5** | **Error 型 of 統一 (10種類 → 3階層)** | 🟡 | `thiserror` 7ファイル vs `anyhow` 47ファイルの混在。ただし `error.rs` の変換層 (22テスト) は模範的な設計。 | 6h | — |


---

## 3. Quick Wins（1時間以内で修正可能）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-15** | `cargo update -p quick-xml` による XML パース脆弱性の解消 | `Cargo.lock` | 依存関係セキュリティ強化 | `[NEW]` |
| **QW-16** | `biome-popup-entry.tsx` の HEX を `var(--bg-primary)` に置換 | `biome-popup-entry.tsx:36` | tokens.css 準拠 | `[NEW]` |
| **QW-17** | `deep-scan.sh` の CRATES から存在しない `apps/watchtower` を除外 | `scripts/deep-scan.sh:48` | スキャンスピード・整合性向上 | `[NEW]` |
| **QW-18** | `unwrap_or_else(|_| loop {})` 回避策を安全な `LazyLock::new` or `Result` 解析へ変更 | `skills/mod.rs:163-164` | DoSリスク(CPU100%ループ)排除 | `[NEW]` |
| **QW-7** | `BiomeEventToast.tsx` のインライン styles から HEX フォールバックを排除 | `BiomeEventToast.tsx:48-53` | tokens.css 準拠 | `[RESOLVED]` |

| **QW-8** | `api_resolver.ts` 内の `window as any` をグローバル宣言 or `typeof window` 型ガードへ | `api_resolver.ts:25` | 型安全性 | `[RESOLVED]` |
| **QW-9** | `dispatcher.rs` の `.ok()` エラー抑制に警告ログ出力を追加 | `dispatcher.rs:134` | デバッグアビリティ | `[RESOLVED]` |
| **QW-10** | `auth.rs` の管理者ハッシュパース失敗時に `warn!` ログを出力 | [auth.rs:142](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L142) | 認証失敗の可観測性確保 | `[RESOLVED]` |
| **QW-11** | `commune_ws.rs` の `.unwrap_or(None)` 2箇所を `match` + `warn!` に置換 | [commune_ws.rs:93](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L93), [commune_ws.rs:112](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L112) | DB クエリ失敗時の可観測性確保 | `[RESOLVED]` |
| **QW-12** | `commune_ws.rs` の `sync_local_clock`結果の `let _ =` を `if let Err(e) = ... { warn!(...) }` に | [commune_ws.rs:271](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L271) | クロック同期失敗時のログ出力 | `[RESOLVED]` |
| **QW-13** | `heartbeat.rs` の設定取得 `.ok().flatten()` を `match` + `warn!` に置換 | [heartbeat.rs:115](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/heartbeat.rs#L115) | 設定DB接続失敗の可観測性確保 | `[RESOLVED]` |
| **QW-14** | `expression.rs` の TTS voice 設定取得 `.unwrap_or(None)` にログ出力追加 | [expression.rs:162](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/expression.rs#L162) | TTS 設定フォールバック時の透過性 | `[RESOLVED]` |

---

## 4. Findings Table（モジュール別・12次元スキャン）

### 4.1. `libs/infrastructure` (インフラ・共通コア)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 | Status |
|---|---|---|---|---|---|
| **Dim 1: Architectural decay** | `skills/mod.rs` が 1,134行の God Module。登録・マッチング・ディスパッチの3責務が密結合。 | [skills/mod.rs:1-1134](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/skills/mod.rs) | 🟡 Medium | 4h | |
| **Dim 1: Architectural decay** | `deep-scan.sh` のスキャン対象に、廃止済みの `apps/watchtower` のパスが残存。 | [deep-scan.sh:48](file:///Users/motista/Desktop/antigravity/aiome/scripts/deep-scan.sh#L48) | 🟡 Medium | 1h | `[NEW]` |
| **Dim 3: Type & Contract Debt** | `JobQueue` トレイト定義と `UniversalJobQueue` の API 乖離。 | [traits.rs:300-370](file:///Users/motista/Desktop/antigravity/aiome/libs/aiome-core-contracts/src/traits.rs#L300) | 🟡 Medium | 3h | |
| **Dim 4: Test debt** | `immune_system.rs` 内にテスト専用の巨大 `MockJQ`（約700行）がインラインで定義。他モジュールのテストでも再利用可能な共通テストユーティリティとして切り出すべき。 | [immune_system.rs:313-1015](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/immune_system.rs#L313) | 🟡 Medium | 2h | |
| **Dim 5: Dependency debt** | `cargo audit`: `quick-xml` 脆弱性（RUSTSEC-2026-0195, 2026-0194）の混入。`tauri` -> `plist` 経由。 | `Cargo.lock` | 🔴 High | 1h | `[NEW]` |
| **Dim 5: Dependency debt** | `cargo audit`: `memmap2` に `RUSTSEC-2026-0186`（未チェック of ポインタオフセット）。`proc-macro-error2` に `RUSTSEC-2026-0173`（unmaintained）。いずれも `allowed` 扱いだが定期的なレビューが必要。 | `Cargo.lock` (transitive) | 🟡 Medium | 1h | |
| **Dim 7: Error handling** | `dispatcher.rs:134` の `.ok()` によるログなきエラー抑制。 | [dispatcher.rs:134](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/task_orchestrator/dispatcher.rs#L134) | 🟡 Medium | 0.5h | `[RESOLVED]` |
| **Dim 9: Documentation drift** | 未ドキュメントの `pub fn` が **181件** (`deep-scan.sh` レポート)。公開 API のうちドキュメントコメントが欠損。 | `deep-scan.sh` レポート | 🟢 Low | 4h | |
| **Dim 10: Zero-Panic Policy 形骸化** | `unwrap_or_else(|_| loop {})` による、パニック検出回避と引き換えの無限ループ (CPU 100% DoS) 設計。 | [skills/mod.rs:163-164](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/skills/mod.rs#L163) | 🔴 High | 0.5h | `[NEW]` |
| **Dim 11: Tauri IPC 型安全性** | Rust 構造体との共有型定義（TypeScript interface）の手動同期。`ts-rs` 等による自動型ブリッジ同期の欠如。 | `management-console/src/types/` | 🟡 Medium | 5h | `[RESOLVED]` |


### 4.2. `apps/api-server` (API バックエンド)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 | Status |
|---|---|---|---|---|---|
| **Dim 7: Error handling** | 管理者パスワードハッシュパースエラー時の警告ロギング欠落（静かな失敗）。 | [auth.rs:142](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L142) | 🟡 Medium | 0.5h | `[RESOLVED]` |
| **Dim 7: Error handling** | `commune_ws.rs`: DB クエリ(`sql_fetch_optional!`)結果の `.unwrap_or(None)` による silent error suppression（2箇所）。チャネルID 取得失敗やノード秘密鍵取得のDB エラーが握り潰される。 | [commune_ws.rs:93](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L93), [commune_ws.rs:112](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L112) | 🟡 Medium | 0.5h | `[RESOLVED]` |
| **Dim 7: Error handling** | `commune_ws.rs`: Lamport Clock 同期結果の `let _ =` 破棄。同期失敗時に警告なし。 | [commune_ws.rs:271](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/commune_ws.rs#L271) | 🟢 Low | 0.3h | `[RESOLVED]` |
| **Dim 7: Error handling** | `heartbeat.rs`: 設定DB 取得の `.ok().flatten()` によるエラー抑制。 | [heartbeat.rs:115](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/heartbeat.rs#L115) | 🟢 Low | 0.3h | `[RESOLVED]` |
| **Dim 7: Error handling** | `expression.rs`: TTS voice 設定取得の `.unwrap_or(None)` によるエラー抑制。 | [expression.rs:162](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/expression.rs#L162) | 🟢 Low | 0.3h | `[RESOLVED]` |
| **Dim 8: Security hygiene** | `vault.rs` への `Authenticated` 型制約追加。 | `vault.rs` | 🟢 Resolved | — | `[RESOLVED]` |

### 4.3. `apps/management-console` (フロントエンド)
| 次元 | 指摘内容 | 対象ファイルと行数 | 深刻度 | 見積もり工数 | Status |
|---|---|---|---|---|---|
| **Dim 3: Type & Contract Debt** | `WorkflowBuilder.tsx` の `as any` キャスト 3箇所。 | [WorkflowBuilder.tsx:101, 234, 271](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/components/WorkflowBuilder.tsx#L101) | 🟡 Medium | 1.5h | |
| **Dim 3: Type & Contract Debt** | `workflowConverter.ts` の動的プロパティアクセスに対する `as any` キャスト。 | [workflowConverter.ts:139](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/workflowConverter.ts#L139) | 🟡 Medium | 1h | |
| **Dim 11: Tauri IPC 型安全性** | `window` グローバル拡張オブジェクトアクセス時の `as any` キャスト。 | [api_resolver.ts:25](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/api_resolver.ts#L25) | 🟡 Medium | 0.5h | `[RESOLVED]` |
| **Dim 12: tokens.css 遵守度** | `biome-popup-entry.tsx` の背景色 `#030712` のインライン HEX ハードコード。 | [biome-popup-entry.tsx:36](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/biome-popup-entry.tsx#L36) | 🔴 High | 0.5h | `[NEW]` |
| **Dim 12: tokens.css 遵守度** | 8元素カラーの WebGL `THREE.Color` 内での HEX ハードコード。 | [BiomeCellGrid.tsx:35-44](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeCellGrid.tsx#L35) | 🔴 High | 3h | `[RESOLVED]` |
| **Dim 12: tokens.css 遵守度** | 元素カラーマッピング用 HEX リテラルの直書き。 | [BiomeGame.tsx:248-255](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeGame.tsx#L248) | 🔴 High | 2h | `[RESOLVED]` |
| **Dim 12: tokens.css 遵守度** | HUD ネオングロー用のインライン HEX フォールバック。 | [BiomeHUD.tsx:98](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeHUD.tsx#L98) | 🟡 Medium | 0.5h | `[RESOLVED]` |


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
5. **quick-xml 脆弱性の解消方針**:
   `Cargo.lock` での `cargo update -p quick-xml` の適用により、Tauri 経由の脆弱性を安全に解消してよいか？
6. **`apps/watchtower` のスキャンパス除外**:
   `deep-scan.sh` の `CRATES` 設定から存在しない `apps/watchtower` を安全に削除してよいか？

---


## 7. メトリクス推移

| 指標 | v8.0 | v8.1 | v8.5 | v9.0 | v10.0 (2026-07-02) | トレンド |
|---|---|---|---|---|---|---|
| 総 LOC | 152k | 152k | 152k | 175k | **176k** (Rust 144k + TS 32k) | ↑ 微増 |
| Rust テスト定義数 (`#[test]`) | — | — | — | 1,347 | **1,348** | ↑ +1 (TDD適用) |
| `cargo test` 実行パス数 | 4,524 | 4,524 | 4,524 | 4,524 | **4,525** | ↑ +1 (TDD適用) |
| U-002 違反 (TSX/WebGL) | 0 | 12 | 12 | 0 | **1** (`biome-popup-entry`) | ⚠️ 残存1件検出 |
| `as any` 本番使用 (TS) | 1 | 5 | 5 | 5 | **5** | → |
| CC-6 違反 (Auth) | 6 | 0 | 0 | 0 | **0** | ✅ 完全解消維持 |
| Silent error suppression | 1 | 1 | 1 | 0 | **0** | ✅ 完全解消維持 |
| God Module (本番1k+行) | 3 | 3 | 3 | 1 | **1** (`skills/mod.rs`) | → |
| `cargo audit` warnings | — | — | — | 2 | **4** (quick-xml+allowed) | ⚠️ 依存関係脆弱性追加 |
| 未ドキュメント pub fn | — | — | — | 181 | **388** (全クレート合計) | ↑ 網羅的な再集計 |

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
