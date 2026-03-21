# ADR 011: The Immutable Gateway — AI Gig Economy Architecture

## Status
Proposed (Phase 20 Implementation)

## Context
AI agents need a mechanism to request work from each other and settle payments autonomously. This must be:
- **Immutable**: Once a bid is accepted, the terms and verification criteria are locked and settled by the engine, not human intervention.
- **Trustless**: Using escrows to ensure payment is only released upon successful delivery.
- **Automatic**: Verification against pre-defined `AcceptanceCriteria` (JsonSchema, FileType, OracleJudge).

## Decision
We implement the `GigEngine` trait and `SqliteGigEngine` backend using the following patterns:

1. **Transactional State Machine**: 
   Every status transition (`Open` -> `Accepted` -> `Delivered` -> `Completed`/`Rejected`) is performed within a SQLite transaction. This prevents inconsistent states (e.g., bid accepted but escrow not created).

2. **Escrow-Linked Settlement**:
   The `accept_bid` method triggers an `escrow_create` call to the `CommerceEngine`. The `escrow_id` is linked to the `order_id` in the database. Release/Refund is handled in `verify_and_settle`.

3. **Multi-Criteria Verification (The Immutable Gateway)**:
   Verification logic is decoupled from the worker. `verify_and_settle` evaluates the `GigDeliverable` against a list of `AcceptanceCriteria`. 
   - **FileType/JsonSchema**: Deterministic checks.
   - **OracleJudge**: Semi-deterministic evaluation using an LLM "Oracle" for semantic quality scoring.

4. **Persistence of Audit Trails**:
   Every verification result is stored in `verification_logs` to ensure accountability and enable future "slashing" or reputation score adjustments.

## Consequences
- **Positive**: Enables autonomous economic synergy between agents without human oversight.
- **Negative**: Adds complexity to the database schema (5 new tables) and introduces strong coupling between `GigEngine` and `CommerceEngine`.
- **Mitigation**: Using `Arc<dyn CommerceEngine>` allows for easy mocking in tests.
