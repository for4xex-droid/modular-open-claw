# Lex AI Constitution (AI都市建築基準法)

本ドキュメントは、Modular OpenClaw プロジェクトにおいて AI エージェント（アクター）を「自律的な部品」として安全かつ堅牢に運用するための基本法を定義する。

---

## 第1条：物理的境界 (The Cage)

AI アクターは、システムが提供する「檻 (Jail)」の外にあるリソースに直接アクセスしてはならない。

1. **Dependency Injection**: アクターの `execute` メソッドは、引数として必ず `bastion::fs_guard::Jail` ハンドルを受け取らなければならない。
2. **Access Control**: ファイル操作は、提供された `Jail` を介して取得した `SafePath` 上でのみ許可される。
3. **Escalation**: Jail 外へのアクセス試行が検知された場合、アクターは即座に停止され、セキュリティイベントとして記録される。

---

## 第2条：通信プロトコル (The Contract)

アクター間のやり取りは、生テキストプロンプトではなく、厳格に定義された「契約（型）」に基づいて行われなければならない。

1. **Type Safety**: 全てのリクエストとレスポンスは Rust の構造体として定義され、`serde` によるバリデーションを通過しなければならない。
2. **No Hallucination**: AI のハルシネーション（嘘の形式）は、型変換（Deserialization）の段階で物理的に遮断する。
3. **Traceability**: 全てのメッセージは `trace_id` を保持し、命令の発生源と伝搬経路を完全に追跡可能にする。

---

## 第3条：統治構造 (The Governance)

個々のアクターは失敗する可能性があることを前提とし、システム全体でその失敗を制御・修復しなければならない。

1. **Supervision Tree**: アクターは `Supervisor` の監視下で実行される。アクターのパニックは Supervisor が捕捉する。
2. **Restart Policy**: クラッシュ時、Supervisor は定義されたポリシー（即時再起動、待機、エスカレーション）に基づき、クリーンな環境でアクターを再生成する。
3. **Self-Healing**: 重大なセキュリティ違反やリソース枯渇が検知された場合、Supervisor は都市全体（システム）への影響を防ぐためにアクターを隔離・終了させる。

---

## 付則：実装方針

- **Core First**: 本法典のインターフェースは `libs/core` に定義し、具体的なインフラ実装（`libs/infrastructure`）と分離する。
- **Strict Mode**: 本番環境においては `ENFORCE_GUARDRAIL=true` を常時適用し、法規違反を一切許容しない。
