/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Where does this asset come from?
/// Crucial for compliance. Wild/custom assets MUST NOT sync to the P2P Hub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetOrigin {
    /// Built-in official asset (safe to sync ID)
    Official,
    /// Verified marketplace asset (safe to sync UUID)
    Marketplace(Uuid),
    /// Unverified local user-loaded asset (NURTURE Compliance Rule: NEVER SYNC TO HUB)
    LocalCustom,
}

/// Metadata about a visual asset (clothes, models, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub origin: AssetOrigin,
    pub file_path: String,
    pub model_type: ModelType,
    pub hash: String, // checksum for verification
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    Vrm,
    Inochi2d,
    StaticImage,
}

impl AssetManifest {
    /// SECURITY/COMPLIANCE BOUNDARY:
    /// Returns false if this asset contains unvetted local binary that would contaminate the Hub.
    pub fn is_hub_syncable(&self) -> bool {
        match self.origin {
            AssetOrigin::Official => true,
            AssetOrigin::Marketplace(_) => true,
            AssetOrigin::LocalCustom => false, // STRICT COMPLIANCE: Local binary stays local
        }
    }
}
