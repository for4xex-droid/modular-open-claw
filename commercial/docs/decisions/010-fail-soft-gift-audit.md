# ADR-010: Fail-Soft Strategy for Cross-Domain Atomicity in Commerce Bridge

## Status
Accepted

## Context
Project NURTURE's `NurtureCommerceBridge` orchestrates complex economic transactions (e.g., `deliver_gift`) that span across multiple independent domains:
1. **LicenseStore**: Manages digital rights and ownership.
2. **EconomyLedger**: Manages immutable financial and audit records.

These domains are implemented as separate traits (`LicenseStore` and `EconomyLedger`), each maintaining its own independent connection pool (e.g., `SqlitePool`). Because of this architectural decoupling, we cannot easily wrap operations that span both domains inside a single database transaction (`Unit of Work` pattern) without significantly refactoring the trait definitions and violating bounded contexts.

During the hardening of the `deliver_gift` pipeline (Phase 3), we introduced a strict `Fail-Closed` DB transaction *inside* `LicenseStore::transfer_license` to mathematically eliminate the risk of license duplication (phantom licenses) during ownership transfer.
However, immediately after a successful license transfer, we must record an `EntryType::Gift` in the `EconomyLedger`. If this ledger record fails, we are left with a "Cross-Domain Inconsistency": the license has moved, but no audit trail exists.

## Decision
We will employ a **Fail-Soft** strategy for the Ledger audit logging during non-financial (zero-coin) transactions like `deliver_gift`.
If the `record_batch` operation fails after a successful `transfer_license`, we will:
1. Log a `tracing::error!` ("⚠️ Gift ledger audit record failed (gift itself succeeded)").
2. Return `Ok(())` to allow the business operation to complete without rolling back the external state (which is impossible anyway since the DB transaction in `LicenseStore` has already committed).

## Consequences

### Positive
- **High Availability**: Temporary ledger DB unavailability or lock contention will not block users/agents from receiving gifts.
- **Architectural Purity**: We maintain the strict trait segregation between `LicenseStore` and `EconomyLedger`, avoiding tight coupling or leaking DB-specific transaction objects into trait signatures.
- **Fail-Closed Guarantees**: The most critical vulnerability (license duplication or arbitrary creation) is fully protected within the isolated `LicenseStore` transaction.

### Negative
- **Phantom Transfers (Loss of Auditability)**: In rare failure scenarios, an asset may change ownership without a corresponding ledger entry, requiring manual database inspection to reconstruct the history.

## Future Considerations
If strict global consistency becomes a business requirement, we must refactor the infrastructure to support a Saga pattern (with compensating transactions) or introduce a global `TransactionManager` (Unit of Work) that spans multiple repositories. For Phase 3, the Fail-Soft approach provides the optimal balance of resilience and decoupling.
