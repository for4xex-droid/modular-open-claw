/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait SpecProvider: Send + Sync {
    /// Export the internal workflows and constraints to a spec-kit compatible format
    async fn export_to_spec_kit(&self, target_dir: &str) -> Result<(), AiomeError>;
}

pub struct FsSpecProvider {
    source_dir: PathBuf,
}

impl FsSpecProvider {
    pub fn new(source_dir: impl AsRef<Path>) -> Self {
        Self {
            source_dir: source_dir.as_ref().to_path_buf(),
        }
    }

    /// Sanitize content by redacting secret-like patterns.
    /// This is a best-effort defense-in-depth measure; source files
    /// should not contain real secrets in the first place.
    fn sanitize_content(content: &str) -> String {
        let redactor = crate::security::secret_redactor::SecretRedactor::new();
        redactor.redact(content).into_owned()
    }

    /// Validate that `target_path` resolves to a location under `allowed_parent`.
    /// Prevents path traversal attacks (e.g. `../../etc/cron.d`).
    fn validate_export_path(target_path: &Path, allowed_parent: &Path) -> Result<(), AiomeError> {
        // Create the directory first so canonicalize works on the resolved path
        std::fs::create_dir_all(target_path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create export dir: {}", e),
        })?;

        let canonical_target =
            target_path
                .canonicalize()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to canonicalize export path: {}", e),
                })?;

        let canonical_parent =
            allowed_parent
                .canonicalize()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to canonicalize parent path: {}", e),
                })?;

        if !canonical_target.starts_with(&canonical_parent) {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Export path '{}' escapes allowed boundary '{}'",
                    canonical_target.display(),
                    canonical_parent.display()
                ),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl SpecProvider for FsSpecProvider {
    async fn export_to_spec_kit(&self, target_dir: &str) -> Result<(), AiomeError> {
        let target_root = Path::new(target_dir);
        let out_dir = target_root.join(".specify").join("templates");

        // C1 fix: Validate export path is under target_dir (no traversal)
        // We use target_dir's parent as the allowed boundary since we create
        // the .specify subdirectory ourselves.
        if let Some(allowed_parent) = target_root.parent() {
            if allowed_parent.exists() {
                Self::validate_export_path(&out_dir, allowed_parent)?;
            } else {
                // Parent doesn't exist yet — create and validate
                std::fs::create_dir_all(allowed_parent).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to create parent dir: {}", e),
                    }
                })?;
                Self::validate_export_path(&out_dir, allowed_parent)?;
            }
        } else {
            std::fs::create_dir_all(&out_dir).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create export dir: {}", e),
            })?;
        }

        if !self.source_dir.exists() || !self.source_dir.is_dir() {
            return Ok(());
        }

        // C3 fix: Explicitly disable symlink following to prevent sandbox escape
        let walker = walkdir::WalkDir::new(&self.source_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            // Skip symlinks entirely as a defense-in-depth measure
            if entry.path_is_symlink() {
                tracing::warn!(
                    "Skipping symlink during spec export: {}",
                    entry.path().display()
                );
                continue;
            }

            if entry.path().is_file()
                && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
            {
                // C6 fix: Use tokio::fs for async I/O instead of blocking std::fs
                let content = tokio::fs::read_to_string(entry.path()).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to read {}: {}", entry.path().display(), e),
                    }
                })?;

                // C2 fix: Regex-based secret redaction instead of hardcoded string match
                let sanitized = Self::sanitize_content(&content);

                let rel_path = entry.path().strip_prefix(&self.source_dir).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;

                let dest_path = out_dir.join(rel_path);
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: format!(
                                "Failed to create parent dir {}: {}",
                                parent.display(),
                                e
                            ),
                        }
                    })?;
                }

                let exported = format!("---\nsource: aiome\nspec_version: 1.0\n---\n{}", sanitized);

                tokio::fs::write(&dest_path, exported).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to write {}: {}", dest_path.display(), e),
                    }
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_export_spec_kit_basic() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();

        // Create a mock workflow with a secret pattern
        std::fs::write(
            src_dir.path().join("workflow.md"),
            "Setup:\nAPI_KEY=my_secret_12345\nTOKEN: bearer_abc\nNormal text here.", // gitleaks:allow
        )
        .unwrap();

        let provider = FsSpecProvider::new(src_dir.path());
        provider
            .export_to_spec_kit(out_dir.path().to_str().unwrap())
            .await
            .unwrap();

        let exported_path = out_dir
            .path()
            .join(".specify")
            .join("templates")
            .join("workflow.md");
        assert!(exported_path.exists());

        let content = std::fs::read_to_string(exported_path).unwrap();
        assert!(content.contains("source: aiome"));
        assert!(content.contains("REDACTED"), "Secrets should be redacted");
        assert!(
            !content.contains("my_secret_12345"),
            "Raw secret value must not appear"
        );
        assert!(
            !content.contains("bearer_abc"),
            "Raw token value must not appear"
        );
        assert!(
            content.contains("Normal text here"),
            "Non-secret text should be preserved"
        );
    }

    #[tokio::test]
    async fn test_sanitize_content_patterns() {
        let input = "PASSWORD=hunter2\nCLIENT_SECRET: abcdef\nACCESS_KEY=AKIA1234\nSafe line.";
        let result = FsSpecProvider::sanitize_content(input);
        assert!(!result.contains("hunter2"));
        assert!(!result.contains("abcdef"));
        assert!(!result.contains("AKIA1234"));
        assert!(result.contains("Safe line."));
        // Verify they are replaced with REDACTED
        assert!(result.contains("[REDACTED]"));
    }

    #[tokio::test]
    async fn test_export_empty_source_dir() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();

        // No files in source — should succeed silently
        let provider = FsSpecProvider::new(src_dir.path());
        provider
            .export_to_spec_kit(out_dir.path().to_str().unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_export_nonexistent_source_dir() {
        let out_dir = tempdir().unwrap();
        let fake_source = out_dir.path().join("nonexistent");

        let provider = FsSpecProvider::new(&fake_source);
        // Should succeed — no files to export
        provider
            .export_to_spec_kit(out_dir.path().to_str().unwrap())
            .await
            .unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_export_skips_symlinks() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let external_dir = tempdir().unwrap();

        // Create an external file that should NOT be exported
        std::fs::write(external_dir.path().join("secret.md"), "TOP_SECRET=data").unwrap();

        // Create a symlink inside src_dir pointing outside
        std::os::unix::fs::symlink(
            external_dir.path().join("secret.md"),
            src_dir.path().join("escape.md"),
        )
        .unwrap();

        // Also create a legitimate file
        std::fs::write(src_dir.path().join("legit.md"), "Normal content").unwrap();

        let provider = FsSpecProvider::new(src_dir.path());
        provider
            .export_to_spec_kit(out_dir.path().to_str().unwrap())
            .await
            .unwrap();

        let templates_dir = out_dir.path().join(".specify").join("templates");

        // Symlinked file should NOT be exported
        assert!(
            !templates_dir.join("escape.md").exists(),
            "Symlinked files must not be exported"
        );

        // Legitimate file should be exported
        assert!(
            templates_dir.join("legit.md").exists(),
            "Regular files should be exported"
        );
    }
}
