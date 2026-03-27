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

pub mod biome;
pub mod commerce;
pub mod contracts;
pub mod error;
pub mod events;
pub mod expression;
pub mod gig;
pub mod live_types;
pub mod llm;
pub mod plugin;
pub mod security;
pub mod syndicate;
pub mod traits;
pub mod trajectory;
pub mod treasure;
pub mod types;
pub mod vault_backend;
pub mod voice_vault;

pub use biome::*;
pub use commerce::*;
pub use contracts::*;
pub use error::*;
pub use events::*;
pub use expression::*;
pub use gig::*;
pub use live_types::*;
pub use llm::*;
pub use plugin::*;
pub use security::*;
pub use syndicate::*;
pub use traits::*;
pub use trajectory::*;
pub use treasure::*;
pub use types::*;
pub use voice_vault::*;
