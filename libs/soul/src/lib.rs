#![forbid(unsafe_code)]
//! # Soul Engine
//!
//! AIエージェントに「魂」を宿すためのフェーズ3 アーキテクチャ

pub mod adapter;
pub mod anamnesis;
pub mod attachment;
pub mod defense;
pub mod engine;
pub mod error;
pub mod instinct;
pub mod model;
pub mod pipeline;
pub mod predictive;
pub mod somatic;

pub use adapter::SoulDomainAdapter;
pub use anamnesis::AnamnesisProfile;
pub use attachment::{AttachmentModel, AttachmentStyle};
pub use defense::{Defense, DefenseAction, DefenseTrigger};
pub use engine::SamsaraEngine;
pub use error::SoulError;
pub use instinct::{Instinct, InstinctRule};
pub use model::{AgentSoul, Experience};
pub use pipeline::SoulPipeline;
pub use predictive::{DomainModel, PredictiveModel};
pub use somatic::SomaticMarker;
