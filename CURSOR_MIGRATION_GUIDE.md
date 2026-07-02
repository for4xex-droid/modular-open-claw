# 🚀 Cursor への移行プロセスと検証ガイド (CURSOR_MIGRATION_GUIDE.md)

現在このワークスペースで稼働している開発AIとしての行動規範、安全ルール、およびツール環境を **Cursor (Composer / Agent モード)** に完全に移植し、同等の精度と安全性で開発を継続するための検証プロセスと手順です。

---

## 1. 設定の移植（`AGENTS.md` への一本化）

Cursor はプロジェクトのルートディレクトリにある `AGENTS.md` を自動で読み込み、AIへの指示（システムプロンプト）にブレンドします。

> [!NOTE]
> 2026-07-03 の整理で、`.cursorrules`（AGENTS.md の短縮英語版）は**廃止・削除**しました。
> 行動規範・Safety-Critical Zone・Verification Protocol はすべて `AGENTS.md` が単一の情報源（Single Source of Truth）です。
> `.cursorrules` を再作成すると二重定義による指示競合と常時トークン消費の増加を招くため、再作成しないでください。

---

## 2. MCP サーバーの Cursor 連携

本プロジェクトは独自の MCP サーバー（`deep-dive-local`）などを活用しています。Cursor でもこれらのツールが動作するよう、設定を紐付けます。

### 移行手順:
1. Cursor の `Settings (⚙️) -> Models -> MCP` を開きます。
2. `+ Add New MCP Server` をクリックします。
3. 以下の設定を入力し、連携を完了させます：
   * **Name**: `deep-dive-local`
   * **Type**: `command` (または `sse` などの対応方式に応じて設定)
   * **Command**: 現在の MCP サーバーの起動コマンド（`/Users/motista/.gemini/antigravity-ide/` 付近にある MCP サーバーバイナリや Node スクリプトの起動パス）を指定します。

---

## 3. ワークフロー機能の Cursor での代替

Antigravity IDE で提供されているスラッシュコマンド（ワークフロー）は、Cursor の以下の機能にマッピングして実行します。

| Antigravity 側機能 | Cursor での代替・実行方法 |
|---|---|
| `/goal` (長時間・徹底実行) | Cursor **Composer (Cmd + I)** で Agent モード（または Yolo モード）を選択して実行。 |
| `/perfect-plan` (計画スキャン) | Composer 内で計画の作成のみを指示し、編集結果が自動適用される前に「Accept/Reject」で確認する。 |
| `SafeToAutoRun` コマンド承認 | Cursor の Terminal 設定で、AI によるコマンド自動実行に対するセキュリティレベルを設定（「Require approval」に設定することを強く推奨）。 |

---

## 4. 移行プロセス実行チェックリスト

乗り換えの準備ができたら、以下のチェックリストを順に実行してください。

1. **[ ] 設定ファイルの確認**:
   * `aiome/` ルートに `AGENTS.md` が存在することを確認（`.cursorrules` は廃止済み。再作成しない）。
   * `aiome.code-workspace` を Cursor で開き、マルチルート開発環境を有効化する。
2. **[ ] 環境変数の引き継ぎ**:
   * `.env` および `.env.secret` が Cursor のターミナル環境やインテグレーション側から正しく読み込めるか確認する。
   * **重要**: macOS の GUI アイコンから直接 Cursor を起動すると、システム側の環境変数（`GEMINI_API_KEY` 等）が欠損する可能性があります。環境変数を確実に引き継ぐため、普段お使いのシェルターミナルから `cursor /Users/motista/Desktop/antigravity/aiome/aiome.code-workspace` コマンドを使用してマルチルートワークスペース（Aiome 本体・Nurture `commercial/`・Management Console）を起動することを強く推奨します。
3. **[ ] データベース認証の確認**:
   * 開発ログイン用の `aiome.db` 上のパスワード（`SuperSecretPassword123!`）が正しく認識され、ログインができるか Cursor 内のターミナルからテストサーバー（Tauri側）を起動して確認（詳細手順: [docs/guides/LOCAL_LOGIN_VERIFICATION.md](docs/guides/LOCAL_LOGIN_VERIFICATION.md)）。
4. **[ ] ビルドおよびテストの実行テスト**:
   * `cargo check --workspace` および `cargo test --workspace` を Cursor ターミナルから実行し、正常終了することを確認する。
