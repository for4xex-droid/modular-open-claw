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

## クイックスタート

```bash
# 前提条件: Ollama + ComfyUI がローカルで動作していること

# ビルド
cargo build -p shorts-factory

# 実行
cargo run -p shorts-factory

# テスト
cargo test --workspace
```

## ライセンス

MIT
