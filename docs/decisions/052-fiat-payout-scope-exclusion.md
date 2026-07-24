# ADR-052: Fiat Payout Scope Exclusion

## Status
Accepted

## Context
Project NURTURE originally envisioned creator rewards paid in fiat currency and optional cash-out of AiomeCoin. Operating a prepaid payment instrument (AiomeCoin) with user-to-user transfers or fiat redemption triggers Payment Services Act (資金決済法) exchange-transaction and refund obligations that exceed current operational and legal capacity.

Remnants of the removed payout path remain in schema (`nurture_payout_requests`), API naming (`/withdraw`), and legal copy ("収益の出金"), creating compliance ambiguity.

## Decision

1. **No fiat payout from AiomeCoin or Creator Points (CP)** — CP converts to AiomeCoin only (`withdraw_points` / future `/convert-points`). No bank transfer, Tremendous, or gift-card redemption of CP balances.

2. **`TremendousGiftEngine` is A2C-only** — External gifts are triggered by AI symbiotic reward (D-6), not CP withdrawal. Budget caps remain $5/gift and $20/day aggregate (`gift.rs`).

3. **P2P coin transfer blocked by default** — `EconomyPolicy.allow_p2p_transfer` defaults to `false` (ADR-052 + prepaid instrument hygiene). See nurture quality plan Phase B-1.

4. **Supersede prior CP→gift proposals** — `REMAINING_TASKS.md` L90–91 and `UNCERTAINTY_BREAKTHROUGH.md` CP→Tremendous paths are out of scope.

5. **Schema cleanup** — Drop dead `nurture_payout_requests` table; rename public API from `/withdraw` to `/convert-points` with one-release alias.  
   **Update (2026-07-25)**: Alias `/api/v1/commerce/withdraw` **removed** ahead of the 2026-08-01 sunset (NR-14 / Wave 0c). Use `/api/v1/commerce/convert-points` only.

## Consequences
- Creators monetize via in-ecosystem CP→Coin conversion and marketplace sales, not direct fiat withdrawal through Aiome.
- Legal copy and OpenAPI must use "convert" / "exchange" language, not "withdraw" / "cash out".
- B-1 P2P block and A-2 DROP depend on this ADR as the policy anchor.
