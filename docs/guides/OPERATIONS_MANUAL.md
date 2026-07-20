# Aiome Operations Manual — 実用運用ガイド
**Version:** 3.8  
**Last Updated:** 2026-07-21

---

## 1. Prerequisites (前提条件)

### 1.1 Hardware
- **推奨**: Mac mini M4 Pro (24GB RAM) 以上
- **Storage**: SSD 10GB+ (データ蓄積のため)

### 1.2 Software Dependencies

| Software | Version | Purpose | Install |
|----------|---------|---------|---------|
| Rust | 1.85+ | コア開発 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | Management Console UI | `brew install node` |
| Python | 3.10+ | ruri-v3 embedding server | `brew install python` |
| Ollama | Latest | バックグラウンドLLM | `brew install ollama` |
| SQLite | 3.40+ | DB (ビルトイン) | Rust `sqlx` に含まれる |

### 1.3 API Keys

| Key | 取得先 | 用途 |
|-----|--------|------|
| `GEMINI_API_KEY` | [Google AI Studio](https://aistudio.google.com/) | フロントエンド推論 (Gemini 2.5 Flash) |

### 1.4 LLM構成 (Pattern B: 推奨)

| 用途 | プロバイダー | モデル | コスト |
|------|-----------|--------|--------|
| フロントエンド (Agent Console) | Gemini Cloud | `gemini-2.5-flash` | 月≈100円 |
| バックグラウンド (Soul Mutator等) | Ollama (Local) | `qwen3.5:9b` | 無料 |

---

## 2. Initial Setup (初期セットアップ)

### 2.1 環境変数の設定

```bash
# プロジェクトルートの .env ファイルを編集
cp .env.example .env

# .env の主要設定:
GEMINI_API_KEY=your_gemini_key_here
BG_LLM_PROVIDER=ollama
BG_LLM_MODEL=qwen3.5:9b
API_SERVER_SECRET=your_random_secret_here_must_be_16_chars_min
VAULT_SECRET=your_vault_secret
FEDERATION_SECRET=your_hub_secret # [Deferred to v1.5]
TTS_PROVIDER=openai
TTS_OPENAI_API_KEY=your_openai_key
TTS_OPENAI_MODEL=tts-1
SEARCH_API_KEY=your_brave_search_or_tavily_api_key
X_BEARER_TOKEN=your_x_twitter_bearer_token
WP_API_URL=https://your-wordpress-site.com
WP_API_TOKEN=your_jwt_or_app_password

# --- MCP Tool API Keys ---
# Stripe MCP / discovery のプレースホルダは $STRIPE_API_KEY（api-server 正本）。
# Nurture 連盟の STRIPE_SECRET_KEY は別系統（混同禁止）。
STRIPE_API_KEY=sk_test_your_key_here
DISCORD_TOKEN=your_discord_bot_token
NOTION_API_KEY=your_notion_integration_token
X_API_KEY=your_x_api_key
X_API_SECRET=your_x_api_secret
X_ACCESS_TOKEN=your_x_access_token
X_ACCESS_TOKEN_SECRET=your_x_access_token_secret

# --- Polar Commerce API Keys (P-1 Webhook Integration) ---
POLAR_API_KEY=your_polar_api_key_here
POLAR_BASE_URL=https://sandbox-api.polar.sh
POLAR_WEBHOOK_SECRET=whsec_your_polar_webhook_secret_here

# --- System Alerts (Discord Integration) ---
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/xxxx/xxxx # Webhook URL for system alerts (e.g. Stripe webhook failures)

# --- オプション (デフォルト値あり) ---
AIOME_DB_PATH=sqlite://workspace/aiome.db       # DBパス
OLLAMA_HOST=http://127.0.0.1:11434              # Ollamaホスト
OLLAMA_MODEL=qwen3.5:9b                          # Ollamaモデル
PORT=3015                                        # APIサーバーポート
KEY_PROXY_URL=http://127.0.0.1:3017             # Abyss Vault URL
SAMSARA_HUB_REST=http://127.0.0.1:3016          # [Deferred to v1.5] Samsara Hub REST URL
SAMSARA_HUB_WS=ws://127.0.0.1:3016/api/v1/federation/ws  # [Deferred to v1.5] Hub WebSocket
ALLOWED_ORIGINS=http://localhost:1420,http://localhost:5173  # CORS許可オリジン
EMBEDDING_PROVIDER=ruri                          # 埋め込みプロバイダー (ruri/gemini/ollama)
RURI_EMBED_URL=http://localhost:8100             # RuriサーバーURL
ABYSS_VAULT_PATH=~/.aiome/abyss_vault            # APIキー物理隔離ディレクトリ
TIMESFM_AUTH_TOKEN=your_secure_token             # TimesFM 予測側認証トークン (必須)
TIMESFM_SIDECAR_URL=http://localhost:3020        # TimesFM サイドカーURL
CONTAINER_RUNTIME=podman                         # DockerConductor/ Delegator で強制利用するコンテナランタイム (podman or docker). 指定がない場合は podman 優先の自動フォールバック。
LOCAL_LLM_CONCURRENCY=2                          # Fast tier 用ローカルモデルへの同時実行セマフォ制限数 (デフォルト: 2)

# --- Nurture / Desktop Economy (OP-088 / OP-089) ---
# Desktop 製品既定は InProcess / Economy チャネル（通常は設定不要）。公式同梱 sidecar = api-server + key-proxy のみ。
# 配布チャネル: economy（--features nurture）/ oss（commercial 非リンク）→ docs/guides/DESKTOP_CHANNELS.md
# 正本: NURTURE_MODE=disabled|cloud|local|in_process
# NURTURE_MODE=
#   local      = nurture-api sidecar（開発。要 --with-nurture-sidecar。公式パッケージでは失敗しうる）
#   cloud      = NURTURE_CLOUD_URL 必須
#   disabled   = 経済オフ（Mock）
#   in_process = 明示 InProcess（未設定時の既定と同じ）
#
NURTURE_INTERNAL_SECRET=your_secret              # S2S 認証 + OXP 署名鍵（coin-charge / forget / stripe proxy 共通。API_SERVER_SECRET とは別）
# InProcess（既定）: Tauri が NURTURE_API_URL=http://127.0.0.1:3015 を注入（自己 HTTP）
# Local / Docker:   NURTURE_API_URL=http://127.0.0.1:3020（または http://nurture-api:3020）
# NURTURE_API_URL=http://127.0.0.1:3020
NURTURE_DRM_MASTER_KEY=your_drm_key              # release 必須。Desktop debug は {data_dir}/.nurture_drm_master_key を自動生成・永続化
# 互換（NURTURE_MODE 未設定時のみ）: NURTURE_CLOUD_URL / NURTURE_DISABLED / NURTURE_IN_PROCESS
# NURTURE_A2C_DRY_RUN=1                           # A2C 恩返し: 未設定/1/true=ログのみ。0 のみで Tremendous 実送信（Human 確認後）

# --- Security (Production) ---
A2A_NODE_TOKEN=your_a2a_token                    # release ビルド必須。未設定/空文字時は起動失敗
# production compose: api-server の A2A_NODE_TOKEN は A2A_AUTH_TOKEN をパススルー（同値）
# JWT_ISSUER=aiome                                # 設定時は JWT の iss を検証
# JWT_AUDIENCE=aiome-api                          # 設定時は JWT の aud を検証

# --- Data Directory Override ---
# AIOME_DATA_DIR=/path/to/custom/data             # CLIとTauriデスクトップアプリで同じデータ領域を参照するためのパスオーバーライド
```

> **Note**: すべての環境変数は `libs/shared/src/config.rs` の `AiomeConfig::load()` で一元管理されています。デフォルト値が設定されているため、必須のもの以外は未設定でも起動可能です。

### 2.2 ビルドと初期検証

```bash
# 1. ビルド
cargo build -p api-server

# 2. テスト実行
cargo test --workspace
```

---

## 3. Commands (コマンド一覧)

### 3.1 API Server 起動

```bash
RUST_LOG=info cargo run -p api-server
# → http://localhost:3015 でManagement Consoleにアクセス
```

### 3.2 Management Console (フロントエンド) 起動

```bash
cd apps/management-console
npm install
npm run dev
# → http://localhost:1420 でアクセス
```

---

## 4. Configuration (設定)

### 4.1 `styles.toml` (演出スタイル定義)
必要に応じて生成スキルのパラメータを定義します。

### 4.2 `SOUL.md` (AI人格定義)
AIの性格や話し方を定義するファイルです。オンボーディング時に設定されます。

### 4.3 Settings UI
`http://localhost:3015` → Settings ページから、以下を変更可能:
- AI Name（AIの表示名）
- Avatar（性別・スタイル）
- LLM Provider（フロントエンド / バックグラウンド）
- Background LLM（プロバイダー / モデル / APIキー）
- AI Training & Voice（LoRAアダプタ、ベースモデル、TTS音声プロバイダの設定）

---

## 5. Database (データベース)

### 5.1 スキーマ概要
- `jobs`: 全ジョブの履歴
- `karma_logs`: 学習した教訓の蓄積 (ティア制: HOT/WARM/COLD, FTS5検索対応)
- `system_settings`: LLM設定、AI名などのシステム設定
- `chat_history`: Agent Consoleのチャット履歴
- `chat_memory_summaries`: チャット記憶の蒸留サマリー
- `soul_mutation_history`: 魂の変異履歴
- `evolution_chronicle`: 進化ハッシュチェーン記録
- `agent_stats`: エージェントステータス (レベル/経験値/共鳴)
- `commune_messages` / `commune_peers` / `commune_topics`: Communeプロトコル
- `immune_rules`: 免疫ルール (フェデレーション対応)
- `ai_artifacts` / `artifact_edges`: アーティファクトストア
- `expressions`: Expression Engine (自律表現)
- `sns_metrics_history`: SNSメトリクスレコード
- `federation_peers`: フェデレーションピア [Deferred to v1.5]
- `arena_history`: スキルアリーナ対戦履歴
- `hypotheses`: 科学的夢想 (Scientific Dream) による改善仮説と実験ステータス

### 5.2 科学的自己改善ループ (Scientific Dream)
エージェントがレベル 5 以上に到達すると、`DreamState` において「科学的夢想」が有効化されます。
1.  **仮説生成**: 過去の失敗や成功（Karma）を分析し、性能向上のための仮説を LLM で生成。
2.  **実験ジョブ投入**: 生成された仮説に基づき、専用の実験タスクを `JobQueue` に投入。
3.  **反復レビュー**: `Oracle::multi_review` により、生成物の品質を LLM 間で相互批判・修正し、最高品質を担保します。

管理者は `hypotheses` テーブルを確認することで、エージェントがどのような仮説に基づいて自律進化を試みているかを追跡できます。

### 5.3 DB ファイルの場所
SQLite DB は `workspace/aiome.db` に自動作成されます。

### 5.4 Data Protection & Backups
システム保護のため、2層のバックアップ機構が動作します。
1. **Pre-Migration Guard**: 起動時（マイグレーション前）に自動で `.pre_migration.bak` スナップショットを作成し、スキーマ変更失敗時のロールバックを担保します。
2. **Automated Rolling Backups (Cron推奨)**: WAL-safe なホットスナップショットと世代管理を行うスクリプトが用意されています。詳細は [`docs/operations/BACKUP.md`](../operations/BACKUP.md) を参照してください。

---

## 6. Monitoring (監視)

### 6.1 ログ出力
```bash
RUST_LOG=info cargo run -p api-server
```

---

## 7. Troubleshooting (トラブルシューティング)

| Symptom | Cause | Solution |
|---------|-------|----------|
| `401 Unauthorized` | 認証トークン不一致 | ブラウザをリロードし再認証、`.env` の `API_SERVER_SECRET` を確認 |
| `403 Forbidden` | API キーが無効 | `.env` のキーを確認 |
| Settings画面が開かない | 401エラー | ブラウザタブをリフレッシュして再認証 |
| Ollamaモデル選択不可 | 認証切れ or Ollama未起動 | `ollama serve` を確認、ブラウザリロード |
| 日本語入力でテキストが消えない | IMEバグ (修正済み) | 最新版にアップデート |
| `Ollama connection refused` (Docker) | Docker 内部から localhost:11434 に接続試行 | `OLLAMA_BASE_URL=http://host.docker.internal:11434` を設定し、`extra_hosts` に `host.docker.internal:host-gateway` を追加 |
| XTTS 音声合成エラー | XTTS サーバー未起動 or エンドポイント不一致 | `XTTS_ENDPOINT` (デフォルト: `http://localhost:18020`) と `XTTS_SPEAKER` を `.env` で確認 |
| Shadow Worker 接続失敗 | gRPC ホストが解決不能 | `SHADOW_CLONE_GRPC_HOST` / `SHADOW_CLONE_GRPC_PORT` を確認。Docker 環境では shadow-worker サービスが必要 |

---

## 8. Production Deployment Checklist

> **Human Public Beta**: 本番 Docker では [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-6 Part B のスキップ規則を優先（G1←NT-2 / R2-1←NT-1 / OP-012・014 は過去 PASS 時）。下の localhost / `cargo run` 行は開発向け。

- [ ] `.env` に `GEMINI_API_KEY` を設定
- [ ] `.env` に `API_SERVER_SECRET` を設定
- [ ] `.env` に `VAULT_SECRET` を設定 (Key Proxy用)
- [ ] `.env` に `FEDERATION_SECRET` を設定 (Samsara Hub用 - v1.5へ延期)
- [ ] `.env` に `TIMESFM_AUTH_TOKEN` を設定し、Docker側と一致させる (Phase 3D)
- [ ] `SOUL.md` を確認・カスタマイズ
- [ ] Ollama でモデルをダウンロード (`ollama pull qwen3.5:9b`)
- [ ] `ALLOWED_ORIGINS` にフロントエンドURLを追加
- [ ] `cargo run -p api-server` でテスト起動
- [ ] ブラウザで `http://localhost:3015` にアクセスし動作確認
- [ ] 起動ログに 🚨 エラーがないことを確認
- [ ] `TTS_PROVIDER` が正しく設定されているか確認
- [ ] Docker 環境: `OLLAMA_BASE_URL` が `http://host.docker.internal:11434` に設定されているか確認
- [ ] Docker 環境: `extra_hosts` に `host.docker.internal:host-gateway` が含まれているか確認
- [ ] XTTS 利用時: `XTTS_ENDPOINT` と `XTTS_SPEAKER` が `.env` に設定されているか確認
- [ ] Shadow Worker 利用時: `SHADOW_CLONE_GRPC_HOST` / `SHADOW_CLONE_GRPC_PORT` が設定されているか確認
- [ ] `main.rs` の初期化フローでシークレット of Zeroize が実行されているかログで確認
- [ ] Polar Webhook 連携: `POLAR_API_KEY` および `POLAR_WEBHOOK_SECRET` が設定されているか確認 (P-1)
- [ ] 運用アラート: `AlertManager` による重複排除キャッシュやサーキットブレーカーとの連動確認 (A-3)
- [ ] 運用アラート: Discord Webhook アラート送信のために `.env` に `DISCORD_WEBHOOK_URL` を設定 (Phase C-4)
- [ ] Tauri デスクトップ: **通常は設定不要**（既定 InProcess）。公式同梱は `api-server` + `key-proxy` のみ（`nurture-api` 非同梱）
- [ ] Tauri ビルド検証: `python3 scripts/desktop_sidecar_manager.py --check-core`（常用）/ `--check-all`（リリース・実 nurture-api 混入禁止）
- [ ] Tauri Local escape: `--with-nurture-sidecar` + `NURTURE_MODE=local`（公式パッケージでは sidecar 無しで失敗しうる）
- [ ] Tauri Cloud: `NURTURE_MODE=cloud` + `NURTURE_CLOUD_URL`
- [ ] **Nurture S2S (OP-061)**: `NURTURE_API_URL` と `NURTURE_INTERNAL_SECRET` をセットで設定。forget / coin-charge / stripe proxy は同鍵で OXP+Bearer。URL のみ・secret なしは forget が 500（fail-closed）
- [ ] **Coin-charge DLQ (OP-060)**: 起動ログに `Coin Charge DLQ worker` が出ること。`outbox_dead_letters` の `coin_charge_failed` が滞留し続ける場合は URL/secret/Nurture 到達性を確認（不正 JSON は `coin_charge_failed_poison` に隔離）
- [ ] Tauri デスクトップ (release): `NURTURE_DRM_MASTER_KEY` が環境変数または `{data_dir}/.nurture_drm_master_key` で解決できるか確認
- [ ] api-server (release): `A2A_NODE_TOKEN` が非空で設定されているか確認（未設定/空文字時は起動失敗）。production compose は `A2A_AUTH_TOKEN` を `A2A_NODE_TOKEN` にパススルー
- [ ] nurture-api (cloud-storage): `S3_BUCKET_NAME` が設定されているか確認（未設定時は Mock へフォールバックせず起動エラー）
- [ ] CI Postgres テスト: `docker-compose.test.yml` を用いた `db_config_test` がローカルで PASS するか確認
- [ ] **PostgreSQL 本番検証 (OP-012)**: `bash scripts/verify-production-postgres.sh` が Positive / Negative / Revert すべて PASS すること（`docker-compose.production-verify.yml`、ポート `127.0.0.1:5434`）
- [ ] **Keychain CLI 検証 (OP-014)**: `bash scripts/verify-keychain-cli.sh` が PASS すること（`abyss-vault` set/get/delete + 非 whitelist 拒否）
- [ ] **Quick Start 実走 (G1)**: Human が [`docs/guides/QUICK_START_VERIFICATION.md`](QUICK_START_VERIFICATION.md) に沿って 5 分以内に完走すること
- [ ] **Stripe 本番反映 (R2-1)**: [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-1 **v1.6** — 進行は `/nt-assist` + `python3 scripts/nt_gate.py step0`。詳細はランブック（本 §8 に手順を再掲しない）

### 8.1 Release Verification Scripts（リリース前検証）

本番 compose 全体（`docker-compose.production.yml`）は secrets とカスタムイメージに依存するため、軽量スタックで DB 整合性を先に検証します。

| スクリプト | 用途 | 前提 |
|---|---|---|
| `scripts/verify-production-postgres.sh` | 3 DB（`aiome` / `nurture` / `samsara_hub`）マイグレーション + BAN 統合 | Docker、`docker-compose.production-verify.yml` |
| `scripts/verify-keychain-cli.sh` | `abyss-vault` CLI ラウンドトリップ + macOS Keychain smoke | `VAULT_MASTER_PASSWORD`（一時ディレクトリ） |
| `scripts/nt_gate.py` | NT-1 Step 0 / compose 衛生ゲート（`step0`/`hygiene`/`self-test`）。秘密は扱わない | `/nt-assist`、本番ホスト or `--skip-docker` |
| `docs/guides/QUICK_START_VERIFICATION.md` | G1: clone → 起動 → Setup Wizard → チャット | Human 実走 |

検証用 Postgres URL（任意）: `.env.example` の `PRODUCTION_VERIFY_PG_BASE` を参照。


### 9. local Embedding Server (ruri-v3) の起動
1. `tools/ruri-embed-server` に移動。
2. `python3 -m venv venv` で仮想環境作成。
3. `source venv/bin/activate` (Mac/Linux)。
4. `pip install -r requirements.txt`。
5. `python3 server.py` で起動 (デフォルト 8100 ポート)。

---
*Happy coding!*
