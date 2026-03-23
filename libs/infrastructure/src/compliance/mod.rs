/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

/// 音声 CSAM ハッシュ (知覚ハッシュ)
pub mod audio_hasher;
/// 年齢確認 (Stripe Identity)
pub mod ekyc;
/// eKYCセッション永続化
pub mod ekyc_store;
/// アセット検疫 (CSAM/Compliance)
pub mod quarantine;

pub use audio_hasher::AudioHasher;
pub use ekyc::{EkycEngine, EkycSession, StripeEkycEngine};
#[cfg(any(test, debug_assertions))]
pub use ekyc::MockEkycEngine;
pub use ekyc_store::{EkycSessionStore, SqliteEkycSessionStore};
#[cfg(any(test, debug_assertions))]
pub use ekyc_store::MockEkycSessionStore;
pub use quarantine::{AssetReason, QuarantineStore, SqliteQuarantineStore};
