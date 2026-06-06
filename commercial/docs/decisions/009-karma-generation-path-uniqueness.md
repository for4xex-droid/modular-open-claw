# ADR 009: Karma Generation Path Uniqueness

## Status
Accepted

## Context
Karma generation represents the reputation and trust level of agents within the Aiome/Nurture ecosystem. If multiple subsystems autonomously generate Karma for overlapping events, it leads to reputation inflation and inconsistent tracking. We identified that the payment webhook layer (Stripe/Polar) and the settlement layer (Bridge) could potentially both trigger Karma generation for the same underlying transaction.

## Decision
We mandate a Single Source of Truth (SSOT) propagation path for transaction-based Karma generation:
1. **Trigger Origin**: Webhook Handlers (`stripe/webhook.rs`, `polar/webhook.rs`).
2. **Interface**: `AgentHook::on_transaction_completed`.
3. **Consumer**: `NurtureAgentHook` (acting as a proxy) -> `KarmaForge::cross_synthesize`.

Other internal systems (e.g., `NurtureCommerceBridge::execute_purchase_step`, `SettlementProtocol`) **must not** generate financial Karma directly. They are restricted to generating Karma only for security events (e.g., CSAM violations).

## Consequences
- **Positive**: Prevents duplicate Karma generation and reputation inflation. Provides a clear audit trail from Webhook to KarmaForge.
- **Negative**: If a webhook fails to deliver but the internal transaction somehow succeeds, Karma will not be generated. This is mitigated by robust Webhook idempotency and retry mechanisms.
