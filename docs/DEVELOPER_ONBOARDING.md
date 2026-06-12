# Aiome 開発者オンボーディング & アーキテクチャガイド

本ドキュメントは、Aiome の codebase へのオンボーディング手順、物理構造、および Samsara/Commune/Nurture 連携を含むシステム間データフローを解説する開発者向けガイドです。

---

## 1. 開発環境セットアップ

Aiome の開発には、以下のツールチェーンが必要です。

* **Rust**: `rustup` にて最新の Stable ツールチェーンをインストールしてください。
* **Node.js**: `v18` 以上の LTS バージョンを推奨します。
* **wasm-pack**: WASM スキルのコンパイルに必要です。以下のコマンドでインストールします。
  ```bash
  cargo install wasm-pack
  ```

### クイックビルド手順

1. **Rust ワークスペースのチェック**:
   ```bash
   cargo check --workspace
   ```
2. **単体テストの実行**:
   ```bash
   cargo test --workspace
   ```
3. **ディープスキャンの実行**:
   ```bash
   bash scripts/deep-scan.sh
   ```

---

## 2. ワークスペース構造 (16 クレート解説)

Aiome のリポジトリはモノリポ形式で構成されており、役割に応じてパッケージ化されています。

```
aiome/
├── libs/
│   ├── aiome-contracts/      # トレイト、エラー定義、基本契約コントラクト
│   ├── shared/               # ログ、暗号、メモリ、プロセス堅牢化ユーティリティ
│   ├── core/                 # 認知コア (LoRA, 人格エンジン, 思考社会)
│   ├── infrastructure/       # LLMゲートウェイ、ジョブキュー、DB層 (Postgres/Sqlite)
│   ├── napi-bridge/          # Node.js 向け Rust ネイティブバインディング
│   └── wasm-skills/          # WASMで隔離実行されるツール群 (fs_reader, fs_writer, terminal_exec)
└── apps/
    ├── api-server/           # メイン API サーバー (WebSocket & REST API)
    ├── samsara-hub/          # Samsara同期用ハブ (シミュレータデータ同期)
    └── key-proxy/            # エージェント署名およびSovereign鍵管理プロキシ
```

---

## 3. 人格進化 (Samsara) データフロー

WebGL バイオームシミュレータからのイベントが、どのように Rust API を経由して PostgreSQL/SQLite および LLM プロバイダーと同期するかのシーケンス図です。

```mermaid
sequenceDiagram
    autonumber
    participant Biome as WebGL Biome Simulator
    participant API as apps/api-server (Rust)
    participant Hub as apps/samsara-hub (WASM)
    participant DB as Database (PostgreSQL/SQLite)
    participant LLM as LLM Provider (Gemini/Ollama)

    Biome->>API: 1. シミュレーション状態イベント送信 (WebSocket/POST)
    activate API
    API->>Hub: 2. 状態進化のトリガー要求
    activate Hub
    Hub->>DB: 3. 現在の Soul / DNA 状態取得
    DB-->>Hub: 4. 状態返却
    Hub->>LLM: 5. 認知的進化のための推論要求
    LLM-->>Hub: 6. 進化結果 / 行動決定の返却
    Hub->>DB: 7. 更新された Soul 状態の保存 (Samsara 状態遷移)
    Hub-->>API: 8. 進化処理完了の通知
    deactivate Hub
    API-->>Biome: 9. クライアント側 Biome への反映指示 (WebSocket 送信)
    deactivate API
```

---

## 4. P2P Commune 対話同期プロトコル

複数エージェント間での対話同期メカニズムとハブ経由のメッセージ伝播図です。

```mermaid
graph TD
    subgraph Agent A
        AA[Agent A Core] <-->|Local Bus| AP[Commune Broker]
    end
    subgraph Agent B
        BA[Agent B Core] <-->|Local Bus| BP[Commune Broker]
    end
    subgraph Commune Hub
        Hub[apps/samsara-hub]
    end

    AP <-->|P2P WebRTC / WS| BP
    AP -->|Message Sync (CRDT)| Hub
    BP -->|Message Sync (CRDT)| Hub
    Hub -->|Central Validation| DB[(PostgreSQL)]
```

