# ADR 031: Deferral of JobQueue ISP Refactoring

## Status
Accepted

## Context
During the P3 Infrastructure Stabilization Phase, the Segmented Deep Scanner (`deep-scan.sh`) identified a significant discrepancy in the `JobQueue` trait implementation (`CC-1` warning). Specifically, the `UniversalJobQueue` struct implements 93 methods, whereas the core `JobQueue` trait defines only 7 methods.

Although sub-traits (e.g., `TaskRegistry`, `AuditStore`, `ChatStore`) exist to logically segregate these methods following the Interface Segregation Principle (ISP), the actual implementation blocks (`impl UniversalJobQueue`) have not been fully split into these sub-trait blocks.

Fully completing this ISP refactoring would require:
1. Moving ~86 methods into their respective `impl SubTrait for UniversalJobQueue` blocks.
2. Updating mock implementations (`MockJobQueue`) to reflect all 93 methods across their respective trait definitions.
3. Updating at least 22 files across the `infrastructure` and `aiome-core` crates that depend on these method signatures.

## Decision
We intentionally **defer the completion of the JobQueue ISP refactoring until Phase 4 (WASM Agent Async Queue Scaling)**. 

The `UniversalJobQueue` currently functions practically as a single-responsibility interface consumed primarily by the `UniversalGigEngine`. At this late stage of the Stabilization Phase (Phase 3), performing "open-heart surgery" on the core queue interface poses an unacceptable regression risk without immediate functional benefit. 

The `CC-1` warning in the deep scan will be acknowledged as a known technical debt item.

## Consequences
- **Positive**: Eliminates the risk of catastrophic regressions in the critical path of the P3 stabilization phase. Focus remains strictly on security and production readiness.
- **Negative**: Unit testing modules that rely on `MockJobQueue` will remain slightly cumbersome, as the mock does not perfectly mirror the segregated trait boundaries.
- **Negative**: The `CC-1` warning in `deep-scan.sh` will persist until Phase 4.

## Review Notes
As ruled by the Architect (2026-04-28):
> "安定化フェーズ（Stabilization）の最終盤で、22ファイル以上に影響が及ぶインターフェースのコア分割（オープンハート手術）を行うのは、アーキテクチャの自殺行為です。現在は実用上の単一責任として機能しているため、WASMエージェントの非同期キューが本格稼働する Phase 4 までインターフェースの物理分割を見送る"
