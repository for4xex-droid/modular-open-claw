/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![forbid(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
//! # Soul Engine
//!
//! AIエージェントに「魂」を宿すためのフェーズ3 アーキテクチャ

pub mod adapter;
pub mod anamnesis;
pub mod attachment;
pub mod bounding_middleware;
pub mod defense;
pub mod engine;
pub mod error;
pub mod instinct;
pub mod model;
pub mod pipeline;
pub mod predictive;
pub mod semantic_recaller;
pub mod somatic;

pub use adapter::SoulDomainAdapter;
pub use anamnesis::AnamnesisProfile;
pub use attachment::{AttachmentModel, AttachmentStyle};
pub use bounding_middleware::BoundingGuard;
pub use defense::{Defense, DefenseAction, DefenseTrigger};
pub use engine::SamsaraEngine;
pub use error::SoulError;
pub use instinct::{Instinct, InstinctRule};
pub use model::{AgentSoul, Experience};
pub use pipeline::SoulPipeline;
pub use predictive::{DomainModel, PredictiveModel};
pub use semantic_recaller::SemanticRecaller;
pub use somatic::SomaticMarker;
