# ADR 035: A2UI taskApproval Component Delegation Design

## Status
Accepted

## Context
A2UI Generative UI Phase 1 の実装において、LLM がユーザーに自律的なタスクの承認を求める `taskApproval` コンポーネントの設計が議論された。
システム内にはすでに `TaskApprovalOverlay.tsx` を用いた堅牢な SSE ベースの承認フローが存在しており、A2UI 側で `taskApproval` コンポーネントを独立して実装すると、「どちらのUIを使うべきか」「状態管理が2重化する」という重大なアーキテクチャ上の衝突（二重承認システム問題）が発生することが Perfect Plan スキャンで判明した。

## Decision
**Option C: Delegation（委譲）パターンを採用する。**

1. A2UI の `taskApproval` コンポーネントは**「情報の表示」に特化**し、独自の承認ロジックを持たない。
2. 承認アクションのボタン(`button` component with `action: "approve_job:..."`) が押下された場合はバックエンド（`/api/v1/a2ui/action`）へリクエストを送信する。
3. バックエンドはこれを解釈し、既存のシステムと同様に `task_awaiting_input` のフローを駆動させ、必要に応じて既存の `TaskApprovalOverlay` に引き継がせるか、直接 Job を承認状態に移行させる。

## Consequences
### Positive
- `JobQueue` および既存の承認フロー（`TaskApprovalOverlay`）との状態二重管理を回避できる。
- A2UI を「純粋なプレゼンテーション層」として維持でき、関心の分離（Separation of Concerns）が保たれる。
- ユーザー体験上、UIの齟齬が発生しない。

### Negative
- フロントエンドの実装において、A2UIからのイベントなのか、バックエンドからのSSEイベントなのか、状態を跨ぐ際の連携を注意深くテストする必要がある。
