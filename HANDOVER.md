# HANDOVER: Aiome First Penguin (v1.0) Release

## 🎯 現在のゴールとステータス
- **目標**: Aiome v1.0 (First Penguin) リリースに向けた、インフラの最終安定化、セキュリティ設定の完了、およびリリース前監査。
- **直前の状況**: `/perfect-plan` ワークフローを用いて実コードベースを徹底走査しました。過去の計画にあった「幻覚」や「致命的な欠落」を修正し、実行可能な**完全版計画書 `implementation_plan_v3.md`** を完成させた状態です。

## 🚨 計画走査で発見された重大な欠落（新計画 v3 にて対応済み）
以前の計画のまま進めていれば、本番環境で確実に障害が発生していました。以下の3点は新セッションで真っ先に対応（実装）する必要があります。
1. **`CELL_ID` Panic 問題**: `shadow-worker` は `CELL_ID` 環境変数が未設定だと起動直後に panic して即死します。Compose 定義への追加と `.env.example` の修正が必要です。
2. **gRPC 接続未配線**: `api-server` はデフォルトで `localhost:50051` の証明ゲートを見に行くため、Docker ネットワーク上の `shadow-worker` に到達不能でした。`api-server` 側に `SHADOW_CLONE_GRPC_HOST=shadow-worker` の明示的な配線が必要です。
3. **隠れた環境変数の波及漏れ**: `KEY_PROXY_URL`, `VAULT_SECRET`, `SHADOW_CLONE_GRPC_PORT` など、稼働に必要な環境変数をすべて洗い出しました。

## ⏸️ 凍結事項（DEFERRED）- 絶対遵守
> [!CAUTION]
> **OGP 画像 (`og:image`) およびプロモーション動画の埋め込みタスクは完全凍結中**です。
> ユーザーから「完成版のロゴ・音声素材」が提供されるまで、コードの変更を一切行ってはなりません。SNS のキャッシュ汚染によるブランド毀損を防ぐため、**仮画像やプレースホルダーでの代用は厳禁**です。

## ⏩ 新セッションでのネクストアクション
新しいセッションが開始されたら、以下の順序で実装フェーズに入ってください。

1. **`docker-compose.production.yml` の修正 (Priority 1)**
   - `shadow-worker` サービス定義の新規追加（`CELL_ID`, `KEY_PROXY_URL` などの環境変数を含む）。
   - `api-server` の `environment` に `SHADOW_CLONE_GRPC_HOST=shadow-worker` 等を追加し、`depends_on` に `shadow-worker` を追加。
2. **`.env.example` の修正 (Priority 1)**
   - `CELL_ID` のコメントを解除し、デフォルト値を設定。
3. **イメージタグの固定 (Priority 2)**
   - Compose 内の `ruri-embed-server:latest` を `v1.0.0-first-penguin` に変更。
4. **最終検証と法務文書調整 (Priority 3〜6)**
   - ワークスペース全体での `cargo build --release` 実行と `enforce_unwrap_deny.py` による Zero-Panic 確認。
   - `docs/legal/TERMS_OF_SERVICE.md` の "Beta" 表記を "Early Access" 等に修正。
   - LP への `og:url` のみ追加。
   - `/release-preflight` ワークフローの実行。

## 🛡️ 引き継ぐべき開発原則（AGENTS.md）
- **Scope Lock 原則**: 「続けろ」と言われても、勝手にフェーズ（計画→実装）を進めない。コード変更ツールを呼ぶ前に必ず「実装に移行してよいか？」とユーザーに承認を求めること。
- **Verification Protocol**: 実装後は必ず「正常系」→「異常系（意図的エラー注入）」→「復旧」の3段階テストを行うこと。「エラーが出なかった」は検証成功の証拠にはならない。
