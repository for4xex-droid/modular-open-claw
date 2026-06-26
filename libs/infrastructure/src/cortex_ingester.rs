/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use sha2::{Digest, Sha256};
use shared::security::SecurityPolicy;
use std::sync::{Arc, OnceLock};

static TITLE_REGEX: OnceLock<regex::Regex> = OnceLock::new();

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum SourceType {
    Web,
    Pdf,
    Manual,
    GitHub,
    Rss,
    Query,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Web => "web",
            SourceType::Pdf => "pdf",
            SourceType::Manual => "manual",
            SourceType::GitHub => "github",
            SourceType::Rss => "rss",
            SourceType::Query => "query",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "web" => Ok(SourceType::Web),
            "pdf" => Ok(SourceType::Pdf),
            "manual" => Ok(SourceType::Manual),
            "github" => Ok(SourceType::GitHub),
            "rss" => Ok(SourceType::Rss),
            "query" => Ok(SourceType::Query),
            _ => Err(format!("Unknown source type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CortexDocument {
    pub id: String,
    pub title: String,
    pub source_url: Option<String>,
    pub content_md: String,
    pub content_hash: String,
    pub source_type: SourceType,
    pub ingested_at: String,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub wiki_article_refs: Vec<String>,
}

#[derive(Clone)]
pub struct CortexIngester {
    llm_provider: Arc<dyn LlmProvider>,
    pool: DatabasePool,
    http_client: reqwest::Client,
}

impl CortexIngester {
    pub fn new(llm_provider: Arc<dyn LlmProvider>, pool: DatabasePool) -> Self {
        Self {
            llm_provider,
            pool,
            http_client: aiome_core::http::get_http_client().clone(),
        }
    }

    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn ingest_url(&self, url: &str) -> Result<CortexDocument, AiomeError> {
        let policy = SecurityPolicy::default();
        if policy.validate_url(url).await.is_err() {
            return Err(AiomeError::SecurityViolation {
                reason: "Invalid or restricted URL".to_string(),
            });
        }

        const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2MB

        let resp = self
            .http_client
            .get(url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AiomeError::NetworkError {
                reason: format!("Failed to fetch URL: {}", e),
            })?;

        if let Some(len) = resp.content_length() {
            if len > MAX_BODY_BYTES as u64 {
                return Err(AiomeError::Infrastructure {
                    reason: "File too large. Max 2MB allowed for Cortex Ingestion.".to_string(),
                });
            }
        }

        // Read body with enforced size limit (defends against missing/lying Content-Length)
        let bytes = resp.bytes().await.map_err(|e| AiomeError::NetworkError {
            reason: format!("Failed to read response body: {}", e),
        })?;
        if bytes.len() > MAX_BODY_BYTES {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Response body too large: {} bytes (max {})",
                    bytes.len(),
                    MAX_BODY_BYTES
                ),
            });
        }
        let html = String::from_utf8_lossy(&bytes).to_string();

        self.process_html_to_document(url, &html).await
    }

    #[tracing::instrument(skip(self), fields(url = %url))]
    pub async fn ingest_url_via_obscura(&self, url: &str) -> Result<CortexDocument, AiomeError> {
        let policy = SecurityPolicy::default();
        if policy.validate_url(url).await.is_err() {
            return Err(AiomeError::SecurityViolation {
                reason: "Invalid or restricted URL".to_string(),
            });
        }

        let manifest = crate::security::PermissionManifest {
            allow_shell_execution: true,
            allow_filesystem_write: false,
            allow_network: true,
            ..Default::default()
        };

        let output = crate::security::SafeCommandBuilder::new("obscura")
            .arg("--timeout")
            .arg("15")
            .arg("--output-format")
            .arg("html")
            .arg(url)
            .profile(aiome_core::security::SandboxProfile::BrowserAgent)
            .build(manifest)?
            .output()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to execute obscura: {}", e),
            })?;

        if !output.status.success() {
            let stderr_preview = String::from_utf8_lossy(&output.stderr);
            let stderr_safe = shared::strings::truncate_bytes_safely(&stderr_preview, 512);
            tracing::warn!(status = %output.status, stderr = %stderr_safe, "obscura process failed");
            return Err(AiomeError::Infrastructure {
                reason: format!("obscura failed with status: {}", output.status),
            });
        }

        // Guard against oversized output (4MB limit) to prevent binary response DoS
        const MAX_OBSCURA_OUTPUT: usize = 4 * 1024 * 1024;
        if output.stdout.len() > MAX_OBSCURA_OUTPUT {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "obscura output too large: {} bytes (max {})",
                    output.stdout.len(),
                    MAX_OBSCURA_OUTPUT
                ),
            });
        }

        let html = String::from_utf8_lossy(&output.stdout).to_string();
        self.process_html_to_document(url, &html).await
    }

    async fn process_html_to_document(
        &self,
        url: &str,
        html: &str,
    ) -> Result<CortexDocument, AiomeError> {
        // 🛡️ [GlassWorm Shield] Strip invisible unicode before processing
        let clean_html = shared::guardrails::strip_invisible_unicode(html).into_owned();

        if clean_html.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Extracted content is empty. This page might require javascript to render."
                    .to_string(),
            });
        }

        let raw_md = html2md::parse_html(&clean_html);

        // Take at most 8000 chars without cloning the full content
        let sample_for_llm: String = raw_md.chars().take(8000).collect();

        let title_regex = TITLE_REGEX.get_or_init(|| {
            regex::Regex::new(r"(?i)<title[^>]*>(.+?)</title>")
                .expect("Failed to compile HTML title regex") // allow-anti-pattern: static regex
        });
        let html_title = title_regex
            .captures(&clean_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Web Article".to_string());

        let prompt = format!(
            "Please generate a short JSON metadata block for this document. Keys: `title`, `summary` (1-2 sentences), `tags` (array of up to 5 keywords). Use the provided HTML Title if relevant. Provide ONLY valid JSON.\n\nHTML Title: {}\n\nContent Sample:\n{}",
            html_title, sample_for_llm
        );

        let llm_resp = self
            .llm_provider
            .complete(&prompt, Some("You are a helpful JSON extractor."))
            .await?;

        // Extract JSON
        let content_json = llm_resp
            .content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let metadata_val: serde_json::Value = match serde_json::from_str(content_json) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(raw = %content_json.chars().take(200).collect::<String>(), error = %e, "LLM returned unparseable JSON for metadata extraction; using defaults");
                serde_json::Value::default()
            }
        };

        let title = metadata_val
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&html_title)
            .to_string();
        let summary = metadata_val
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags: Vec<String> = metadata_val
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let doc = CortexDocument {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            source_url: Some(url.to_string()),
            content_md: raw_md.clone(),
            content_hash: Self::compute_hash(&raw_md),
            source_type: SourceType::Web,
            ingested_at: chrono::Utc::now().to_rfc3339(),
            tags,
            summary,
            wiki_article_refs: vec![],
        };

        self.save_document(&doc).await?;
        Ok(doc)
    }

    pub async fn ingest_text(
        &self,
        title: &str,
        content: &str,
    ) -> Result<CortexDocument, AiomeError> {
        // 🛡️ [GlassWorm Shield] Strip invisible unicode before processing
        let clean_title = shared::guardrails::strip_invisible_unicode(title).into_owned();
        let content_md = shared::guardrails::strip_invisible_unicode(content).into_owned();

        let doc = CortexDocument {
            id: uuid::Uuid::new_v4().to_string(),
            title: clean_title,
            source_url: None,
            content_md: content_md.clone(),
            content_hash: Self::compute_hash(&content_md),
            source_type: SourceType::Manual,
            ingested_at: chrono::Utc::now().to_rfc3339(),
            tags: vec![],
            summary: None,
            wiki_article_refs: vec![],
        };

        self.save_document(&doc).await?;
        Ok(doc)
    }

    pub async fn ingest_pdf(&self, data: &[u8], title: &str) -> Result<CortexDocument, AiomeError> {
        let manifest = crate::security::PermissionManifest {
            allow_shell_execution: true,
            allow_filesystem_write: false,
            allow_network: false,
            ..Default::default()
        };

        let mut cmd = crate::security::SafeCommandBuilder::new("pdftotext")
            .arg("-")
            .arg("-")
            .profile(aiome_core::security::SandboxProfile::Strict)
            .build(manifest)?;

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to spawn pdftotext: {}", e),
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(data)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to write to pdftotext stdin: {}", e),
                })?;
            let _ = stdin.shutdown().await;
        }

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Failed to extract text from PDF: failed to open stdout".to_string(),
            })?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Failed to extract text from PDF: failed to open stderr".to_string(),
            })?;

        let mut stdout_data = Vec::new();
        let mut stderr_data = Vec::new();

        let read_and_wait = async {
            use tokio::io::AsyncReadExt;
            let mut stdout_limit = stdout.take(10_485_760); // Max 10MB text
            let mut stderr_limit = stderr.take(65_536); // Max 64KB errors
            let (res_stdout, res_stderr, res_status) = tokio::join!(
                stdout_limit.read_to_end(&mut stdout_data),
                stderr_limit.read_to_end(&mut stderr_data),
                child.wait()
            );
            let status = res_status?;
            res_stdout?;
            res_stderr?;
            Ok::<std::process::ExitStatus, std::io::Error>(status)
        };

        let status = match tokio::time::timeout(std::time::Duration::from_secs(30), read_and_wait)
            .await
        {
            Ok(res) => res.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to extract text from PDF: {}", e),
            })?,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await; // Reap child to prevent zombie processes (PID leak)
                return Err(AiomeError::Infrastructure {
                    reason: "Failed to extract text from PDF: pdftotext extraction timed out (30s limit exceeded)".to_string(),
                });
            }
        };

        if !status.success() {
            let stderr_preview = String::from_utf8_lossy(&stderr_data);
            let stderr_safe = shared::strings::truncate_bytes_safely(&stderr_preview, 512);
            return Err(AiomeError::Infrastructure {
                reason: format!("Failed to extract text from PDF: pdftotext failed with exit status {} - stderr: {}", status, stderr_safe),
            });
        }

        let raw_text = String::from_utf8(stdout_data).map_err(|e| AiomeError::Infrastructure {
            reason: format!(
                "Failed to extract text from PDF: invalid UTF-8 output: {}",
                e
            ),
        })?;

        // 🛡️ [GlassWorm Shield] Strip invisible unicode before processing
        let text = shared::guardrails::strip_invisible_unicode(&raw_text).into_owned();

        let mut sample_for_llm = text.clone();
        if sample_for_llm.len() > 8000 {
            sample_for_llm.truncate(8000);
        }

        let prompt = format!(
            "Please generate a short JSON metadata block for this PDF document. Keys: `title` (infer from text if necessary as string), `summary` (1-2 sentences), `tags` (array of up to 5 keywords). Provide ONLY valid JSON.\n\nProvided Title: {}\n\nContent Sample:\n{}",
            title, sample_for_llm
        );

        let llm_resp = self
            .llm_provider
            .complete(&prompt, Some("You are a helpful JSON extractor."))
            .await?;

        // Extract JSON
        let content_json = llm_resp
            .content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let metadata_val: serde_json::Value =
            serde_json::from_str(content_json).unwrap_or_default();

        let final_title = metadata_val
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(title)
            .to_string();
        let summary = metadata_val
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags: Vec<String> = metadata_val
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let doc = CortexDocument {
            id: uuid::Uuid::new_v4().to_string(),
            title: final_title,
            source_url: None,
            content_md: text.clone(),
            content_hash: Self::compute_hash(&text),
            source_type: SourceType::Pdf,
            ingested_at: chrono::Utc::now().to_rfc3339(),
            tags,
            summary,
            wiki_article_refs: vec![],
        };

        self.save_document(&doc).await?;
        Ok(doc)
    }

    async fn save_document(&self, doc: &CortexDocument) -> Result<(), AiomeError> {
        let pool = self.pool.get_sqlite_pool_or_err()?; // we use sqlite primarily for the cortex db

        let tags_json = serde_json::to_string(&doc.tags).unwrap_or_default();
        let wiki_refs_json = serde_json::to_string(&doc.wiki_article_refs).unwrap_or_default();

        sqlx::query(
            "INSERT INTO cortex_documents (
                id, title, source_url, content_md, content_hash, source_type, tags, summary, wiki_article_refs, ingested_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&doc.id)
        .bind(&doc.title)
        .bind(&doc.source_url)
        .bind(&doc.content_md)
        .bind(&doc.content_hash)
        .bind(doc.source_type.as_str())
        .bind(&tags_json)
        .bind(&doc.summary)
        .bind(&wiki_refs_json)
        .bind(&doc.ingested_at)
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to insert cortex_document: {}", e),
        })?;

        // [Activity Log]
        let summary = format!("Ingested document: {}", doc.title);
        let detail_json = serde_json::json!({
            "source_type": doc.source_type.as_str(),
            "doc_id": doc.id,
            "url": doc.source_url
        })
        .to_string();
        let log_res = sqlx::query("INSERT INTO cortex_activity_log (event_type, summary, detail_json) VALUES ('ingest', ?, ?)")
            .bind(&summary)
            .bind(&detail_json)
            .execute(pool)
            .await;

        if let Err(e) = log_res {
            tracing::warn!("Failed to log ingest activity: {}", e);
        }

        Ok(())
    }

    pub async fn list_documents(&self, limit: i64) -> Result<Vec<CortexDocument>, AiomeError> {
        let pool = self.pool.get_sqlite_pool_or_err()?;

        let rows = sqlx::query(
            "SELECT id, title, source_url, content_md, content_hash, source_type, tags, summary, wiki_article_refs, ingested_at
             FROM cortex_documents ORDER BY ingested_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to query cortex_documents: {}", e),
        })?;

        let mut docs = Vec::new();
        for row in rows {
            use sqlx::Row;
            let id: String = row.try_get("id").unwrap_or_default();
            let title: String = row.try_get("title").unwrap_or_default();
            let source_url: Option<String> = row.try_get("source_url").unwrap_or_default();
            let content_md: String = row.try_get("content_md").unwrap_or_default();
            let content_hash: String = row.try_get("content_hash").unwrap_or_default();
            let source_type_str: String = row.try_get("source_type").unwrap_or_default();
            let tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
            let summary: Option<String> = row.try_get("summary").unwrap_or_default();
            let wiki_refs_json: String = row
                .try_get("wiki_article_refs")
                .unwrap_or_else(|_| "[]".to_string());
            let ingested_at: String = row.try_get("ingested_at").unwrap_or_default();

            let source_type = SourceType::from_str(&source_type_str).unwrap_or(SourceType::Manual);
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let wiki_article_refs: Vec<String> =
                serde_json::from_str(&wiki_refs_json).unwrap_or_default();

            docs.push(CortexDocument {
                id,
                title,
                source_url,
                content_md,
                content_hash,
                source_type,
                ingested_at,
                tags,
                summary,
                wiki_article_refs,
            });
        }
        Ok(docs)
    }

    pub async fn delete_document(&self, id: &str) -> Result<(), AiomeError> {
        let pool = self.pool.get_sqlite_pool_or_err()?;

        sqlx::query("DELETE FROM cortex_documents WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to delete cortex_document: {}", e),
            })?;

        Ok(())
    }
}
