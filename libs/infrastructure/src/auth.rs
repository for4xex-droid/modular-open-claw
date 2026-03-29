/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

//! Infrastructure Auth Bridge
//!
//! This module now re-exports from `libs/shared::auth` to support modularization.

pub use shared::auth::{AiomeCustomClaims, AuthManager, JwtAuthManager};

#[cfg(any(test, debug_assertions))]
pub use shared::auth::MockAuthManager;
