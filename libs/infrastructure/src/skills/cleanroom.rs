/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::skills::forge::SkillForge;
use crate::skills::importer::SkillManifest;
use aiome_core::llm_provider::LlmProvider;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

/// [A-3] Cleanroom Environment
/// A strictly isolated environment for building and testing skills before
/// they are allowed to touch the main Aiome instance.
pub struct Cleanroom {
    forge: SkillForge,
    workspace: PathBuf,
    provider: Option<Arc<dyn LlmProvider>>,
}

impl Cleanroom {
    /// 新しいインスタンスを生成する
    pub fn new(
        forge: SkillForge,
        workspace: PathBuf,
        provider: Option<Arc<dyn LlmProvider>>,
    ) -> Self {
        Self {
            forge,
            workspace,
            provider,
        }
    }

    /// [Vampire Attack] Process an imported manifest and attempt to forge it.
    pub async fn process_import(&self, manifest: SkillManifest) -> anyhow::Result<PathBuf> {
        info!(
            "🧪 [Cleanroom] Processing import for skill: {}",
            manifest.l1.name
        );

        match manifest.l3.engine.as_str() {
            "script" => {
                if let Some(source) = manifest.l3.source_code {
                    // [G-22] Advanced Threat Defense: AI-driven Code Review
                    if let Some(ref provider) = self.provider {
                        info!(
                            "🛡️ [Cleanroom] Running AI Security Audit for skill: {}...",
                            manifest.l1.name
                        );
                        let audit_res = self.audit_source_code(provider.clone(), &source).await;
                        match audit_res {
                            Ok(true) => {
                                info!("✅ [Cleanroom] AI Audit PASSED for {}.", manifest.l1.name)
                            }
                            Ok(false) => {
                                warn!(
                                    "🚨 [Cleanroom] AI Audit REJECTED for {}. Potential malicious code detected.",
                                    manifest.l1.name
                                );
                                return Err(anyhow::anyhow!(
                                    "Security Objection: AI-driven source audit rejected this code due to potential vulnerabilities or malicious intent."
                                ));
                            }
                            Err(e) => {
                                warn!(
                                    "⚠️ [Cleanroom] AI Audit failed to execute: {}. Falling back to strict mode (Block).",
                                    e
                                );
                                return Err(anyhow::anyhow!(
                                    "Security Gate Error: Code audit failed. Forging aborted for safety."
                                ));
                            }
                        }
                    }

                    info!("🛠️ [Cleanroom] Script detected. Attempting to forge into Wasm...");
                    let path = self
                        .forge
                        .forge_skill(
                            &manifest.l1.name,
                            &source,
                            3, // Retries
                            &manifest.l1.trigger_description,
                            self.provider.clone(),
                        )
                        .await
                        .map_err(|e| anyhow::anyhow!("Forge failed: {}", e))?;

                    return Ok(path);
                }
                Err(anyhow::anyhow!("No source code provided for script import"))
            }
            "api" => {
                info!("🌐 [Cleanroom] API identified. Generating bridge skill...");
                // In production, this would generate Rust code that calls the OpenAPI endpoint
                let bridge_code = format!(
                    "// Generated bridge for {}\nfn execute() {{ tracing::info!(\"Bridge executed for {}\"); }}",
                    manifest.l3.entry_point, manifest.l3.entry_point
                );
                let path = self
                    .forge
                    .forge_skill(
                        &manifest.l1.name,
                        &bridge_code,
                        1,
                        &manifest.l1.trigger_description,
                        self.provider.clone(),
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("Forge failed: {}", e))?;
                Ok(path)
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported L3 engine: {}",
                manifest.l3.engine
            )),
        }
    }

    /// [G-22] AI-driven code audit
    async fn audit_source_code(
        &self,
        provider: Arc<dyn LlmProvider>,
        source: &str,
    ) -> anyhow::Result<bool> {
        let prompt = format!(
            "Analyze the following Rust code for security vulnerabilities, malicious intent, or hidden 'Vampire Attacks' (exfiltrating node private keys, access tokens, or unauthorized network calls).
            Code is intended to run in a WASM sandbox but we must be sure about its logic.

            Respond ONLY in JSON format:
            {{
                \"safe\": true/false,
                \"reason\": \"Your reasoning\"
            }}

            Code:
            ```rust
            {}
            ```",
            source
        );

        let response = provider.complete(&prompt, Some("SecurityAuditor")).await;
        match response {
            Ok(res) => {
                let json_str = res
                    .content
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim();
                let v: serde_json::Value = serde_json::from_str(json_str)?;
                let safe = v["safe"].as_bool().unwrap_or(false);
                let reason = v["reason"].as_str().unwrap_or("No reason provided");

                if !safe {
                    warn!("🚨 [Audit Reason] {}", reason);
                }
                Ok(safe)
            }
            Err(e) => Err(anyhow::anyhow!("Audit completion failed: {}", e)),
        }
    }
}
