# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-06-29 (v8.3 — エラー抑制調査・テスト unwrap の妥当性・非同期ライフサイクル監視の検証)
**前回監査日**: 2026-06-29 (v8.2 — 12次元長期負債・フロントエンド型安全性・WebGLカラーブリッジ負債の監査)
**対象コードベース**: **152k LOC** (Rust ~128k + TypeScript ~24k)
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh`, Git hotspot analysis, grep-based deep scan
**分析コミット**: `e88704a3`

---

## 1. Executive Summary

直近で発生した Quick Wins（CC-6 違反、Zero-Panic 違反、環境変数不一致）が完全に解消されたことを受け、本フェーズでは**「12次元監査フレームワーク」**に基づくコードベースの深層分析を実施しました。

今回の監査により、以下の重大かつ長期的な技術的負債が浮き彫りになりました：
1. **WebGLコンテキストとtokens.cssの断絶 (Dimension 12)**: `BiomeCellGrid.tsx` や `BiomeGame.tsx` 等の Canvas/Three.js 描画ロジックにおいて、カラーパレット（`#4fc3f7` など）が HEX リテラルでハードコードされており、テーマ変更やデザイントークンの変更に追従できない構造になっています。
2. **フロントエンドにおける any キャストの乱用 (Dimension 3/11)**: `WorkflowBuilder.tsx` や `workflowConverter.ts` などの主要な UI ロジックで `as any` キャストが 5 箇所以上使用されており、Tauri バックエンドの型定義（Rust）との一貫性・型安全性を損なうリスクがあります。
3. **JobQueue トレイトと UniversalJobQueue の乖離 (Dimension 3)**: コアコントラクトの `JobQueue` インターフェース定義に対し、実装側でのみパブリックに公開されている補助メソッドが肥大化しており、トレイトによる抽象化が形骸化しています。
4. **ログ無しのエラー抑制 (.ok()) の存在 (Dimension 7)**: `dispatcher.rs` 内で、JSON シリアライズ失敗時のエラーがログ出力されることなく `.ok()` で静かに捨てられています。

---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **WebGL カラーコードと tokens.css のブリッジ化 (U-002)** | 🔴 | `BiomeCellGrid.tsx:35-44` および `BiomeGame.tsx:248-255` における WebGL 元素カラーの HEX ハードコードの排除と、memoized CSS 変数ブリッジの適用。 | 5h | `[NEW]` |
| **P2** | **フロントエンド型安全性 (as any) の解消** | 🔴 | `WorkflowBuilder.tsx` や `workflowConverter.ts` における `as any` キャストを、厳密な型定義または `unknown` 絞り込みにリファクタリング。 | 4h | `[NEW]` |
| **P3** | `mcp/discovery.rs` (1,113行) God Module の分解 | 🟡 | OAuth エンドポイント、トークン交換、MCP テンプレートが1ファイルに密結合。URL 変更時のリグレッションリスク。 | 6h | — |
| **P4** | **JobQueue トレイトの API 乖離の是正 (CC-1)** | 🟡 | `traits.rs:300-370` で定義されたトレイト契約と `UniversalJobQueue` の独自 API の整合性整理。 | 3h | `[NEW]` |
| **P5** | Error 型の統一 (10種類 → 3階層) | 🟡 | `thiserror` 7ファイル vs `anyhow` 47ファイルの混在。ただし `error.rs` の変換層 (22テスト) は模範的な設計。 | 6h | — |

---

## 3. Quick Wins（解消・新規追加）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-7** | `BiomeEventToast.tsx` のインライン styles から HEX フォールバックを排除 | `BiomeEventToast.tsx:48-53` | tokens.css への 100% 依存への準拠 | `[NEW]` |
| **QW-8** | `api_resolver.ts` 内の `window as any` をグローバル宣言または `typeof window` での型ガードへ移行 | `api_resolver.ts:25` | グローバルオブジェクトアクセスの型安全性確保 | `[NEW]` |
| **QW-9** | `dispatcher.rs` の `.ok()` エラー抑制に警告ログ出力を追加 | `dispatcher.rs:134` | JSON 変換失敗時のデバッグアビリティ向上 | `[NEW]` |

