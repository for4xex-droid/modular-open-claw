/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use minijinja::Environment;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[async_trait]
pub trait PromptRegistry: Send + Sync {
    /// Render a prompt template with the given context.
    /// Follows the override hierarchy: Base -> Extension -> Preset
    async fn render(&self, template_name: &str, context: Value) -> Result<String, AiomeError>;

    /// Reload the templates from the filesystem
    async fn reload(&self) -> Result<(), AiomeError>;
}

pub struct MinijinjaPromptRegistry {
    env: RwLock<Environment<'static>>,
    base_dir: PathBuf,
}

impl MinijinjaPromptRegistry {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self, AiomeError> {
        let registry = Self {
            env: RwLock::new(Environment::new()),
            base_dir: base_dir.as_ref().to_path_buf(),
        };
        // Blocking reload during initialization is acceptable
        futures::executor::block_on(registry.reload())?;
        Ok(registry)
    }

    fn load_directory(
        base_dir: &Path,
        sub_dir: &str,
        env: &mut Environment<'static>,
    ) -> Result<(), AiomeError> {
        let target_dir = base_dir.join(sub_dir);
        if !target_dir.exists() || !target_dir.is_dir() {
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&target_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file()
                && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
            {
                let content = std::fs::read_to_string(entry.path()).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to read template: {}", e),
                    }
                })?;

                let rel_path = entry.path().strip_prefix(&target_dir).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to strip prefix: {}", e),
                    }
                })?;
                let template_name = rel_path.to_string_lossy().to_string();

                // Add or override template
                env.add_template_owned(template_name, content)
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Minijinja template error: {}", e),
                    })?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl PromptRegistry for MinijinjaPromptRegistry {
    async fn render(&self, template_name: &str, context: Value) -> Result<String, AiomeError> {
        let env = self.env.read().map_err(|_| AiomeError::Infrastructure {
            reason: "Failed to acquire read lock on PromptRegistry".into(),
        })?;

        let tmpl = env
            .get_template(template_name)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Template '{}' not found: {}", template_name, e),
            })?;

        tmpl.render(context)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to render template '{}': {}", template_name, e),
            })
    }

    async fn reload(&self) -> Result<(), AiomeError> {
        let mut new_env = Environment::new();

        // Hierarchy: Base -> Extension -> Preset
        Self::load_directory(&self.base_dir, "base", &mut new_env)?;
        Self::load_directory(&self.base_dir, "extensions", &mut new_env)?;
        Self::load_directory(&self.base_dir, "presets", &mut new_env)?;

        let mut env_lock = self.env.write().map_err(|_| AiomeError::Infrastructure {
            reason: "Failed to acquire write lock on PromptRegistry".into(),
        })?;
        *env_lock = new_env;

        Ok(())
    }
}

/// A no-op implementation of PromptRegistry used as a production fallback
/// when the Minijinja-based registry fails to initialize.
/// Returns the serialized context JSON, allowing callers to degrade gracefully.
pub struct NoopPromptRegistry;

#[async_trait]
impl PromptRegistry for NoopPromptRegistry {
    async fn render(&self, _template_name: &str, context: Value) -> Result<String, AiomeError> {
        // Return serialized context so callers can still function
        Ok(context.to_string())
    }

    async fn reload(&self) -> Result<(), AiomeError> {
        Ok(())
    }
}

/// Backward-compatible alias for test code.
/// Re-exports NoopPromptRegistry under its former name for test ergonomics.
#[cfg(any(test, debug_assertions))]
pub use NoopPromptRegistry as MockPromptRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_render_hierarchy_override() {
        let dir = tempdir().unwrap();

        // Setup Base
        let base_dir = dir.path().join("base");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(base_dir.join("test.md"), "Base: {{ name }}").unwrap();

        // Setup Extensions
        let ext_dir = dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();
        // Extensions should override Base
        fs::write(ext_dir.join("test.md"), "Extension: {{ name }}").unwrap();

        let registry = MinijinjaPromptRegistry::new(dir.path()).unwrap();

        let result = registry
            .render("test.md", serde_json::json!({"name": "Aiome"}))
            .await
            .unwrap();
        assert_eq!(result, "Extension: Aiome");

        // Setup Preset (highest priority)
        let preset_dir = dir.path().join("presets");
        fs::create_dir_all(&preset_dir).unwrap();
        fs::write(preset_dir.join("test.md"), "Preset: {{ name }}").unwrap();

        registry.reload().await.unwrap();
        let result = registry
            .render("test.md", serde_json::json!({"name": "Aiome"}))
            .await
            .unwrap();
        assert_eq!(result, "Preset: Aiome");
    }
}
