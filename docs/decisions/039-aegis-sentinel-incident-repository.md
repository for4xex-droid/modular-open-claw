# ADR-039: Aegis Sentinel Incident Repository and Lock Optimization

## Status
Accepted

## Context
During the end-to-end integration of the Aegis Sentinel autonomous immune system, it was discovered that recording WASM execution incidents and skill evaluation failures in `SkillArena::record_outcome` suffered from two architectural issues:
1. **Raw SQL Proliferation**: Hardcoded `INSERT INTO aegis_incidents` statements were embedded within the application logic, binding the code specifically to SQLite and violating DRY (Don't Repeat Yourself) principles.
2. **Lock Contention**: The database I/O was executed while holding a write-lock (`arena.write().await`) on the `SkillArena` internal state, severely impacting the latency and scalability of parallel agent evaluations.

## Decision
1. **Repository Pattern (IncidentRepository)**: We created `IncidentRepository` within `libs/infrastructure/src/aegis/incident_repo.rs` to abstract database operations for Aegis Sentinel. This repository standardizes incident tracking, supports both SQLite and Postgres drivers transparently, and eliminates raw SQL from the domain logic.
2. **Lock Scope Minimization**: We refactored `SkillArena::record_outcome` to gather necessary data while holding the `RwLock` write-guard, immediately drop the lock, and only then perform the asynchronous database I/O via the `IncidentRepository`.

## Consequences

### Positive
- **Database Agnosticism**: Standardized queries via SQLx abstraction enable smooth transitions between SQLite (development/edge) and PostgreSQL (production).
- **Improved Concurrency**: Dropping the `RwLock` before awaiting database operations prevents blocking parallel evaluator threads, increasing overall evaluation throughput.
- **Maintainability**: Centralizing incident persistence in `IncidentRepository` simplifies future enhancements, such as log rotation, payload truncation, or integration with external SIEM systems.

### Negative
- **Minor Overhead**: Introducing a repository trait requires slightly more boilerplate code for injection and mocking in test environments.

## Related
- Phase 8.8 Aegis Sentinel Integration (See `RIPPLE_MAP.md`)
- `libs/infrastructure/src/aegis/incident_repo.rs`
- `libs/infrastructure/src/skills/skill_arena.rs`
