# ADR-045: Closed-Loop Intelligence & Quality Gate Integration

## Status
Accepted

## Context
SoT Engine (Society of Thought) が生成した出力の品質および安全性（アライメント）を担保するため、自律的な最適化ループと厳格な品質ゲート判定を導入する必要がありました。
Autodata / ComPilot / OpenClaw 論文にインスパイアされた、予算制限付きのリトライループ（Challenger-Verifier パターン）と、平均スコアに応じたリアルタイムの人間レビューフォールバックを組み込むことで、システム全体の信頼性と耐障害性を向上させます。

## Decision
1. **構造化フィードバックモデル**: 
   - `FeedbackCategory` (Security, Correctness, Completeness, Style)
   - `IterationRecord` (スコア、フィードバック文、タイムスタンプ)
   - `OptimizationBudget` (最大再試行回数 `max_rejections`)
   を追加。

2. **Challenger-Verifier ループ**:
   - `SoTEngine` 内で Challenger と Verifier の役割を明確化。
   - 予算範囲内で自律リトライ（最大 `max_rejections`）を実行し、合格または予算尽き（`ChallengerRejected`）となるクローズドループ最適化を実装。
   - フィードバックの一次分類ヘルパー `classify_feedback` を追加。

3. **三値判定品質ゲート**:
   - `oracle` にて平均スコア（Alignment & Growth）に基づいて `ReviewDecision` を決定：
     - `>= 0.85`: `Accept` (自動承認)
     - `0.60 ~ 0.85`: `HumanReview` (人間レビュー要求)
     - `< 0.60`: `Reject` (自動却下)

4. **レピュテーション履歴の SQLite 保存**:
   - `SkillPerformance` テーブルに `optimization_history` (JSON) と `best_scores` (JSON) カラムを追加し、初期化時に自動 `ALTER TABLE` マイグレーション。
   - 非破壊的な `record_outcome_with_feedback` を実装。

5. **Dispatcher のステータス遷移**:
   - `HumanReview` 発生時、ジョブを `JobStatus::AwaitingInput` に遷移させ、`TaskEvent::AwaitingInput` / `TaskEvent::QualityGate` イベントをトリガーしてタスクループを一時停止。

## Consequences
- **信頼性**: 自動化されたクローズドループ最適化により、低スコアの出力が本番環境へ反映されるリスクが物理的にゼロになりました。
- **後方互換性**: 既存の `record_outcome` シグネチャや `ReviewDecision` への `HumanReview` 追加は非破壊的に行われ、既存のシミュレーションとシームレスに調和しています。
- **性能**: リトライ回数は `OptimizationBudget` により制限されており、無限ループによるトークン・リソース浪費が防止されます。
