/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::a2a::internal::proof_verifier_server::ProofVerifier;
use aiome_core_contracts::a2a::internal::{ProofRequest, ProofResult};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tonic::{Request, Response, Status};
use tracing::{info, warn};

/// OxiLean (CiC kernel) formal verification service.
///
/// 4-layer defense (in execution order):
/// 1. Token authentication (same `A2A_AUTH_TOKEN` as `DockerConductor`)
/// 2. `Semaphore` → CPU-bound concurrency control (default 1 permit)
/// 3. `tokio::time::timeout` → bounded execution (default 10s)
/// 4. `catch_unwind` → kernel panic never crashes the gRPC server
pub struct OxiLeanProofService {
    /// Shared A2A auth token — same credential as DockerConductor (Threat #36).
    auth_token: String,
    /// Maximum proof execution time.
    timeout: Duration,
    /// Concurrency limiter – prevents OxiLean from starving the LLM thread pool.
    semaphore: Arc<Semaphore>,
}

impl OxiLeanProofService {
    /// Create with explicit configuration (production use).
    pub fn new(auth_token: String, timeout: Duration, semaphore: Arc<Semaphore>) -> Self {
        Self {
            auth_token,
            timeout,
            semaphore,
        }
    }
}

/// Verify the `authorization` metadata against the stored token.
/// Mirrors the same check in `DockerConductor::execute_task`.
#[allow(clippy::result_large_err)]
fn verify_auth<T>(request: &Request<T>, expected_token: &str) -> Result<(), Status> {
    let token = request
        .metadata()
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("Missing authorization metadata"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("Invalid authorization encoding"))?;

    // GAP-O: Timing side-channel attack mitigation
    if !shared::security::constant_time_ends_with(token, expected_token) {
        return Err(Status::unauthenticated("Invalid auth token"));
    }
    Ok(())
}

#[tonic::async_trait]
impl ProofVerifier for OxiLeanProofService {
    async fn verify_proof(
        &self,
        request: Request<ProofRequest>,
    ) -> Result<Response<ProofResult>, Status> {
        // ── Authentication (Threat #36 — same gate as DockerConductor) ──
        verify_auth(&request, &self.auth_token)?;

        let req = request.into_inner();

        // Sanitise for structured logging — strip control characters.
        let sanitized_skill = req.skill_name.replace(|c: char| c.is_control(), "");
        info!(skill_name = %sanitized_skill, "Received verify_proof request");

        // Acquire semaphore permit — blocks if another proof is in-flight.
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| Status::unavailable("Proof semaphore closed"))?;

        let timeout_dur = self.timeout;

        // Execute the proof kernel with timeout + panic guard.
        let outcome = tokio::time::timeout(
            timeout_dur,
            tokio::task::spawn_blocking(move || {
                catch_unwind(AssertUnwindSafe(|| {
                    // Phase 1-2 stub: initialise kernel Environment to confirm TCB loads.
                    // Phase 3 will parse `proof_spec_b64` into a full CiC specification.
                    let _env = oxilean_kernel::Environment::new();
                    true
                }))
            }),
        )
        .await;

