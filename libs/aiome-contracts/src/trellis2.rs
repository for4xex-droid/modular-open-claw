/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

/// [Phase M-3] TRELLIS.2 Asset Generation MCP Definition
/// This struct defines the schema for the TRELLIS.2 generation tool
/// which Agents will invoke to convert 2D avatar images to 3D GLB assets.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrellisGenerateRequest {
    /// Base64 encoded or publicly accessible URL of the source 2D image
    pub image_source: String,
    
    /// True if the input is a base64 encoded image, false if it's a URL
    #[serde(default)]
    pub is_base64: bool,
    
    /// Expected output quality or generation preset (e.g., "high", "fast")
    #[serde(default = "default_preset")]
    pub preset: String,
}

fn default_preset() -> String {
    "high".to_string()
}

/// Response returned from the TRELLIS.2 generation service
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrellisGenerateResponse {
    /// URL to the finalized `.glb` asset in the blob store
    pub asset_url: String,
    
    /// Estimated polygon count of the generated model
    pub polygon_count: u32,
    
    /// Time taken to generate the model in milliseconds
    pub generation_time_ms: u64,
}
