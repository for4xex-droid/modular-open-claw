<div align="right">
  <strong>日本語</strong> | <a href="README_en.md">English</a>
</div>

<p align="center">
  <img src="docs/assets/logo.png" alt="Aiome Logo" width="300">
</p>

<h1 align="center">Aiome (アイオーム)</h1>
<p align="center">
  <strong>The Self-Healing AI Agent OS</strong><br>
  <em>Written entirely by AI agents. 75,000+ lines of production Rust.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/License-BUSL_1.1-blue.svg" alt="License: BUSL-1.1">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/TLA%2B-Verified-0052cc.svg" alt="TLA+ Verified">
  <a href="https://github.com/motivationstudio-llc/aiome"><img src="https://img.shields.io/badge/Built%20by-Agents-blueviolet" alt="Built by Agents"></a>
</p>

---

![Aiome Quickstart Demo](docs/assets/quickstart_demo.webp)

---

## ⚡ Quick Start (5秒で起動 / No config needed)

Aiome は、面倒な設定なしで、コマンド一発で全機能（チャット、ツール実行、自己修復、シミュレートされたAI経済）が体験できるように設計されています。
商用決済（Commerce / Stripe）等の高度な機能も、キー未設定時には**すべて自動でモックモード**として動作するため、何も壊れません。

### オプション A: Docker を使う（推奨）
初回ビルドの10分以上をスキップし、あらかじめビルドされたイメージと Ollama を立ち上げます。

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
docker compose -f docker-compose.quickstart.yml up -d
```
起動したらブラウザで管理UI（ポート `1420`）にアクセスできます。

### オプション B: ソースからビルドする

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
# ⚠️ 初回コンパイルにはお使いのPC環境で 5〜15分 程度かかります
cargo run --bin api-server
```

> **Commerce 機能について**:
> `STRIPE_API_KEY` を `.env` に設定しない場合、システムはこれを検知して自動的に `MockCommerceEngine` にフォールバックします。課金やギグの発注など、OS内のAIエコノミー機能は何も設定することなくフェイク残高を用いてすぐに体験可能です。

---

## 🌌 Aiome とは？ (Philosophy & Concept)

Aiome は、単なるエージェント・フレームワークを超えた、AIエージェントが安全に活動・進化しために設計された **「自律型 AI オペレーティングシステム」** です。

**コードの100%はAIエージェントによって自律的に記述されました。**
これは単なる実験ではありません。エージェントが自ら、「自分たちが最も安全に、かつ規律を持って活動できる環境」を設計・実装した結果です。

以下のすべての Capability がプラグインではなく、OS に最初から組み込まれています。

- 🛡️ **Trust Layer**: SHA-256 で保護された監査チェーンと、O(1)の境界検証。モデルベーステストにより安全性が保証されています。
- 🧠 **Soul Engine**: エージェントの人格、記憶、そして「感情と進化」を統制するミドルウェア。
- 📚 **Cortex Knowledge Base**: 単なるRAGを超え、LLMが複数文書から概念を抽出し、相互にリンクされたWiki記事としてナレッジを自律的に自己再構築する知識エンジン。「表示レベル制御（Progressive Disclosure）」と「クエリのFile-Back（自己増殖）」機構を備える。
- 🏥 **Self-Healing (Watchtower)**: エラーが起きた際、原因を推論し、自己修復ヒントを抽出して再試行する自律診断ループ。
- 🎨 **Creative Studio**: WASMサンドボックス上で実行されるツール・スキルの動的評価環境。
- 🎭 **Avatar & Voice**: テキストにとどまらない、合成音声とVRMアバターを通した「生きた表現」エンジン。
- 💰 **Agent Economy (Commerce & Gig)**: AI同士がタスクを発注・依存するエスクローと経済基盤。
- 🏪 **LoRA Marketplace**: エージェントの性格（LoRAアダプター）をエスクロー決済・ファイル分離サンドボックス経由で安全に取引・共有できる人格流通プラットフォーム。
- 🎨 **Premium Management Console**: 100% トークン駆動の UI システム。`tokens.css` による中央管理により、生の色指定（HEX/RGBA）を完全に排除。

「野生の天才（LLM）」が現実世界で安全に、かつ長期的に生存・進化するための「頭蓋骨、神経系、そして免疫システム」。これこそが Aiome の存在意義です。

---

## 🏗️ アーキテクチャ (Architecture)

Aiome は堅牢性を担保するため、Rust の TypeState パターンを駆使し、レイヤーを厳重に分離しています。

