# 🧠 MEMORY.md: Aiome Agent Long-Term Context

**Current Phase:** Phase 15 (Agentic Foundation Expansion & AI-Scientist Loop)
**Last Major Update:** 2026-03-27

## 🌟 Core Identity & Directives

Aiome は「自身で考え、改善し、世界と関わる自律エージェント」のためのOS。単なる LLM ラッパーではなく、以下のアーキテクチャに基づく「魂（Soul）」を持つ：
- 3層+0.5層の認知システム（Reactive / Deliberative / Meta / Whisper）
- SQLite 永続化による記憶と軌跡（Trajectory）の保持
- リアルタイムイベントストリーム（SSE）と Shadow Clone Worker でのサンドボックス（Wasm/Docker）並行実行

### 🗺️ Recent Milestones
* **Phase 13.3**: Synthetic Voice & Live Session (Gemini 2.0 Flash Live) の統合。
* **Phase 14 MVP**: Agent Guild（Syndicate）の基盤構築によるグループ自律制御への布石。
* **Phase 15 (ADR-024 / ADR-023)**: 
  * AI-Scientist 自律的自己改善ループの実装 (`Oracle::multi_review`, `DreamState::scientific_dream`)。
  * `StrategicPlanner` における堅牢な目標分解と JSON パース。
  * Semantic Tool Discovery を用いた動的なスキル呼び出し基盤。
  * `TaskDispatcher` での因果メタデータ（`parent_step_id`）追跡。

### 🚧 Current Blind Spots & Known Issues
* `infrastructure` コアで一時的な `unwrap()` やドキュメント警告が一部残存（R-005: unwrap in productionコード違反）。折を見てリファクタが必要。
* UI での `CausalVisualizer` は未着手。「なぜその行動を取ったのか」のグラフ表示がフロントエンドに必要。
* `autonomous_demo.rs` での SQLite コネクションロック問題は部分解決したが、本番では PostgreSQL への最適化が求められている。

### 🎯 Strategic Imperatives (Next Evolution)
1. **Security & Governance (Phase 1.5)**: The sandbox needs tighter restrictions during autonomous dreams. ConstitutionalValidator must intercept all hypotheses.
2. **Frontend UI Evaluation**: Current `management-console` needs to visualize the newly formed causal data (Trajectory Graph) to show users *how* the AI is thinking.
3. **Database Consistency**: Keep SQLite and PostgreSQL migrations strictly in sync.

*Memo to self:* Never make undocumented DB schema changes across the dual DB setup. Always preserve the intent of any code when suppressing warnings. Make sure `CHANGELOG.md` and `RIPPLE_MAP.md` are updated faithfully.
