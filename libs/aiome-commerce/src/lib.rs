#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![forbid(unsafe_code)]

pub mod ekyc;
pub mod factory;
pub mod gift;
pub mod gig;
pub mod mock;
pub mod polar;
pub mod splitter;
pub mod stripe;
pub mod syndicate;
pub mod wallet;

// Re-exports for convenience
pub use crate::ekyc::StripeEkycEngine;
pub use crate::factory::CommerceEngineFactory;
pub use crate::gig::UniversalGigEngine;
pub use crate::polar::PolarCommerceEngine;
pub use crate::stripe::StripeCommerceEngine;
pub use crate::syndicate::UniversalSyndicateStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commerce_engine_structure() {
        // Verify module structure exists
    }
}
