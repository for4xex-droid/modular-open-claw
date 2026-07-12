/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

//! Policy × wallet spend-limit synthesis (OP-083-B / ADR-050 hardening).
//!
//! Daily and monthly use **different** zero semantics (do not unify):
//! - **Daily** `0` = hard cap of zero (positive spend rejected via `min`)
//! - **Monthly** `0` = unlimited (skip check when effective is 0)

use commerce_protocol::error::NurtureError;

/// Daily effective limit = `wallet_limit.min(policy_limit)`.
///
/// Unlike monthly, `0` is **not** unlimited: `(10000, 0)` and `(0, 10000)` both yield `0`
/// (positive spend rejected). Matches interceptor / commerce_impl raw `min`.
#[inline]
pub fn effective_daily_limit(wallet_limit: u64, policy_limit: u64) -> u64 {
    wallet_limit.min(policy_limit)
}

/// Monthly effective limit: stricter of wallet vs policy; either side `0` means
/// "that side unlimited" so the other side wins; both `0` → `0` (unlimited).
#[inline]
pub fn effective_monthly_limit(wallet_limit: u64, policy_limit: u64) -> u64 {
    match (wallet_limit, policy_limit) {
        (0, 0) => 0,
        (w, 0) => w,
        (0, p) => p,
        (w, p) => w.min(p),
    }
}

/// Projected daily spend after `amount`. Returns `DailyLimitExceeded` when
/// `amount > 0` and projection exceeds the effective daily limit.
pub fn check_daily_spend(
    spent_today: u64,
    amount: u64,
    wallet_daily_limit: u64,
    policy_daily_limit: u64,
) -> Result<(), NurtureError> {
    if amount == 0 {
        return Ok(());
    }
    let limit = effective_daily_limit(wallet_daily_limit, policy_daily_limit);
    let projected = spent_today.checked_add(amount).ok_or_else(|| {
        NurtureError::PolicyViolation(
            "システムエラー: 支出計算のオーバーフローを検知しました".to_string(),
        )
    })?;
    if projected > limit {
        return Err(NurtureError::DailyLimitExceeded {
            limit,
            current: projected,
        });
    }
    Ok(())
}

/// Projected monthly spend after `amount`. Skips when effective monthly is `0`
/// (unlimited). Returns `MonthlyLimitExceeded` when over the cap.
pub fn check_monthly_spend(
    spent_this_month: u64,
    amount: u64,
    wallet_monthly_limit: u64,
    policy_monthly_limit: u64,
) -> Result<(), NurtureError> {
    if amount == 0 {
        return Ok(());
    }
    let limit = effective_monthly_limit(wallet_monthly_limit, policy_monthly_limit);
    if limit == 0 {
        return Ok(());
    }
    let projected = spent_this_month.checked_add(amount).ok_or_else(|| {
        NurtureError::PolicyViolation(
            "システムエラー: 支出計算のオーバーフローを検知しました".to_string(),
        )
    })?;
    if projected > limit {
        return Err(NurtureError::MonthlyLimitExceeded {
            limit,
            current: projected,
        });
    }
    Ok(())
}

/// Combined daily + monthly checks (NurtureError). Callers that need `AiomeError`
/// should map, or call [`effective_daily_limit`] / [`effective_monthly_limit`] directly.
pub fn check_spend_limits(
    spent_today: u64,
    spent_this_month: u64,
    amount: u64,
    wallet_daily_limit: u64,
    policy_daily_limit: u64,
    wallet_monthly_limit: u64,
    policy_monthly_limit: u64,
) -> Result<(), NurtureError> {
    check_daily_spend(spent_today, amount, wallet_daily_limit, policy_daily_limit)?;
    check_monthly_spend(
        spent_this_month,
        amount,
        wallet_monthly_limit,
        policy_monthly_limit,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_min_zero_is_hard_cap() {
        assert_eq!(effective_daily_limit(10_000, 0), 0);
        assert_eq!(effective_daily_limit(0, 10_000), 0);
        assert_eq!(effective_daily_limit(100, 50), 50);
        assert_eq!(effective_daily_limit(50, 100), 50);
    }

    #[test]
    fn monthly_zero_is_unlimited_side() {
        assert_eq!(effective_monthly_limit(0, 0), 0);
        assert_eq!(effective_monthly_limit(500, 0), 500);
        assert_eq!(effective_monthly_limit(0, 500), 500);
        assert_eq!(effective_monthly_limit(100, 50), 50);
    }

    #[test]
    fn daily_zero_limit_rejects_positive_spend() {
        let err = check_daily_spend(0, 1, 10_000, 0).unwrap_err();
        assert!(matches!(
            err,
            NurtureError::DailyLimitExceeded {
                limit: 0,
                current: 1
            }
        ));
        let err = check_daily_spend(0, 1, 0, 10_000).unwrap_err();
        assert!(matches!(
            err,
            NurtureError::DailyLimitExceeded {
                limit: 0,
                current: 1
            }
        ));
    }

    #[test]
    fn monthly_unlimited_allows_large_spend() {
        assert!(check_monthly_spend(0, 1_000_000, 0, 0).is_ok());
        assert!(check_monthly_spend(0, 100, 500, 0).is_ok());
    }

    #[test]
    fn monthly_cap_rejects() {
        let err = check_monthly_spend(400, 200, 500, 0).unwrap_err();
        assert!(matches!(
            err,
            NurtureError::MonthlyLimitExceeded {
                limit: 500,
                current: 600
            }
        ));
    }

    #[test]
    fn zero_amount_skips() {
        assert!(check_spend_limits(0, 0, 0, 0, 0, 0, 0).is_ok());
        assert!(check_spend_limits(999, 999, 0, 0, 0, 1, 1).is_ok());
    }

    #[test]
    fn daily_overflow_is_policy_violation() {
        let err = check_daily_spend(u64::MAX, 1, u64::MAX, u64::MAX).unwrap_err();
        assert!(matches!(err, NurtureError::PolicyViolation(_)));
    }
}
