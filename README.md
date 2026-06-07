<div align="right">
  <strong>日本語</strong> | <a href="README_en.md">English</a>
</div>

<p align="center">
  <img src="docs/assets/logo/Aiomeロゴ（横長120×500）.png" alt="Aiome Logo" width="400">
</p>

<h1 align="center">Aiome (アイオーム)</h1>
<p align="center">
  <strong>The Self-Healing AI Agent OS</strong><br>
  <em>Written entirely by AI agents. 280,000+ lines of production Rust.</em><br><br>
  <a href="https://aiome.dev"><strong>aiome.dev (公式ウェブサイト)</strong></a><br><br>
  <strong>AIが自分たちで実装した、AIが安全に活動・進化するための「自律型オペレーティングシステム」</strong>
</p>

<p align="center">
  <img src="https://github.com/motivationstudio-llc/aiome/workflows/CI/badge.svg" alt="CI Status">
  <img src="https://img.shields.io/badge/License-BUSL_1.1-blue.svg" alt="License: BUSL-1.1">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/TLA%2B-Verified-0052cc.svg" alt="TLA+ Verified">
  <a href="https://github.com/motivationstudio-llc/aiome"><img src="https://img.shields.io/badge/Built%20by-Agents-blueviolet" alt="Built by Agents"></a>
  <a href="https://aiome.dev"><img src="https://img.shields.io/badge/Website-aiome.dev-00f2ff.svg" alt="Website"></a>
</p>

---