---

## 4. Findings Table（12次元別）

### Dimension 3: Type & Contract Debt（型・契約の負債）
- **フロントエンド `as any` による型定義の緩さ**:
  - [workflowConverter.ts:139](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/workflowConverter.ts#L139): `const details = (nodeType as any)[typeName]` (動的プロパティアクセスの any キャスト)
  - [WorkflowBuilder.tsx:101](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/components/WorkflowBuilder.tsx#L101): `const eventData = data as any`
  - [WorkflowBuilder.tsx:234](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/components/WorkflowBuilder.tsx#L234): `const nodeType = node.data?.node_type as any`
- **JobQueue トレイトの定義と UniversalJobQueue 実装の API 乖離 (CC-1)**:
  - [traits.rs:300-370](file:///Users/motista/Desktop/antigravity/aiome/libs/aiome-core-contracts/src/traits.rs): `JobQueue` 定義に対し、実装側でのみ公開された pub fn が多数あり、抽象インターフェースとしての契約が機能していません。

### Dimension 7: Error handling & observability
- **[NEW] ログ無しのエラー握り潰し (.ok() の使用)**:
  - [dispatcher.rs:134](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/task_orchestrator/dispatcher.rs#L134): `serde_json::to_string(&details_map).ok()`
  - **対策**: `warn!` や `debug!` ログを追加するか、コンテキスト付きのエラーハンドリングを追加する必要があります。

### Dimension 11: Tauri IPC 型安全性 (Aiome固有)
- **Tauri グローバルアクセス時の any 使用**:
  - [api_resolver.ts:25](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/api_resolver.ts#L25): `window && (window as any).__TAURI_INTERNALS__`
  - **対策**: グローバル window オブジェクトの拡張インターフェースを `global.d.ts` で定義する必要があります。

### Dimension 12: tokens.css 遵守度 / U-002 違反 (Aiome固有)
- **WebGL / Canvas コンテキストでの HEX カラーコードのハードコード**:
  - [BiomeCellGrid.tsx:35-44](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeCellGrid.tsx#L35): 元素表示用カラーの `new THREE.Color('#4fc3f7')` などのハードコード（8箇所）。
  - [BiomeGame.tsx:248-255](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeGame.tsx#L248): 元素カラーマッピング用 HEX リテラルの直書き。
  - [BiomeHUD.tsx:98](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/biome/BiomeHUD.tsx#L98): ネオングロー表示用のインライン HEX指定 `color: 'var(--accent-cyan, #06b6d4)'`。

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

## 6. メトリクス推移

| 指標 | v7.0 | v7.2 | v8.0 | v8.1 | v8.2 | v8.3 (2026-06-29) | トレンド |
|---|---|---|---|---|---|---|---|
| 総 LOC | 152k | 152k | 152k | 152k | 152k | **152k** | → |
| Rust テスト数 | 4,459 | 4,459 | 4,524 | 4,524 | 4,524 | **4,524** | → |
| U-002 違反 (TSX/WebGL) | 0 | 0 | 0 | 0 | 12 | **12** | 維持 |
| `as any` 本番使用 (TS) | 1 | 1 | 1 | 1 | 5 | **5** | 維持 |
| CC-6 違反 (Auth) | 0 | 0 | 6 | 0 | 0 | **0** | 完全解消維持 ✅ |
| ログなしエラー抑制 (.ok()) | 0 | 0 | 1 | 1 | 1 | **1** | 可視化 ⚠️ |
| God Module (1k+ 行) | 3 | 3 | 3 | 3 | 3 | **3** | 維持 |

---

*Generated by `/tech-debt-audit` workflow — 2026-06-29 v8.3 (エラー抑制調査・テスト unwrap の妥当性・非同期ライフサイクル監視の検証完了)*
