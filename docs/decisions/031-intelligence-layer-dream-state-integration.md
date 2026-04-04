# 031: Intelligence Layer & DreamState Integration

## Status
Accepted

## Context
Aiomeの Block Intelligence Architecture（「From Hierarchy to Intelligence」モデル）をシステム・アーキテクチャに統合するにあたり、以前の `DreamState` と自己学習ループは完全にデッドコードとなっていました。また、未知のカテゴリ・要件をもつジョブ（たとえば新しいスキルの発想や実験）がキューに積まれた場合、従来の `TaskDispatcher` は該当する Conductor（実行者）がいないため、処理を継続できずハードフェイルさせる設計となっていました。

これは、システムが「教えられたことしか実行できない（Hierarchy）」状態に留まっていることを意味し、自律的な「Intelligence Layer」としての役割を果たせていませんでした。

## Decision
以下の3つのコンポーネントを統合することで、自律的インテリジェンス層を確率・稼働させます。

1. **DreamService Backend Loopの稼働**:
   - `apps/api-server/src/internal_services/dream.rs` を新設し、アプリ起動時に10分間のインターバルで実行されるバックグラウンドループ（`spawn_all`）として実稼働させます。
   - バックグラウンドでシステムが暇な時に `DreamState` を呼び出し、内部のキューや外部のトレンドスコア（`TrendSonar`）を参照して、自発的に探索的ジョブや自己反省ジョブ（`scientific_experiment` 等）を JobQueue に積みます。

2. **TaskDispatcher の ToolDiscovery Fallback 機構**:
   - `TaskDispatcher` にて、Jobは引き当てたが適格な Conductor が1つも見つからなかった場合の挙動を変更します。
   - ただ破棄するのではなく、`ToolDiscoveryEngine::suggest_tools` にフォールバックし、必要なスキル（MCP）をAI自身に発見・提案させます。
   - エラー履歴として「このツールが必要である」という情報を自己記録することで、将来的な自動インストールフローへの布石とします。

3. **GenericLlmConductor の新設**:
   - `DreamState` が発案する抽象度が高いジョブ（`scientific_experiment`, `data_processing`）は既存の専用コンダクタで処理しきれないため、汎用的なLLMプロンプティングを行う `GenericLlmConductor` パターンを新設・登録しました。

## Consequences
### Positive
- AIがアイドル時間を利用して自律的に仮説検証タスクや自己診断タスクを生み出す（Dream）ことが可能になった。
- 未知の要件が投げ込まれても、システムがクラッシュ・放棄するのではなく、自発的に解決策を探索（ToolDiscovery Fallback）するしなやかな耐障害性を獲得した。
- 「上から下への命令」アーキテクチャから「自律知能による横の連携」への転換点が完成した。

### Negative
- `DreamService` が常にバックグラウンドでLLMをキックする（`DreamState::dream`）ため、デプロイ環境によっては無停止の推論によるコスト・リソース増につながる可能性がある。今後のチューニング（HeartbeatWakeupService等によるスリープ制御）が求められる。
