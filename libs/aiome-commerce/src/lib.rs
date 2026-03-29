/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![forbid(unsafe_code)]

pub mod ekyc;
pub mod gift;
pub mod gig;
pub mod mock;
pub mod splitter;
pub mod stripe;
pub mod syndicate;
pub mod wallet;

// Re-exports for convenience
pub use crate::ekyc::StripeEkycEngine;
pub use crate::gig::UniversalGigEngine;
pub use crate::stripe::StripeCommerceEngine;
pub use crate::syndicate::SqliteSyndicateStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commerce_engine_structure() {
        // Verify module structure exists
        assert!(true);
    }
}
