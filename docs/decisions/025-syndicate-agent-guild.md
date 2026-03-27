# ADR-025: Syndicate (Agent Guild) Infrastructure

## Status
Proposed

## Context
エージェントが単体で動作するだけでなく、チーム（ギルド）として協力する必要性が高まっています。
複数のエージェントが特定の目的（例：大規模開発、24時間監視、相互レビュー）のためにリソースやコンテキストを共有する仕組みが必要です。

## Decision
1. **SyndicateOps の導入**: `aiome-contracts` にギルド管理用の抽象インターフェースを定義する。
2. **UUID 基盤**: ギルド ID は将来のフェデレーション（ノード間同期）を見据え、UUID (v4) を採用する。
3. **認可モデル**: ギルドの操作（削除、メンバー管理）は `owner_id` を持つエージェントのみに制限する。
4. **永続化**: `infrastructure` に `SqliteSyndicateStore` を実装し、既存の SQLite データベース内に `guilds` および `guild_members` テーブルを追加する。
5. **トランザクション戦略**: SQLite のロック競合を避けるため、可能な限り個別クエリによる即時コミット方式を採用する。

## Consequences
- エージェント間の協力関係をシステムレベルで管理可能になる。
- 将来的にギルド単位でのリソース制限や、共有ナレッジベース（RAG）の基盤となる。
- `app_state.rs` および `main.rs` に新たな Component 依存が発生する。
