# Modular OpenClaw — ShortsFactory

YouTube Shorts / TikTok 向けの動画を**全自動で量産**する、Rust ネイティブの自律型動画工場。

## アーキテクチャ

```
apps/shorts-factory  ← メインバイナリ (The Body)
      ↓
libs/core            ← ドメインロジック (トレイト定義)
      ↓
libs/infrastructure  ← I/O実装 (ComfyUI, FFmpeg, SQLite)
      ↓
libs/shared          ← 共通型 (Config, Security, Guardrails)
```

## 技術スタック

| コンポーネント | 技術 |
|---|---|
| 言語 | Rust (メモリ安全・ネイティブ速度) |
| LLM | Qwen 2.5-Coder via Ollama |
| Agent | rig-core v0.30 |
| 画像/動画生成 | ComfyUI (localhost:8188) |
| 動画編集 | FFmpeg |
| データベース | SQLite |

## セキュリティ

3層防御 + CI 自動スキャン:

- **Guardrails**: プロンプトインジェクション検知 (ランタイム)
- **SecurityPolicy**: ツール/ネットワークのホワイトリスト (ランタイム)
- **Sentinel**: シークレットスキャン + `cargo audit` + unsafe 検出 (CI)

詳細: [docs/SECURITY_DESIGN.md](docs/SECURITY_DESIGN.md)

## 実行コンポーネント

### 1. 工場本体 (Core / Command Center)
```bash
# サーバーモードで起動 (GUI / Discord連携に必須)
cargo run -p shorts-factory -- serve
```
- Web UI: `http://localhost:3000` (コマンドセンター)
- API Port: `5000`

### 2. 監視所 (Watchtower - Discord Bot)
```bash
# 別ターミナルで起動 (.env にトークンが必要)
cargo run -p watchtower
```
- コマンド: `/status`, `/stats`, `/nuke`, `/generate`
- 詳細: [docs/WATCHTOWER_USER_GUIDE.md](docs/WATCHTOWER_USER_GUIDE.md)

### 3. エージェント育成・進化 (Evolution System)
- **Project Ani**: 交流と成功体験による AI の人格成長。
- **Unleashed Mode**: 全ての制限を解除する Platinum Edition フラグ。
- 詳細: [docs/EVOLUTION_STRATEGY.md](docs/EVOLUTION_STRATEGY.md)

### 🛠 サービス永続化 (macOS launchd)

`scripts/` にある `.plist` ファイルを使用することで、OS 起動時に自動でバックグラウンド実行させることができます。

```bash
cp scripts/*.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.aiome.core.plist
launchctl load ~/Library/LaunchAgents/com.aiome.watchtower.plist
```

## テスト

```bash
cargo test --workspace
```

## ライセンス

MIT
