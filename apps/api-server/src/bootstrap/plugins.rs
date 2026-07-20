/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::*;
use std::sync::Arc;

/// Plugin agent hooks を HookManager に追加する（register 後に呼ぶ）
pub fn attach_plugin_hooks(
    plugin_registry: &crate::plugin_loader::PluginRegistry,
    hook_manager: &infrastructure::security::hook_manager::HookManager,
) {
    for hook in plugin_registry.get_agent_hooks() {
        hook_manager.add_hook(hook);
    }
}

fn nurture_in_process_enabled() -> bool {
    std::env::var("NURTURE_IN_PROCESS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

/// OP-088 P5-c C1': auth 後・Factory 前に Plugin を登録し Bridge を返す。
#[cfg(feature = "nurture")]
pub async fn try_register_in_process_commerce(
    plugin_registry: &mut crate::plugin_loader::PluginRegistry,
    cancel_token: tokio_util::sync::CancellationToken,
    nurture_secret: &Option<String>,
    db: &DatabaseResult,
    event_sender: tokio::sync::broadcast::Sender<aiome_core_contracts::events::CoreEvent>,
    auth_manager: Arc<dyn infrastructure::auth::AuthManager>,
) -> anyhow::Result<Option<Arc<dyn aiome_core_contracts::commerce::CommerceEngine>>> {
    if !nurture_in_process_enabled() {
        return Ok(None);
    }
    if plugin_registry.has_s2s_router() {
        return Ok(plugin_registry.commerce_engine());
    }

    let nurture_secret = nurture_secret.clone().ok_or_else(|| {
        anyhow::anyhow!("NURTURE_INTERNAL_SECRET is required for in-process mode")
    })?;
    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").ok();
    let polar_webhook_secret = std::env::var("POLAR_WEBHOOK_SECRET").ok();
    let drm_master_key = require_drm_master_key()?;

    let created = nurture_api::plugin::create_plugin(
        db.db_pool.clone(),
        db.system_agent_id,
        event_sender,
        db.job_queue.clone() as Arc<dyn aiome_core_contracts::traits::JobQueue>,
        cancel_token,
        nurture_secret,
        stripe_webhook_secret,
        polar_webhook_secret,
        auth_manager,
        drm_master_key,
    )
    .await?;

    let engine = created
        .plugin
        .commerce_engine()
        .ok_or_else(|| anyhow::anyhow!("NurturePlugin did not expose commerce_engine (ADR-013)"))?;
    plugin_registry.set_s2s_router(created.s2s_router);
    plugin_registry.register(created.plugin);
    tracing::info!(
        "🔌 [Plugin] Nurture registered in-process via C1' (Bridge is sole CommerceEngine)"
    );
    Ok(Some(engine))
}

/// `NURTURE_IN_PROCESS=true` かつ `--features nurture` 時に NurturePlugin を登録する。
/// P5-c: C1' で登録済みなら二重 create 禁止（no-op）。
#[cfg(feature = "nurture")]
pub async fn register_in_process_plugins(
    plugin_registry: &mut crate::plugin_loader::PluginRegistry,
    cancel_token: tokio_util::sync::CancellationToken,
    nurture_secret: &Option<String>,
    db: &DatabaseResult,
    core: &CoreServicesResult,
) -> anyhow::Result<()> {
    if !nurture_in_process_enabled() {
        return Ok(());
    }
    if plugin_registry.has_s2s_router() {
        tracing::info!("🔌 [Plugin] Nurture already registered (C1'); skipping duplicate create");
        return Ok(());
    }

    let _ = try_register_in_process_commerce(
        plugin_registry,
        cancel_token,
        nurture_secret,
        db,
        core.event_sender.clone(),
        core.auth_manager.clone(),
    )
    .await?;
    Ok(())
}

/// Fail-Closed: empty/missing DRM key must not fall back to a hardcoded debug key.
#[cfg(feature = "nurture")]
fn require_drm_master_key() -> anyhow::Result<String> {
    std::env::var("NURTURE_DRM_MASTER_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "NURTURE_DRM_MASTER_KEY is required for in-process mode (inject via Tauri or env)"
            )
        })
}

#[cfg(all(test, feature = "nurture"))]
mod drm_tests {
    use super::require_drm_master_key;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_require_drm_master_key_ok() {
        std::env::set_var("NURTURE_DRM_MASTER_KEY", "injected-drm");
        let Ok(key) = require_drm_master_key() else {
            panic!("expected Ok when NURTURE_DRM_MASTER_KEY is set");
        };
        assert_eq!(key, "injected-drm");
        std::env::remove_var("NURTURE_DRM_MASTER_KEY");
    }

    #[test]
    #[serial]
    fn test_require_drm_master_key_missing_fails() {
        std::env::remove_var("NURTURE_DRM_MASTER_KEY");
        let Err(err) = require_drm_master_key() else {
            panic!("expected Err when NURTURE_DRM_MASTER_KEY is missing");
        };
        let err = err.to_string();
        assert!(
            err.contains("NURTURE_DRM_MASTER_KEY"),
            "expected Fail-Closed error, got: {err}"
        );
    }

    #[test]
    #[serial]
    fn test_require_drm_master_key_empty_fails() {
        std::env::set_var("NURTURE_DRM_MASTER_KEY", "");
        let Err(err) = require_drm_master_key() else {
            panic!("expected Err when NURTURE_DRM_MASTER_KEY is empty");
        };
        assert!(err.to_string().contains("NURTURE_DRM_MASTER_KEY"));
        std::env::remove_var("NURTURE_DRM_MASTER_KEY");
    }
}

#[cfg(not(feature = "nurture"))]
pub async fn try_register_in_process_commerce(
    _plugin_registry: &mut crate::plugin_loader::PluginRegistry,
    _cancel_token: tokio_util::sync::CancellationToken,
    _nurture_secret: &Option<String>,
    _db: &DatabaseResult,
    _event_sender: tokio::sync::broadcast::Sender<aiome_core_contracts::events::CoreEvent>,
    _auth_manager: Arc<dyn infrastructure::auth::AuthManager>,
) -> anyhow::Result<Option<Arc<dyn aiome_core_contracts::commerce::CommerceEngine>>> {
    if nurture_in_process_enabled() {
        tracing::warn!(
            "⚠️ [Plugin] NURTURE_IN_PROCESS=true but api-server was built without `--features nurture`. Falling back to CommerceEngineFactory."
        );
    }
    Ok(None)
}

#[cfg(not(feature = "nurture"))]
pub async fn register_in_process_plugins(
    _plugin_registry: &mut crate::plugin_loader::PluginRegistry,
    _cancel_token: tokio_util::sync::CancellationToken,
    _nurture_secret: &Option<String>,
    _db: &DatabaseResult,
    _core: &CoreServicesResult,
) -> anyhow::Result<()> {
    if nurture_in_process_enabled() {
        tracing::warn!(
            "⚠️ [Plugin] NURTURE_IN_PROCESS=true but api-server was built without `--features nurture`. Skipping in-process registration."
        );
    }
    Ok(())
}
