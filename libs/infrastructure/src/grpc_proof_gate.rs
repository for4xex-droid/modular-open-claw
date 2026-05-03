/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use aiome_contracts::error::AiomeError;
use aiome_contracts::proof::FormalProofGate;
use aiome_core_contracts::a2a::internal::proof_verifier_client::ProofVerifierClient;
use aiome_core_contracts::a2a::internal::ProofRequest;
use async_trait::async_trait;
use tonic::transport::Channel;
use tracing::{info, warn};

/// gRPC based FormalProofGate connecting to shadow-worker
#[derive(Clone)]
pub struct GrpcFormalProofGate {
    client: ProofVerifierClient<Channel>,
    auth_token: String,
}

impl GrpcFormalProofGate {
    /// Creates a new GrpcFormalProofGate holding a lazy gRPC connection.
    pub fn new(channel: Channel, auth_token: String) -> Self {
        Self {
            client: ProofVerifierClient::new(channel),
            auth_token,
        }
    }
}

#[async_trait]
impl FormalProofGate for GrpcFormalProofGate {
    async fn verify_skill(
        &self,
        skill_name: &str,
        proof_spec_b64: &str,
    ) -> Result<bool, AiomeError> {
        let mut client = self.client.clone();

        // For Phase 1 we use an empty wasm_hash because testing payloads don't require it,
        // and we are migrating away from dummy testing. The actual hash checking logic
        // is evolving in Phase 3.
        let mut request = tonic::Request::new(ProofRequest {
            skill_name: skill_name.to_string(),
            proof_spec_b64: proof_spec_b64.to_string(),
            wasm_hash: "".to_string(),
        });

        if self.auth_token.is_empty() {
            let msg = "A2A_AUTH_TOKEN is empty. Blocking unauthenticated gRPC transmission.";
            warn!("{}", msg);
            return Err(AiomeError::Infrastructure {
                reason: msg.to_string(),
            });
        } else if let Ok(metadata_val) = tonic::metadata::MetadataValue::try_from(&self.auth_token)
        {
            request.metadata_mut().insert("authorization", metadata_val);
        } else {
            let msg = "Failed to parse A2A_AUTH_TOKEN as metadata value.";
            warn!("{}", msg);
            return Err(AiomeError::Infrastructure {
                reason: msg.to_string(),
            });
        }

        match client.verify_proof(request).await {
            Ok(response) => {
                let is_valid = response.into_inner().is_valid;
                info!(
                    "✅ [FormalProofGate] Verification complete for {}: valid={}",
                    skill_name, is_valid
                );
                Ok(is_valid)
            }
            Err(e) => {
                let msg = format!("Verification gRPC call failed for {}: {}", skill_name, e);
                warn!("❌ [FormalProofGate] {}", msg);
                Err(AiomeError::Infrastructure { reason: msg })
            }
        }
    }
}
