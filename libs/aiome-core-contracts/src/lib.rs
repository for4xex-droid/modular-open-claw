/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![forbid(unsafe_code)]
#![allow(
    unused_imports,
    unused_variables,
    dead_code,
    unused_mut,
    ambiguous_glob_reexports
)]

pub use aiome_contracts::*;

// Internal Modules
pub mod a2a;
pub mod audit;
pub mod biome;
pub mod contracts;
pub mod ekyc;
pub mod events;
pub mod expression;
pub mod forecast;
pub mod gig;
pub mod invariant;
pub mod live_types;
pub mod lora_marketplace;
pub mod syndicate;
pub mod traits;
pub mod trajectory;
pub mod treasure;
pub mod types;
pub mod vault_backend;
pub mod voice_vault;

pub use a2a::*;
pub use audit::*;
pub use biome::*;
pub use contracts::*;
pub use ekyc::*;
pub use events::*;
pub use expression::*;
pub use forecast::*;
pub use gig::*;
pub use invariant::*;
pub use live_types::*;
pub use syndicate::*;
pub use traits::*;
pub use trajectory::*;
pub use treasure::*;
pub use types::*;
pub use vault_backend::*;
pub use voice_vault::*;
pub mod oxilean;
