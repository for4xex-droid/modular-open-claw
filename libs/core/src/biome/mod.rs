/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

pub use aiome_contracts::biome::*;

pub mod autonomous;
pub mod dialogue;

// Re-export autonomous engine and config to maintain compatibility with existing code
pub use autonomous::{AutonomousBiomeEngine, AutonomousConfig};