```text
apps/api-server      ← メインバイナリ (The Body / Management Engine)
apps/watchtower      ← 外部チャネル連携 (The Soul / Discord & Telegram Bridge)
      ↓
libs/core            ← ドメインロジック (Open)
      ↓
libs/infrastructure  ← I/O実装 (SQLite / Ollama等 / Open)
      ↓
libs/soul            ← 魂のエンジン (Agents' L1-L3 Soul Engine / Open)
      ↓
libs/aiome-commerce  ← AI経済エンジン（Mock / Stripe）
```

---

## 🛡️ Trust Layer（防衛特化）

直接シェルをLLMに渡すことは、無限ループやAPIキー漏洩のリスクを孕む「脆い自由」です。Aiome は:
1. 追加のツール(Skill)はWASM空間でサンドボックス化
2. プロンプトインジェクションに対するローカル検知レイヤー（Guardrails）
3. SQLite上の暗号学的ハッシュチェーン (Karma) を使い、「自分が過去に何のタスクに失敗したか」を改ざん不可能な形で記録
4. gVisor コンテナ隔離
5. **GlassWorm Shield**: 不可視Unicode文字列を利用したステルス攻撃やLLMポイズニングを防ぐ超高速サニタイザーの全周配備
6. **Precomputed Relational Intelligence**: エージェントによる自律コード改修時の未知のカスケードエラーやパスエイリアス乖離を完全に防ぐ、超高速な静的 AST 物理依存スキャナと影響範囲クエリエマージェンシー機構の標準搭載
7. **Automated Chaos Engineering**: 意図的な障害注入（LLMタイムアウトや不正フォーマット）をテスト環境で自律実行し、「予測不能なAIの失敗」に対するシステムの縮退運転（Graceful Degradation）を完全に担保

---

## 🧠 Soul Engine & Self-Healing

1. **Strategic Planner & Scientist Loop**: AI 自身が改善仮説を立て、反復的な自己レビューを経て実験ジョブを投入。
2. **Watchtower Diagnostic Loop**: 失敗したジョブから自律的に教訓を抽出し、次回の試行へ確実にフィードバック（修復ヒントによる冪等再試行）。
3. **Intelligence Layer (DreamState)**: アイドル時にAIが自律的に仮説検証や自己反省のジョブを生み出し、未知の課題には外部シグナルを用いた解決策（ToolDiscovery）を自己探索する完全自律アーキテクチャ。

---

## 🛠️ 技術スタック (Technical Stack)

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Docker](https://img.shields.io/badge/docker-%232496ED.svg?style=for-the-badge&logo=docker&logoColor=white)

| コンポーネント | 採用技術 | 役割 |
|---|---|---|
| **Core Engine** | Rust | 高速・メモリ安全かつ堅牢なセキュリティ基盤 |
| **Formal Verification** | TLA+ / TLC / Rust TypeState | 状態遷移のTLA+仕様化とモデルチェッカーによる数学的検証 |
| **Storage** | SQLite | 依存の少ない組み込みDBによる自己完結型運用 |
| **Expansion** | WebAssembly (WASM) | ネットワーク制限下での安全なスキル実行環境 |

---

## 📚 ドキュメント (Documentation)

- **[AI憲法 (Architecture Law)](docs/architecture/ARCHITECTURE_LAW.md)**: 知的誠実性と安全性を担保する基本原則。
- **[運用マニュアル (Operations Manual)](docs/guides/OPERATIONS_MANUAL.md)**: 詳細な環境構築と運用手順。
- **[セキュリティ設計 (Security Design)](docs/architecture/SECURITY_DESIGN.md)**: 多層防御の詳細。

---

## 🤝 コントリビュート (Contributing)

- **[貢献ガイド (CONTRIBUTING.md)](CONTRIBUTING.md)**: 「Built by Agents」のプロジェクトに人間が貢献するためのルール。
- **[脆弱性の報告 (SECURITY.md)](SECURITY.md)**: セキュリティインシデントの連絡先。

---

## 🛡️ ライセンス (License)

**Aiome Core** は商用化を見据え、**Business Source License 1.1 (BUSL-1.1)** で提供されています。  
*指定日（2030年）に自動的に Apache License 2.0 へと移行します。*

機能の大部分は無償で研究・非商用目的にご利用いただけますが、一定の商用利用制限が存在します。詳細な条項についてはリポジトリ内の `LICENSE` ファイルを必ずご確認ください。

---

*Built automatically by Agents of [motivationstudio, LLC](https://github.com/motivationstudio-llc) — Powering the Future of AI Autonomy.*
