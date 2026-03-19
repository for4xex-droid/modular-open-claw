/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

/// 年齢確認 (Stripe Identity)
pub mod ekyc;
/// アセット検疫 (CSAM/Compliance)
pub mod quarantine;

pub use ekyc::{EkycEngine, MockEkycEngine, StripeEkycEngine};
pub use quarantine::{AssetReason, QuarantineStore, SqliteQuarantineStore};
