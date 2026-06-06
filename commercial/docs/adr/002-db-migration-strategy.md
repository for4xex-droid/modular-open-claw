# ADR-002: SQLite → PostgreSQL マイグレーション戦略

## ステータス
Accepted

## コンテキスト
初期開発スピードを優先するためローカルファイルベースの SQLite を使用しているが、本番環境の並行書き込み負荷（M2M経済圏）には PostgreSQL が必要となる。

## 決定
開発・小規模テストには SQLite (WALモード) を使用し、本番環境では PostgreSQL (SERIALIZABLE) へ移行する。

## 理由
1. **sqlx による抽象化** — Rust の sqlx クレートは Any ドライバーまたはフィーチャ切り替えにより、共通の API で両方の DB を操作可能。
2. **WAL モードの活用** — SQLite でも WAL モードにより単一書き込み・複数読み取りの並行性を確保可能。
3. **SERIALIZABLE 保証** — 経済プロトコルとして二重支払いを防ぐため、PostgreSQL の最強の分離レベルが必要。

## 影響
- SQL の互換性管理（Upsert 構文など）が必要。
- DB ドライバーの動的切り替えロジックを `main.rs` に実装する必要がある。