---

## 5. 【Nurture 連携】S2S 認証・経済 remittance 連携

### 5.1. 共通 JWT 認証 & SSRF 防止設計
Aiome (OS人格側) と Project-Nurture (経済・決済API側) は共通の署名鍵（EdDSA `JWT_PRIVATE_KEY_B64`）による JWT 検証スキームを共有し、SSRF対策の `COMMUNE_HUB_WHITELIST` で許可されたホストとのみ通信を行います。

```mermaid
sequenceDiagram
    autonumber
    participant Aiome as Aiome API Server
    participant Nurture as Project-Nurture API
    participant Hub as Central Validator (EdDSA Verified)

    Aiome->>Aiome: EdDSA 秘密鍵で署名した JWT 作成
    Aiome->>Nurture: HTTP リクエスト送信 (Auth Bearer JWT)
    activate Nurture
    Nurture->>Nurture: EdDSA 公開鍵で JWT 署名検証
    Nurture->>Nurture: ホスト名が WHITELIST 内であるか検証 (SSRF 対策)
    Nurture-->>Aiome: 認証成功・データ返却
    deactivate Nurture
```

### 5.2. OxiLean 証明書による S2S 保護
Nurture API の `require_oxp_certificate` ミドルウェアが、リクエストヘッダー `x-oxilean-proof-certificate` に含まれる `OxiLeanProofCertificate` をパースし、秘密鍵検証、OXPスコア 900以上、タイムスタンプ鮮度（300秒以内）のチェックを行うセキュリティ検証フローです。

```mermaid
sequenceDiagram
    autonumber
    participant Client as Aiome Client (Rust)
    participant Middleware as require_oxp_certificate (Nurture Middleware)
    participant Handler as Nurture internal route (/internal/deduct)

    Client->>Middleware: x-oxilean-proof-certificate を添付して送信
    activate Middleware
    Note over Middleware: 1. 証明書の署名を Nurture 秘密鍵で検証
    Note over Middleware: 2. OXPスコア >= 900 であるかを検証 (高信頼性アサーション)
    Note over Middleware: 3. Timestamp 鮮度検証 (現在時刻 - 送信時刻 < 300秒)
    alt 検証成功
        Middleware->>Handler: 認可を通してリクエストをフォワード
        activate Handler
        Handler-->>Client: 200 OK (Remittance 完了)
        deactivate Handler
    else 検証失敗
        Middleware-->>Client: 403 Forbidden (Invalid Proof Certificate)
    end
    deactivate Middleware
```

### 5.3. 経済 remittance プロトコル (SettlementProtocol)
ユーザーがエージェントを稼働させた際に発生する「推論コスト」が、Aiome から Nurture API の `/internal/deduct` へと流れる決済プロトコルのフローです。

```mermaid
sequenceDiagram
    autonumber
    participant User as User Wallet / Agent
    participant Aiome as Aiome Core Engine
    participant Nurture as Nurture Economy (/internal/deduct)
    participant Ledger as Transaction Ledger (Stripe/Postgres)

    User->>Aiome: エージェント自律活動のトリガー / タスク指示
    activate Aiome
    Note over Aiome: 推論に必要なトークン・リソースコストの計算
    Aiome->>Nurture: POST /internal/deduct (JWT + OXP 証明書)
    activate Nurture
    Note over Nurture: OxiLean 証明書および JWT 認可検証
    Nurture->>Ledger: トランザクション記帳 (残高控除、エスクロー割当)
    Ledger-->>Nurture: 記帳完了 (Transaction ID)
    Nurture-->>Aiome: 決済成功 (Deduct Approved)
    deactivate Nurture
    Aiome->>User: エージェント処理結果の返却
    deactivate Aiome
```
