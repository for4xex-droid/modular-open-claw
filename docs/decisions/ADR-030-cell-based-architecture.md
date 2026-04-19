# ADR-030: Cell-Based Architecture (CBA) への移行と実装戦略

## Status
Accepted

## Context
Aiome フレームワークおよび Nurture プロトコルの急速な進化に伴い、単一インスタンス内での状態管理や依存関係の密結合が、将来の大規模分散デプロイメントにおけるスケーラビリティの足枷となるリスクが顕在化した。
また、将来的に P2P Hub (Samsara Hub) に接続する数十万規模のエージェント経済圏において、各エージェント（セル）間の **完全なパス隔離** と **秘密鍵分離** が必須である。

## Decision
「Cell-Based Architecture (CBA)」を採用し、すべてのデータ永続化層（SQLite, Artifacts）および認証情報（Secrets）を `CELL_ID` による論理・物理名前空間で分離する。
また、コンポーネントの状態依存性をインメモリや限定されたDI（Dependency Injection）に留め、**1プロセス = 1セル** の不変条件（Invariants）を制定する。

### The Invariants（不変条件）
1. **プロセス境界**: 1セルは1システムプロセス群（Dockerコンテナ群）として独立する。プロセス内での複数セルの混在は許可しない。
2. **パス分離**: すべてのファイルシステムI/Oは `data/${CELL_ID}/` 配下で行う。
3. **共有状態の排除**: 全ての状態は DI により供給された SQLite または プロセスローカルなメモリ領域で管理し、シングルトンを通じたクロスセル汚染を防ぐ。

### 秘密鍵マトリクスと隔離戦略

| 分類 | 鍵・シークレット名 | スコープ / 保存場所 | 管理手法 |
|---|---|---|---|
| **セルごと固有** | `JWT_PRIVATE_KEY_B64` | セル単位 (Key-Proxy) | セル固有生成、docker-compose で注入 |
| | `FEDERATION_SECRET` | セル単位 (Samsara Hub) | 同上 |
| | `VAULT_SECRET` | セル単位 (Vault) | 同上 |
| | `NURTURE_INTERNAL_SECRET` | セル単位 (S2S: API ↔ Nurture) | 同上 |
| | `STRIPE_WEBHOOK_SECRET` | セル単位 (Nurture) | 各セルで Stripe Webhook を登録し、固有の Secret 発行 |
| **共有インフラ** | `STRIPE_SECRET_KEY` | プラットフォーム全体 | Nurture コンテナ内で環境変数注入 (or Vault) |
| | `TREMENDOUS_API_KEY` | プラットフォーム全体 | 同上 |
| | `OAUTH_CLIENT_ID / SECRET` | プラットフォーム・アイデンティティ | 同上 |

### Stage 段階的移行計画
- **Stage 0**: CBA 形式化。`CELL_ID` の導入、既存環境の後方互換性を保ちながらの docker-compose 分離。`AppDataResolver` の改修。
- **Stage 1**: セルオーケストレータ実装。秘密鍵自動生成、DB自動マイグレーション機構、フロントエンド API 動的バインディング対応。（未着手）
- **Stage 2**: GPU VRAM エコシステム対応。共有推論プール（Ollama / Ruri）との安全な通信および、`VramArbiter` の外部化。（未着手）

## Consequences
- **Positive**: 完全なコンテナ分離により、未知の脆弱性（例: SSRF）が発生しても他セルへのデータ漏洩を防止できる。
- **Positive**: 水平スケールアウトが容易となり、単体サーバでの運用リソースの効率化と将来的なオーケストレータ（Kubernetes 等）移行が容易になる。
- **Negative**: Nurture コンポーネントおよび各種 API がそれぞれ個別に SQLite インスタンスをポーリングするため、プロセス間オーバーヘッドが増加する可能性がある。
- **Negative**: 自動マイグレーションツール (`aiome-migrate`) などの開発・運用保守コストが増加する。
