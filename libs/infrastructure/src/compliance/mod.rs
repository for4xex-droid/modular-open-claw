/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

/// 年齢確認 (Stripe Identity)
pub mod ekyc;
/// eKYCセッション永続化
pub mod ekyc_store;
/// アセット検疫 (CSAM/Compliance)
pub mod quarantine;
/// 音声 CSAM ハッシュ (知覚ハッシュ)
pub mod audio_hasher;

pub use ekyc::{EkycEngine, MockEkycEngine, StripeEkycEngine, EkycSession};
pub use ekyc_store::{EkycSessionStore, SqliteEkycSessionStore, MockEkycSessionStore};
pub use quarantine::{AssetReason, QuarantineStore, SqliteQuarantineStore};
pub use audio_hasher::AudioHasher;
