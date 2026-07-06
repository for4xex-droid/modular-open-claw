# ADR-050: Monthly Spend Limit (OP-059 R2-3)

## Status
Accepted

## Context
OP-059 hybrid pricing includes a monthly KC allowance; operators also need a cap on agent spending per calendar month (W-7d). Daily limits already exist via `daily_spend_limit` / `spent_today`. Monthly limits require new policy and wallet fields plus DB migration.

## Decision

1. **`EconomyPolicy.monthly_spend_limit`** — global cap (default `0` = disabled / unlimited). Stored in existing `nurture_settings.economy_policy` JSON with `#[serde(default)]` for backward compatibility.

2. **`nurture_wallets.monthly_limit` / `spent_this_month`** — per-wallet cap and rolling counter (default `0` limit = inherit policy-only; `0` policy = unlimited).

3. **Effective limit** — when both policy and wallet limits are non-zero, use `min()` (mirrors daily). Skip check when effective limit is `0`.

4. **Reset** — `get_balance` zeroes `spent_this_month` when `last_reset` month ≠ current month. Debit path resets `spent_this_month` on month boundary before increment (mirrors daily `spent_today` reset).

5. **Enforcement** — `EconomyInterceptor`, `commerce_impl` debit/validate/escrow paths, and in-memory `CoinWallet::spend`.

6. **Settings UI** — `economy.monthly_spend_limit` in management-console Settings (cockpit commerce section). api-server relays to Nurture `/internal/economy-policy/monthly-limit` on update.

## Consequences
- New migration required for sqlite + postgres.
- `MonthlyLimitExceeded` maps to HTTP 403 (same as daily).
