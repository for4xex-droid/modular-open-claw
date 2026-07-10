# ADR-053: Federation Unstubbing Acceptance

## Status

Accepted (2026-07-09)

## Context

Phase 3.5 of `implementation_plan.md` originally planned to add stub warnings to five Federation methods in `job_queue/federation.rs` and defer full implementation to Phase 4.

Subsequent work (CHANGELOG: "Federation Unstubbing") implemented `FederationOps` export/import, peer sync, and related SQL paths before the stub-documentation ADR was written. OP-069 tracked the missing ADR as the remaining deliverable.

## Decision

1. **The current `UniversalJobQueue` Federation implementation is authoritative.** Methods such as `do_export_federated_data` and `do_import_federated_data` are production code, not stubs.
2. **No re-stubbing or parallel Federation layer** will be introduced for documentation parity with the old plan.
3. **Phase 4 Reputation work** may build on this implementation without re-opening Federation transport semantics unless a new ADR explicitly supersedes this one.

## Consequences

- OP-069 ADR item is satisfied without code changes.
- Future Federation changes require normal impact analysis (`federation.rs`, `FederationOps` trait, peer sync settings).
- Stale references to "Federation stub" in older planning docs should be read as historical; this ADR is the acceptance record.
