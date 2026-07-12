/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

#![forbid(unsafe_code)]

pub mod a2a;
pub mod a2c;
pub mod anomaly;
pub mod b2a;
pub mod coin;
pub mod customer;
pub mod karma_package;
pub mod ledger;
pub mod license;
pub mod points;
pub mod policy;
pub mod spend_guard;
pub mod uow;
pub use coin::*;
pub use customer::*;
pub use ledger::*;
pub use license::*;
pub use points::*;
pub use policy::*;
pub use spend_guard::{
    check_daily_spend, check_monthly_spend, check_spend_limits, effective_daily_limit,
    effective_monthly_limit,
};
