/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instinct {
    pub rules: Vec<InstinctRule>,
    pub prompt_fragment: String,
    pub hash: String,
}

impl Default for Instinct {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            prompt_fragment: String::new(),
            hash: "0000000000000000".to_string(), // genesis hash
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstinctRule {
    pub generation_origin: u32,
    pub rule: String,
    pub confidence: f64,
}