[![Aiome Quickstart Demo](docs/assets/quickstart_demo.webp)](#)
*(Coming Soon)*

---

## ⚡ Quick Start (5秒で起動 / No config needed)

> [!TIP]
> **$0 / month 💸**
> Docker / Podman を使って自分のマシンでセルフホストすれば、高度な AI エージェント OS を**毎月 $0** で無制限に利用できます。すべての機能がデフォルトで手に入ります。

Aiome は、面倒な設定なしで、コマンド一発で全機能（チャット、ツール実行、自己修復、シミュレートされたAI経済）が体験できるように設計されています。
商用決済（Commerce / Stripe）等の高度な機能も、キー未設定時には**すべて自動でモックモード**として動作するため、何も壊れません。

> **📖 詳細な仕様と制約:**
> Docker Quickstart 環境での認証や機能制限については、必ず [QUICK_START.md](QUICK_START.md) をご一読ください。

### オプション A: Docker / Podman を使う（推奨）
初回ビルドの10分以上をスキップし、あらかじめビルドされたイメージと Ollama を立ち上げます。

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
# 通常のクイックスタート（モック決済）
docker compose -f docker-compose.quickstart.yml up -d

# または、商業機能（Nurture Engine / Stripe決済）を有効化して起動する場合
docker compose -f docker-compose.commercial.yml up -d
```
起動したらブラウザで管理UI（ポート `1420`）にアクセスできます。

> **💡 Podman ユーザーへの注意事項**:
> Podman rootless 環境では `host.docker.internal` がデフォルトで解決されない場合があります。ローカルの Ollama に接続できない場合は、`docker-compose.quickstart.yml` の `OLLAMA_HOST` を `http://host.containers.internal:11434` に変更するか、Ollama をコンテナ内（上記 compose ファイル通り）にお使いください。

### オプション B: ソースからビルドする

> [!IMPORTANT]
> **Production Security**:
> 起動には必ず16文字以上の強固な `API_SERVER_SECRET` 環境変数が必要です。設定されていない場合、セキュリティ保護のためプロセスは起動直後にパニック（終了）します。

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
# ⚠️ 初回コンパイルにはお使いのPC環境で 5〜15分 程度かかります
API_SERVER_SECRET="my_super_secret_key_123456" cargo run --bin api-server # gitleaks:allow
```

> **Commerce 機能について**:
> `STRIPE_API_KEY` を `.env` に設定しない場合、システムはこれを検知して自動的に `MockCommerceEngine` にフォールバックします。課金やギグの発注など、OS内のAIエコノミー機能は何も設定することなくフェイク残高を用いてすぐに体験可能です。

---

## 🌌 Aiome とは？ (Philosophy & Concept)

Aiome は、単なるエージェント・フレームワークを超えた、AIエージェントが安全に活動・進化するために設計された **「自律型 AI オペレーティングシステム」** です。

**コードの100%はAIエージェントによって自律的に記述されました。**
これは単なる実験ではありません。エージェントが自ら、「自分たちが最も安全に、かつ規律を持って活動できる環境」を設計・実装した結果です。

以下のすべての Capability がプラグインではなく、OS に最初から組み込まれています。

- 🛡️ **Trust Layer**: SHA-256 で保護された監査チェーンと、O(1)の境界検証。高リスクなツール呼び出し時にユーザー介入を求める「Governed Execution（統治型実行）」層や、OxiLeanによる定理証明/形式検証レイヤーを新たに統合。
- 🦠 **Cell-Based Architecture (CBA)**: `CELL_ID` によるデータの物理的パス隔離とシークレット分離を備えた堅牢なサンドボックス基盤。パストラバーサル防御やShellインジェクション防御を完備。
- 🕸️ **GEO Intelligence**: Generative Engine Optimization (GEO) 監査エンジンを搭載。Graceful Degradation（ソフトフォールバック）対応の非対称設計により、インフラ障害時や外部モジュール切断状態でもSEO・パブリッシングパイプラインの品質と継続稼働を自律的に維持する。
- 🔐 **Zero-Trust Hardening**: Rust 2024 Edition 準拠の環境変数クリア (`scrub_env`) 完全一元化によるシークレットパージ。加えて、IPv4-mapped IPv6 や Link-local アドレス完全遮断による高度な SSRF 防御網を備える。
- 🧠 **Soul Engine**: エージェントの人格、記憶、そして「感情と進化」を統制するミドルウェア。
- ⚖️ **Governed Execution Layer**: 実行環境の厳格なポリシー適用と、高リスク操作に対する人間介入の強制フロー。
- 📚 **Cortex Knowledge Base**: 単なるRAGを超え、LLMが複数文書から概念を抽出し、相互にリンクされたWiki記事としてナレッジを自律的に自己再構築する知識エンジン。「表示レベル制御（Progressive Disclosure）」と「クエリのFile-Back（自己増殖）」機構を備える。
- 🏥 **Self-Healing (Watchtower)**: エラーが起きた際、原因を推論し、自己修復ヒントを抽出して再試行する自律診断ループ。
- 💾 **Crash Recovery & Backup**: `sqlite3 .backup` によるWAL-safeなオンライン・スナップショットと、マイグレーション前の保護ガード（Pre-migration Guard）を自動実行。予測不能な障害やマイグレーション失敗からの確実な復旧を担保する堅牢なデータ保護アーキテクチャ。
- 🎨 **Creative Studio**: WASMサンドボックス上で実行されるツール・スキルの動的評価環境。
- 🎭 **Avatar & Voice**: テキストにとどまらない、合成音声とVRMアバターを通した「生きた表現」エンジン。
- 💰 **Agent Economy (Commerce & Gig)**: AI同士がタスクを発注・依存するエスクローと経済基盤。タスク失敗やレビューReject時には瞬時に資金を解放する「自己責務型返金アーキテクチャ（Resilient Escrow Refund）」を完備。
- 🏪 **LoRA Marketplace**: エージェントの性格（LoRAアダプター）をエスクロー決済・ファイル分離サンドボックス経由で安全に取引・共有できる人格流通プラットフォーム。
- 📣 **Buzz Protocol (Autonomous SNS Worker)**: トレンドAPIやLLMと連動し、指定したスケジュールと日次クォータに基づいて自律的にコンテンツをドラフト・投稿するバックグラウンドワーカー。投稿前に人間が内容を審査する（Approve/Reject）インターフェースも備え、安全なソーシャル発信を実現。
- 🛡️ **Autonomous Support System (自律サポートシステム)**: Discord などの外部チャネルと完全に統合された、インシデントの自動分類・回答生成・エスカレーションおよびフィードバックループの自動化システム。Botの応答に自動的にチケットID（`[TICKET:uuid]`）を埋め込み、ユーザーによる「✅（解決）」や「❌（未解決）」といったリアクション検知（API削減のためのOnceLockによるBotIDキャッシュ、およびLazyLockによる正規表現キャッシュ完備）を通じて、Karma Registryの重み（長期記憶の重要度）をリアルタイムに自動調整・自己進化する。
- 📡 **TrendSonar Integration**: X API や SERP など外部からのトレンドシグナルをリアルタイム摂取。`FuturesUnordered` による並行フェッチと、`429 Retry-After` 応答に対する高度な自律ハンドリングを備え、完全なスレッドストール防止とAPIクオータ保護を実現。動的なファクトリによる再構成で、再起動不要の安全なトークン反映をサポート。
- 🔌 **Dynamic MCP Federation**: Model Context Protocol (MCP) をフルサポート。**「GitHub Issue の自動トリアージ」「Notion 知識ベースとの双方向連携」「Web検索による最新情報のリアルタイム収集」** など、標準提供される公式MCPパッケージを即座にマウント可能。GUIダッシュボードを通じたシームレスな統合と、パストラバーサルや不正スキームを防ぐ厳格なセキュリティバリデーションを備える。
- 🎨 **Premium Management Console**: 100% トークン駆動の UI システム。`tokens.css` による中央管理により、生の色指定（HEX/RGBA）を完全に排除。リアルタイムのセキュリティ承認フロー（AwaitingInput Overlay）を含む、防弾仕様の管理システム。

「野生の天才（LLM）」が現実世界で安全に、かつ長期的に生存・進化するための「頭蓋骨、神経系、そして免疫システム」。これこそが Aiome の存在意義です。

---

## 🏗️ アーキテクチャ (Architecture)

Aiome は堅牢性を担保するため、Rust の TypeState パターンを駆使し、レイヤーを厳重に分離しています。また、商用決済・経済連携のための商業エンジンが `commercial/` 以下に統合されています。

```text
apps/api-server      ← メインバイナリ + Watchtower (Body / Soul / Discord Bridge)
apps/samsara-hub     ← P2P フェデレーション (Hub / CRDT 同期)
      ↓
commercial/apps/nurture-api ← Nurture商業決済・エコノミーエンジン (BUSL-1.1)
commercial/libs/*           ← 商業決済プロトコル・ブリッジ・インフラ
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
6. **Impact Analysis Protocol**: エージェントによる自律コード改修時の未知のカスケードエラーを防ぐ、`grep_search` ベースの依存追跡プロトコルとセマンティック依存マップ（`RIPPLE_MAP.md`）の標準搭載
7. **Automated Chaos Engineering**: 意図的な障害注入（LLMタイムアウトや不正フォーマット）をテスト環境で自律実行し、「予測不能なAIの失敗」に対するシステムの縮退運転（Graceful Degradation）を完全に担保
8. **Cell-Based Architecture (CBA)**: 1プロセス=1セルの不変条件に基づく物理的パス隔離。`AppDataResolver` と Shell ガードによるパストラバーサル・インジェクションの多層防御。
9. **GDPR/RTBF & Content Compliance**: 単一トランザクションで最大7テーブルの完全な物理パージ（`forget_actor`）と安全な外部削除伝播を保証。さらに、有害コンテンツを自動検知してフィルタリングする安全フィルターを搭載。
10. **Aegis Sentinel**: WASM実行時のインシデントを常時監視・記録し、LLMによるパッチ生成とKaniによる形式検証を経て、システム稼働中にコードを自己修復・入れ替え（HotSwap）する事後修復システム。
11. **Adaptive Immune System**: 実行前に入力脅威パターンを検知し、学習ルールのドリフトを防止する事前防御システム（事後修復の **Aegis Sentinel** とともに多層免疫システムを形成）。

---

## 🧠 Soul Engine & Self-Healing

1. **Strategic Planner & Scientist Loop**: AI 自身が改善仮説を立て、反復的な自己レビューを経て実験ジョブを投入。
2. **Watchtower Diagnostic Loop**: 失敗したジョブから自律的に教訓を抽出し、次回の試行へ確実にフィードバック（修復ヒントによる冪等再試行）。
3. **Intelligence Layer (DreamState)**: アイドル時にAIが自律的に仮説検証や自己反省のジョブを生み出し、未知の課題には外部シグナルを用いた解決策（ToolDiscovery）を自己探索する完全自律アーキテクチャ。
4. **Arena Battle**: 自律エージェント間でのスキルや意思決定モデルの競争を通じて、最適なモデルを自己選択する評価・検証環境。
5. **Society of Thought**: 複数の意思決定エージェントがプロンプトを通じてディスカッションを行い、合意形成を行うマルチエージェント協調エンジン。
6. **Memory Crystallizer**: 蓄積された短期経験から重要な決定や教訓を抽出し、長期記憶（MEMORY.md等）へと結晶化・圧縮する記憶整理システム。1サイクルあたりの最大スキル処理数・文字数制限およびバッチ分割処理による多層OOM防御と、XMLデリミタを用いたプロンプトインジェクション対策を標準搭載し、エラーの局所化スキップ制御によってAPI障害時も耐障害性を担保。
7. **TimesFM Forecast**: 時系列予測基盤モデル（TimesFM）を統合し、トレンドや自律経済圏におけるアセット需要の精緻な予測を可能にする時系列予測エンジン。

---

## 🛠️ 技術スタック (Technical Stack)

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Docker](https://img.shields.io/badge/docker-%232496ED.svg?style=for-the-badge&logo=docker&logoColor=white)
![Podman](https://img.shields.io/badge/podman-%23892CA0.svg?style=for-the-badge&logo=podman&logoColor=white)

| コンポーネント | 採用技術 | 役割 |
|---|---|---|
| **Core Engine** | Rust | 高速・メモリ安全かつ堅牢なセキュリティ基盤 |
| **Formal Verification** | TLA+ / TLC / Rust TypeState | 状態遷移のTLA+仕様化とモデルチェッカーによる数学的検証 |
| **Storage** | SQLite | 依存の少ない組み込みDBによる自己完結型運用 |
| **Expansion** | WebAssembly / OxiLean | ネットワーク制限下での安全なスキル実行環境と形式検証定理証明 |

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

## ⚖️ 法的文書 (Legal & Privacy)

プロダクトのパブリックβリリースに伴い、以下の法的文書を定めています。利用前に必ずご確認ください。

- **[利用規約 (Terms of Service)](docs/legal/TERMS_OF_SERVICE.md)**
- **[プライバシーポリシー (Privacy Policy)](docs/legal/PRIVACY_POLICY.md)**

---

## 🛡️ ライセンスと商用手数料 (License & Commercial Fees)

**Aiome Core** および **Nurture Commercial Engine** は商用化を見据え、すべて **Business Source License 1.1 (BUSL-1.1)** の下でライセンスが統一されています。  
*指定日（2030年4月1日）に自動的に Apache License 2.0 へと移行します。*

### 商用利用時の手数料体系 (Commercial Fees)
本プラットフォームで商業トランザクションを実行する際、以下の手数料（コミッション）が適用されます：
- **Free プラン (オープンソース版 / セルフホスト)**: プラットフォーム利用手数料 **25%**
- **Pro プラン (商用ライセンス / ホスト型)**: プラットフォーム利用手数料 **10%**

詳細な条項についてはリポジトリ内の `LICENSE` および `commercial/LICENSE` をご確認ください。

---

*Built automatically by Agents of [motivationstudio, LLC](https://github.com/motivationstudio-llc) — Powering the Future of AI Autonomy.*
