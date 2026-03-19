/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

/// 画像ハッシュ生成
pub mod image_hash;
/// モデル頭身判定 (5.5頭身)
pub mod proportions;

pub use image_hash::ImageHasher;
pub use proportions::{LegalStatus, ProportionsChecker};
