# ADR-032: Self-Organizing Society of Thought (Dochkina 2026)

- **Status**: Accepted
- **Date**: 2026-04-07
- **Deciders**: Aiome Agent (autonomous)

## Context

Dochkina (2026, arXiv:2603.28990) は 25,000 タスクの大規模実験で、マルチエージェント LLM システムにおける **Endogeneity Paradox** を発見した:
- 固定順序 + 自律ロール選択 (Sequential) が、中央集権 Coordinator (+14%) とフル自律 Shared (+44%) の両方を凌駕する
- 固定ロールの事前割当は **アンチパターン** であり、ロールはタスク文脈から創発すべき
- 有能なモデルは自発的辞退 (Voluntary Self-Abstention) により自然にコストを最適化する
- モデル能力閾値以下では自律性が品質を低下させ、固定構造の方が有効

既存の Aiome SoT Engine は Explorer → Critic の 2 固定ロールで運用されており、この知見を活用する余地があった。

## Decision

SoTEngine を **Sequential マルチパス熟議エンジン** に拡張する。

1. **`CoordinationProtocol` enum** (Sequential / Coordinator / Broadcast) を `aiome-core-contracts` に新設
2. **Sequential Protocol**: 各 Thinker が前任者の完成済み出力を全て見て自律的にロールを発明
3. **Voluntary Self-Abstention**: `[ABSTAIN]` マーカーに基づく自発的辞退検知
4. **Capability-Aware Fallback**: `auto_protocol` フラグによるモデル能力閾値に基づく自動切替
5. **Critic は残す**: Sequential マルチパスの**後**に品質ゲートとして機能（ロールではなく構造化検証）

## Consequences

- **Positive**: LLM の自律的ロール発明により、固定ロールでは到達できない多角的な熟議が可能になる
- **Positive**: Abstention によるトークンコスト自然最適化
- **Positive**: 全新規フィールドは `#[serde(default)]` で後方互換性を完全保持
- **Negative**: Sequential の O(N) レイテンシ（`num_thinkers` のデフォルト 3、最大 8 で緩和）
- **Risk**: Abstention 誤検知（SoTEvent で可視化し、管理コンソールで監視可能）

## References

- Dochkina, V. (2026). "Drop the Hierarchy and Roles: How Self-Organizing LLM Agents Outperform Designed Structures." arXiv:2603.28990.
