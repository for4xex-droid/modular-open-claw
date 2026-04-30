/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![forbid(unsafe_code)]
#![allow(
    unused_imports,
    unused_variables,
    dead_code,
    unused_mut,
    ambiguous_glob_reexports
)]

pub mod commerce;
pub mod error;
pub mod gig_metadata;
pub mod llm;
pub mod plugin;
pub mod proof;
pub mod rlm;
pub mod security;
pub mod x402;

pub use commerce::*;
pub use error::*;
pub use llm::*;
pub use plugin::*;
pub use rlm::*;
pub use security::*;
pub use x402::*;
