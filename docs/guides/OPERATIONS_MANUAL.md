# Aiome Operations Manual — 実用運用ガイド
**Version:** 3.1  
**Last Updated:** 2026-05-08

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
DISCORD_TOKEN=your_discord_bot_token
NOTION_API_KEY=your_notion_integration_token
X_API_KEY=your_x_api_key
X_API_SECRET=your_x_api_secret
X_ACCESS_TOKEN=your_x_access_token
X_ACCESS_TOKEN_SECRET=your_x_access_token_secret

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
- `biome_messages` / `biome_peers` / `biome_topics`: Biomeプロトコル
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

---

## 8. Production Deployment Checklist
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
- [ ] `main.rs` の初期化フローでシークレットの Zeroize が実行されているかログで確認

### 9. local Embedding Server (ruri-v3) の起動
1. `tools/ruri-embed-server` に移動。
2. `python3 -m venv venv` で仮想環境作成。
3. `source venv/bin/activate` (Mac/Linux)。
4. `pip install -r requirements.txt`。
5. `python3 server.py` で起動 (デフォルト 8100 ポート)。

---
*Happy coding!*