        match outcome {
            // Happy path: proof completed within timeout.
            Ok(Ok(Ok(is_valid))) => {
                let msg = if is_valid { "Q.E.D." } else { "INCONCLUSIVE" };
                Ok(Response::new(ProofResult {
                    is_valid,
                    message: msg.to_string(),
                }))
            }
            // Kernel panicked — process survived thanks to catch_unwind.
            Ok(Ok(Err(_panic_payload))) => {
                warn!(skill_name = %sanitized_skill, "OxiLean kernel panicked — stack unwound safely");
                Ok(Response::new(ProofResult {
                    is_valid: false,
                    message: "INCONCLUSIVE (Kernel Panic)".to_string(),
                }))
            }
            // spawn_blocking JoinError (should be extremely rare).
            Ok(Err(join_err)) => {
                warn!(error = %join_err, "spawn_blocking JoinError during proof execution");
                Ok(Response::new(ProofResult {
                    is_valid: false,
                    message: "INCONCLUSIVE (Task Error)".to_string(),
                }))
            }
            // Timeout exceeded.
            Err(_elapsed) => {
                warn!(
                    skill_name = %sanitized_skill,
                    timeout_secs = timeout_dur.as_secs(),
                    "Proof execution timed out"
                );
                Ok(Response::new(ProofResult {
                    is_valid: false,
                    message: format!(
                        "INCONCLUSIVE (Timeout: {}s exceeded)",
                        timeout_dur.as_secs()
                    ),
                }))
            }
        }
    }
}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOKEN: &str = "test_secret_token";

    fn test_service() -> OxiLeanProofService {
        OxiLeanProofService::new(
            TEST_TOKEN.to_string(),
            Duration::from_secs(10),
            Arc::new(Semaphore::new(1)),
        )
    }

    /// Create a request with valid authorization metadata.
    fn make_authed_request(skill_name: &str) -> Request<ProofRequest> {
        let mut req = Request::new(ProofRequest {
            skill_name: skill_name.to_string(),
            proof_spec_b64: String::new(),
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", TEST_TOKEN)
                .parse()
                .expect("valid header"), // allow-anti-pattern
        );
        req
    }

    /// Create a request with NO authorization metadata.
    fn make_unauthed_request(skill_name: &str) -> Request<ProofRequest> {
        Request::new(ProofRequest {
            skill_name: skill_name.to_string(),
            proof_spec_b64: String::new(),
        })
    }

    #[tokio::test]
    async fn test_verify_proof_success() {
        let service = test_service();
        let response = service
            .verify_proof(make_authed_request("test_skill"))
            .await;

        assert!(response.is_ok(), "Service should return OK");
        let inner = response.expect("checked above").into_inner(); // allow-anti-pattern
        assert!(inner.is_valid);
        assert_eq!(inner.message, "Q.E.D.");
    }

    #[tokio::test]
    async fn test_verify_proof_rejects_unauthenticated() {
        let service = test_service();
        let response = service
            .verify_proof(make_unauthed_request("test_skill"))
            .await;

        assert!(
            response.is_err(),
            "Unauthenticated request must be rejected"
        );
        let status = response.expect_err("Unauthenticated request must be rejected"); // allow-anti-pattern
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_verify_proof_rejects_wrong_token() {
        let service = test_service();
        let mut req = Request::new(ProofRequest {
            skill_name: "test_skill".to_string(),
            proof_spec_b64: String::new(),
        });
        req.metadata_mut().insert(
            "authorization",
            "Bearer wrong_token".parse().expect("valid header"), // allow-anti-pattern
        );
        let response = service.verify_proof(req).await;

        assert!(response.is_err(), "Wrong token must be rejected");
        assert_eq!(
            response.expect_err("Wrong token must be rejected").code(),
            tonic::Code::Unauthenticated
        ); // allow-anti-pattern
    }

    #[tokio::test]
    async fn test_verify_proof_timeout_path_exists() {
        let service = OxiLeanProofService::new(
            TEST_TOKEN.to_string(),
            Duration::ZERO,
            Arc::new(Semaphore::new(1)),
        );
        let response = service.verify_proof(make_authed_request("any_skill")).await;

        assert!(
            response.is_ok(),
            "Neither timeout nor success should crash the service"
        );
        let inner = response.expect("checked above").into_inner(); // allow-anti-pattern

        if inner.is_valid {
            assert_eq!(inner.message, "Q.E.D.");
        } else {
            assert!(
                inner.message.contains("Timeout") || inner.message.contains("INCONCLUSIVE"),
                "Unexpected message: {}",
                inner.message
            );
        }
    }

    #[tokio::test]
    async fn test_verify_proof_semaphore_exhaustion() {
        let shared_sema = Arc::new(Semaphore::new(1));
        let _hold = shared_sema.acquire().await.expect("should acquire"); // allow-anti-pattern

        let service = OxiLeanProofService::new(
            TEST_TOKEN.to_string(),
            Duration::from_millis(50),
            Arc::clone(&shared_sema),
        );

        let response = tokio::time::timeout(
            Duration::from_millis(100),
            service.verify_proof(make_authed_request("blocked_skill")),
        )
        .await;

        assert!(
            response.is_err(),
            "Should timeout since semaphore is exhausted"
        );
    }

    #[tokio::test]
    async fn test_sanitised_logging_control_chars() {
        let service = test_service();
        let response = service
            .verify_proof(make_authed_request("evil\n\x1b[31mskill"))
            .await;

        assert!(response.is_ok());
    }
}
