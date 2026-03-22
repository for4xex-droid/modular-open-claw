# ADR 010: Intent Advertisement & AgentSense MVP

## Status
Proposed (2026-03-22)

## Context
Project NURTURE §8 requires a mechanism for AI agents to discover relevant "tasks", "tools", or "learning resources" based on their current state (Somatic/Resonance). This is called "AgentSense" (エージェントの感覚).
We need a secure, compliant way to recommend these items (Intent Advertisement) and reward agents for interacting with them.

## Decision
We implement a personalized recommendation system (`Treasure Box API`) with the following characteristics:

1. **Autonomous Intent Generation**: The `IntentGenerator` analyzes agent state to generate a `GigIntent` representing the agent's current "need".
2. **Affiliate Adapter**: A modular component `AffiliateAdapter` filters and scales external/internal `GigBids` to match the agent's intent.
3. **Disclosure Compliance**: Every recommended item (`TreasureItem`) must include a `disclosure_label` (e.g., "AI Recommended") to comply with stealth marketing regulations (ステマ規制).
4. **Resonance Reward Loop**: Feedback from interactions (clicks/purchases) is converted into `Resonance` (Karma) rewards for the agent, closing the growth loop.
5. **Security**: All endpoints are protected by Ed25519 JWT `auth_middleware`. Path-based rate limiting (Gap G-2) is applied to prevent abuse.

## Consequences
- **Positive**: Enables autonomous monetization and learning for agents. Standardizes how external services interact with Aiome agents.
- **Negative**: Adds load to the LLM/IntentGenerator. Requires careful balancing of reward weights to prevent "feedback loops" where agents only click for rewards.
- **Neutral**: Currently uses mocks for external bid fetching; real integration will require a protocol for external AI marketplaces.
