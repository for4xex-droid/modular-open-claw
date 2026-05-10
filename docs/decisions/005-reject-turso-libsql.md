# ADR 005: Rejection of Turso and libSQL Migration

## Status
**Rejected**

## Date
2026-05-11

## Context
Recent industry trends and announcements (e.g., Turso's "Unlimited Active Databases for Everybody" and the introduction of MVCC for SQLite via `libSQL`) sparked an evaluation of whether Aiome should migrate from its current `sqlx::sqlite` stack to a `libSQL` / Turso-backed infrastructure.

Aiome currently relies on a strict **Sovereign Verifier Architecture**, utilizing `CELL_ID` to isolate physical SQLite databases (`aiome.db`) per agent/tenant in a local-first environment (`workspace/<CELL_ID>/aiome.db`). While Turso's "DB-per-tenant" philosophy aligns closely with our `CELL_ID` isolation strategy, a thorough codebase and architectural verification via the `/perfect-plan` workflow revealed critical structural incompatibilities and severe risks.

## Decision
We formally **reject** the migration to `libSQL` and the adoption of the Turso Cloud ecosystem. Aiome will continue to use standard `sqlx::sqlite` coupled with our proprietary `Samsara Hub` CRDT-based synchronization.

### Rationales (Devil's Advocate Analysis)

1. **SaaS vs Local-First Conflict (Premise Mismatch)**
   Turso's infrastructure is optimized for Cloud SaaS environments hosting millions of localized databases centrally. Conversely, Aiome is a true Local-First P2P node. Adopting a centralized cloud database architecture directly violates the Sovereign Verifier principle, which mandates that the user retains absolute physical ownership of their node and data.

2. **Reinventing the Wheel (Infrastructure Collision)**
   Aiome already possesses a robust P2P synchronization and federation layer (`Samsara Hub`). Turso offers "Embedded Replicas" designed to sync edge SQLite files with a central cloud. Adopting this would force us to deprecate `Samsara Hub` in favor of a proprietary external mechanism, leading to massive architectural duplication and vendor lock-in.

3. **Macro Incompatibility and Experimental Risks (Worst Case Scenario)**
   Aiome heavily utilizes `sqlx::query!` for compile-time verified SQL queries. Mainstream `sqlx` v0.8 does not provide seamless, drop-in support for `libSQL`. A migration would necessitate rewriting thousands of lines of data access code. Furthermore, the core selling point of `libSQL`—its Multi-Version Concurrency Control (MVCC) to bypass the SQLite single-writer bottleneck—is currently classified as an **experimental feature**. Relying on an experimental feature for Aiome's core Karma Ledger and Economic transactions poses an unacceptable risk of data corruption or consistency loss.

## Consequences
- **Positive**:
  - The stability and compile-time safety of `sqlx` are preserved.
  - The `Samsara Hub` remains the single source of truth for P2P state synchronization.
  - Development resources are saved from a high-risk, low-yield "reinvent the wheel" refactoring effort.
- **Negative / Trade-offs**:
  - We must continue to manage the SQLite single-writer locking constraints natively (e.g., configuring optimal `PRAGMA busy_timeout` and `PRAGMA journal_mode=WAL` which we already do).

## Future Considerations
If `libSQL` becomes completely transparent to standard `sqlx::sqlite` drivers, and its MVCC feature attains production-ready stability without relying on a centralized cloud service, this decision may be revisited strictly for performance optimizations on the local node. Until then, the stack remains locked to standard SQLite.
