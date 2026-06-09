/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

//! Aiome 実装詳細への唯一の接点。将来 HTTP/gRPC に移行可能。

pub mod auth {
    #[cfg(any(test, debug_assertions))]
    pub use shared::auth::MockAuthManager;
    pub use shared::auth::{AiomeCustomClaims, AuthManager, JwtAuthManager};
}

pub mod llm {
    #[cfg(any(test, debug_assertions))]
    pub use aiome_core::llm_provider::MockLlmProvider;
    pub use aiome_core::llm_provider::OllamaProvider;
}

pub mod db {
    pub use shared::db::{DatabasePool, DatabaseTransaction};
}

pub use shared::sql_exec;

pub mod security {
    pub use infrastructure::security::SafeCommandBuilder;
    pub use shared::security::scrub_env;
}

pub mod csam {
    pub mod image_hash {
        pub use shared::csam::image_hash::ImageHasher;
    }
}

pub mod container_runtime {
    pub use shared::container_runtime::detect_runtime;
}

pub mod watchtower {
    pub use shared::watchtower::{AgentStats, CoreEvent, LogEntry, SystemStatus};
}

pub mod job_queue {
    pub use infrastructure::job_queue::{EvaluationOps, UniversalJobQueue};
    pub mod trajectory_store {
        pub use infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore;
    }
}

pub mod trajectory {
    pub use aiome_core_contracts::trajectory::TrajectoryStore;
}

pub mod traits {
    pub use aiome_core_contracts::traits::{JobQueue, LoraEngine, TtsProvider};
}

pub mod immune_system {
    pub use infrastructure::immune_system::AdaptiveImmuneSystem;
}

pub mod supervisor {
    pub use infrastructure::supervisor::{SupervisedTask, TaskSupervisor};
}

pub mod commerce {
    pub use aiome_core_contracts::commerce::{CommerceEngine, EscrowRecord};
}

pub mod error {
    pub use aiome_core_contracts::error::AiomeError;
}

pub mod plugin {
    pub use aiome_core_contracts::plugin::{AiomePlugin, OpaqueRouter};
    pub use aiome_core_contracts::security::AgentHook;
}

pub mod oxilean {
    pub use aiome_core_contracts::oxilean::OxiLeanProofCertificate;
}

// Round 4: 自律進化系の re-export
pub mod evolution {
    pub use infrastructure::job_queue::evolution::EvolutionOps;
    pub use infrastructure::job_queue::taxonomy::KarmaTaxonomy;
}

pub mod contracts {
    pub use aiome_core_contracts::contracts::{KarmaClassification, SamsaraEvent};
    pub use aiome_core_contracts::traits::AgentEvolver;
}

// Round 5: 自律対話の re-export
pub mod commune {
    pub use aiome_core::commune::{AutonomousCommuneEngine, AutonomousConfig};
    pub use aiome_core_contracts::commune::CommuneMessage;
}

pub use aiome_core_contracts::{LlmRequest, LlmResponse};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_synergy_reexports() {
        let _ops: Option<Box<dyn evolution::EvolutionOps>> = None;
        let _taxonomy: Option<evolution::KarmaTaxonomy> = None;
        let _class: Option<contracts::KarmaClassification> = None;
        let _event: Option<contracts::SamsaraEvent> = None;
        let _evolver: Option<Box<dyn contracts::AgentEvolver>> = None;
        let _cfg: Option<commune::AutonomousConfig> = None;
        let _msg: Option<commune::CommuneMessage> = None;
        let _lora: Option<Box<dyn traits::LoraEngine>> = None;
        let _tts: Option<Box<dyn traits::TtsProvider>> = None;
    }
}
