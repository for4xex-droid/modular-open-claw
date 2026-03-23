# ADR-016: Subscription Engine Architecture

## Context
Aiome requires a mechanism for recurring payments/subscriptions to support continuous AI services (A2C - AI to Consumer). This engine needs to handle user-to-agent subscription flows, tiering, and billing status synchronization.

## Decision
1.  **Stripe Core Integration**: We will build the subscription engine on the Stripe Subscriptions / Billing API.
2.  **Tiered Logic**: Defined in `infrastructure/commerce/subscription/` where we map Stripe Product IDs to AI capability tiers (e.g., standard, premium, enterprise).
3.  **Webhook-Driven State**: AI internal states will react to subscription status updates (status, payment_failed, canceled) via the Webhook orchestrator.
4.  **Grace Period & Throttling**: If a subscription is in a failed state, the agent's creativity/resource levels (`fatigue`, `exp`) will be throttled or downgraded gracefully rather than a hard block.
5.  **Interface Consistency**: The `CommerceEngine` trait will be expanded or complemented with a `SubscriptionEngine` trait in `aiome-contracts`.

## Status
Proposed (Phase 27 Target)

## Consequences
- **Financial Scalability**: Easier monetization of individual AI agents.
- **Complexity**: Adds a recurring-state management layer to the database (beyond current one-off ledger transactions).
- **Compliance**: Requires robust handling of Stripe billing sessions and webhooks for data consistency.
