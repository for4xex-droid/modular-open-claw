/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::oss_ast_analyzer::OssAstAnalyzer;
use crate::oss_repository_indexer::OssRepositoryIndexer;
use crate::oss_type_matcher::{OssAdapterCodeGen, OssTypeMatcher};
use crate::skills::forge::SkillForge;
use aiome_contracts::llm::LlmProvider;
use aiome_core::error::AiomeError;
use std::sync::Arc;
use tracing::info;

use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_contracts::traits::Job;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// OSS インテグレーションの全工程を統括するオーケストレーター
pub struct OssIntegrationOrchestrator {
    indexer: OssRepositoryIndexer,
    analyzer: OssAstAnalyzer,
    matcher: OssTypeMatcher,
    codegen: OssAdapterCodeGen,
    forge: SkillForge,
    llm: Arc<dyn LlmProvider>,
}

impl OssIntegrationOrchestrator {
    pub fn new(
        indexer: OssRepositoryIndexer,
        analyzer: OssAstAnalyzer,
        matcher: OssTypeMatcher,
        codegen: OssAdapterCodeGen,
        forge: SkillForge,
        llm: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            indexer,
            analyzer,
            matcher,
            codegen,
            forge,
            llm,
        }
    }

    pub async fn integrate(
        &self,
        github_url: &str,
        target_name: &str,
        local_surface_path: Option<&std::path::Path>,
    ) -> Result<String, AiomeError> {
        self.execute_integration(github_url, target_name, local_surface_path, None, None)
            .await
    }

    async fn execute_integration(
        &self,
        github_url: &str,
        target_name: &str,
        local_surface_path: Option<&std::path::Path>,
        progress_tx: Option<mpsc::Sender<TaskEvent>>,
        job_id: Option<String>,
    ) -> Result<String, AiomeError> {
        info!(
            "🚀 [OssOrchestrator] Starting autonomous integration for: {}",
            github_url
        );

        let conductor_name = self.conductor_name().to_string();
        let send_progress = |msg: String, pct: u8| {
            let ptx = progress_tx.clone();
            let jid = job_id.clone();
            let cname = conductor_name.clone();
            async move {
                if let (Some(tx), Some(id)) = (ptx, jid) {
                    let _ = tx
                        .send(TaskEvent::Progress {
                            job_id: id,
                            conductor_id: cname,
                            message: msg,
                            percent: Some(pct),
                        })
                        .await;
                }
            }
        };

        send_progress(format!("Step 1: Cloning and indexing {}", github_url), 10).await;

        let session = self.indexer.clone_and_index(github_url, &["/"]).await?;
        info!(
            "✅ [OssOrchestrator] Step 1: Repository indexed at {:?}",
            session.temp_dir
        );

        send_progress("Step 2: Analyzing OSS AST".to_string(), 30).await;
        // 2. Step 2: Analyze AST (OSS)
        let oss_surfaces = self.analyzer.analyze_directory(&session.temp_dir)?;
        let mut combined_oss_surface = crate::oss_ast_analyzer::ApiSurface {
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            functions: Vec::new(),
        };
        for s in oss_surfaces {
            combined_oss_surface.structs.extend(s.structs);
            combined_oss_surface.enums.extend(s.enums);
            combined_oss_surface.traits.extend(s.traits);
            combined_oss_surface.functions.extend(s.functions);
        }

        // 3. Step 2b: Analyze AST (Local Context)
        let local_surface = if let Some(path) = local_surface_path {
            info!("🔍 [OssOrchestrator] Scanning local surface at {:?}", path);
            let surfaces = self.analyzer.analyze_directory(path)?;
            let mut combined = crate::oss_ast_analyzer::ApiSurface {
                structs: Vec::new(),
                enums: Vec::new(),
                traits: Vec::new(),
                functions: Vec::new(),
            };
            for s in surfaces {
                combined.structs.extend(s.structs);
                combined.enums.extend(s.enums);
                combined.traits.extend(s.traits);
                combined.functions.extend(s.functions);
            }
            combined
        } else {
            crate::oss_ast_analyzer::ApiSurface {
                structs: Vec::new(),
                enums: Vec::new(),
                traits: Vec::new(),
                functions: Vec::new(),
            }
        };

        send_progress("Step 3: Matching types with local surface".to_string(), 50).await;
        // 4. Step 3: Match Types
        let mismatches = self.matcher.compare(&local_surface, &combined_oss_surface);
        info!(
            "✅ [OssOrchestrator] Step 3: Detected {} mismatches",
            mismatches.len()
        );

        send_progress(
            format!(
                "Step 3b: Generating adapter for {} mismatches",
                mismatches.len()
            ),
            70,
        )
        .await;
        // 5. Step 3b: Generate Adapter
        let adapter_code = self
            .codegen
            .generate_adapter(
                &mismatches,
                "Expect traits/structs to match local standards",
                &format!("OSS API Surface: {:?}", combined_oss_surface),
            )
            .await?;
        info!("✅ [OssOrchestrator] Step 3b: Adapter code generated");

        send_progress("Step 4: Forging skill with AI Self-Heal".to_string(), 90).await;
        // 6. Step 4: Forge & Self-Heal
        let wasm_path = self
            .forge
            .forge_skill(
                target_name,
                &adapter_code,
                3, // Retry with AI Self-Heal
                &format!("Autonomous integration of OSS: {}", github_url),
                Some(self.llm.clone()),
            )
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Final forging failed: {}", e),
            })?;

        info!("🎯 [OssOrchestrator] Integration SUCCESS: {:?}", wasm_path);
        Ok("Dummy OSS Analysis Result".to_string())
    }
}

#[async_trait]
impl TaskConductor for OssIntegrationOrchestrator {
    fn conductor_name(&self) -> &str {
        "OssIntegrationConductor"
    }

    fn capable_categories(&self) -> Vec<String> {
        vec!["oss_integration".to_string()]
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: tokio::sync::mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        // Extract parameters from job
        // In a real scenario, these would be properly parsed from job.karma_directives or topic
        let github_url = job
            .karma_directives
            .unwrap_or_else(|| "https://github.com/example/repo".into());
        let target_name = &job.topic;

        self.execute_integration(
            &github_url,
            target_name,
            None, // local_surface_path is omitted for remote autonomous job
            Some(progress_tx),
            Some(job.id),
        )
        .await
        .map(|s| (s, None))
    }
}
