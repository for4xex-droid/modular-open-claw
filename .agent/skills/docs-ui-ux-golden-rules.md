---
name: docs-ui-ux-golden-rules
description: ドキュメント操作・UI/UX開発・ビルド検証時にAIエージェントが絶対に遵守すべき行動規範。違反は即ユーザーの時間浪費。
---

# 📜 Golden Rules: Documentation & UI/UX

> **このルールは、エージェントがドキュメント作成やUI/UX改修で「アホになる」
> 悪癖を根絶するために制定された**絶対的な行動規範**です。**
> 違反は直ちにユーザーの時間の浪費と信頼毀損を引き起こします。

---

## Part A: ドキュメント操作ルール

### D-001: 存在確認の義務 (Verify Before Touch) [CRITICAL]
ファイルの作成・削除・移動・Git操作の前には、`list_dir` / `view_file` / `git status` で対象の実在を確認せよ。Artifacts（`brain/` や `~/.gemini/`）内の一時ファイルをプロジェクトの実ファイルと混同してはならない。

### D-002: 生成サイズ制限 (Chunk Generation) [CRITICAL]
50行を超える新規ドキュメントを1回の `write_to_file` で出力してはならない。スクリプト結合（`cat`）か、セクション分割による複数回追記を使え。

### D-003: 機密ドキュメントの絶対保護 (Secret Zone) [HIGH]
戦略・企業価値・ビジネスプランは `docs/vision/` のみに配置。他のGit管理下ディレクトリに置くな。

### D-004: 事実ベースの記述 (No Hallucination) [HIGH]
ドキュメントに書く構造体・API・アーキテクチャ仕様は、必ず `grep_search` や `view_file` で現行コードの実在を確認してから記述せよ。未実装は `[Planned]` を明示。

---

## Part B: ビルド・実行検証ルール

### B-001: 実装後の即時ビルド検証 (Build After Every Change) [CRITICAL]
**コード変更をした直後に、必ず以下のコマンドで検証せよ。**
- Rust: `cargo check --workspace --tests`
- Frontend: `cd apps/management-console && npm run lint`

### B-002: コマンド出力の完全確認 (Read Every Output) [CRITICAL]
**`run_command` を実行した後、そのエラー出力を必ず読め。`command_status` で結果を確認せよ。**
コマンドが失敗した事実を無視して進めるな。

### B-003: マイグレーション同期の義務 (Dual Migration Sync) [HIGH]
**DBスキーマを変更した場合、SQLite と PostgreSQL の両方のマイグレーションを同期せよ。**
- `libs/infrastructure/src/job_queue/migrations.rs` (SQLite)
- `libs/infrastructure/migrations/` (PostgreSQL)

### B-004: Mockの cfg ガード強制 (Mock Isolation) [HIGH]
**Mock構造体は例外なく `#[cfg(any(test, debug_assertions))]` で囲え。**
`cargo check --release` で参照不可能であることを確認。

> NG実例: `MockPromptRegistry` が本番コードの fallback として参照されており、`NoopPromptRegistry` への改名・分離を要した（出典: CHANGELOG）。「Mock」と名の付くものが release ビルドに到達した時点で違反。

### B-005: バックグラウンドプロセス管理 (Process Lifecycle) [HIGH]
**`Command::new()` で子プロセスを起動する場合の必須設定:**
- `.kill_on_drop(true)`
- タイムアウト設定
- `Stdio::null()` などのパイプ管理

---

## Part C: UI/UX 開発ルール

### U-001: Tailwindの絶対禁止 (No Tailwind) [CRITICAL]
このプロジェクトにTailwindは存在しない。ユーティリティクラス (`flex`, `mb-4` 等) を書くな。Vanilla CSS + CSS変数 (`tokens.css`) のみを使用せよ。

### U-002: デザイントークンの全参照 (Honor the Tokens) [CRITICAL]
色・隙間・角丸・レイアウト幅の生値（`#00f2ff`, `12px`, `280px`）のハードコード禁止。すべて `var(--xxx)` で参照。
検証コマンド: `python3 scripts/test_ui_hex_violations.py`（0 違反で合格）。

> 実績: 2026-06-10、Biome UI のインライン生値38箇所を tokens.css 参照へ置換し 0 違反を達成（出典: RIPPLE_MAP.md 2026-06-10 節）。
> NG実例: `biome-popup-entry.tsx` L36 の背景色 `#030712` 直書きが技術的負債として摘出された（出典: CHANGELOG [Unreleased] / OPEN.md OP-029）。

### U-003: 編集後の型チェック義務 (TSC After Edit) [HIGH]
TSXを編集したら `npm run lint` を即実行。通らなければ「完了」と報告するな。

### U-004: OpenAPI生成型の使用強制 (Generated Types Only) [HIGH]
APIレスポンスの型を手書きするな。`npm run generate-types` で `src/types/generated.ts` から取得。

### U-005: Canvas/3Dコンポーネントへの介入制限 (WebGL Safety) [HIGH]
`react-three-fiber` / `vis-network` を含むファイルを編集する際、`useFrame` 内で `setState` を呼ぶな。毎フレーム再レンダリングによるフリーズを避ける。

---

## Part D: 共通ルール

### X-001: 失敗時の根本原因特定義務 (Root Cause, Not Excuses) [CRITICAL]
操作が失敗したら「別の方法で」と逃げるな。`git status` / `list_dir` / エラーログを確認し、**なぜ失敗したかを正確に特定してから**次のアクションを取れ。

### X-002: 分割指示の絶対服従 (Chunked Execution) [HIGH]
「分割しろ」と指示されたら、再度一括処理を走らせるな。1ステップずつ実行・完了確認・報告を繰り返せ。
