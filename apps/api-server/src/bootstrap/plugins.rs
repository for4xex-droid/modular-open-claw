/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::*;

/// Plugin agent hooks を HookManager に追加する（register 後に呼ぶ）
pub fn attach_plugin_hooks(
    plugin_registry: &crate::plugin_loader::PluginRegistry,
    hook_manager: &infrastructure::security::hook_manager::HookManager,
) {
    for hook in plugin_registry.get_agent_hooks() {
        hook_manager.add_hook(hook);
    }
}

/// `NURTURE_IN_PROCESS=true` かつ `--features nurture` 時に NurturePlugin を登録する
#[cfg(feature = "nurture")]
pub async fn register_in_process_plugins(
    plugin_registry: &mut crate::plugin_loader::PluginRegistry,
    cancel_token: tokio_util::sync::CancellationToken,
    nurture_secret: &Option<String>,
    db: &DatabaseResult,
    core: &CoreServicesResult,
) -> anyhow::Result<()> {
    let in_process = std::env::var("NURTURE_IN_PROCESS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    if !in_process {
        return Ok(());
    }

    let nurture_secret = nurture_secret.clone().ok_or_else(|| {
        anyhow::anyhow!("NURTURE_INTERNAL_SECRET is required for in-process mode")
    })?;

    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").ok();
    let polar_webhook_secret = std::env::var("POLAR_WEBHOOK_SECRET").ok();

    // OP-088 P0: Desktop/Tauri が注入する。debug 固定鍵は廃止（偽成功・弱い DRM 防止）
    let drm_master_key = require_drm_master_key()?;

    let created = nurture_api::plugin::create_plugin(
        db.db_pool.clone(),
        db.system_agent_id,
        core.event_sender.clone(),
        db.job_queue.clone() as Arc<dyn aiome_core_contracts::traits::JobQueue>,
        cancel_token,
        nurture_secret,
        stripe_webhook_secret,
        polar_webhook_secret,
        core.auth_manager.clone(),
        drm_master_key,
    )
    .await?;

    // JWT 外 S2S（G9）。Plugin merge_routes / nurture_routes には載せない。
    plugin_registry.set_s2s_router(created.s2s_router);
    plugin_registry.register(created.plugin);
    tracing::info!(
        "🔌 [Plugin] Nurture registered in-process (NURTURE_IN_PROCESS=true, S2S /internal ready)"
    );
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
pub async fn register_in_process_plugins(
    _plugin_registry: &mut crate::plugin_loader::PluginRegistry,
    _cancel_token: tokio_util::sync::CancellationToken,
    _nurture_secret: &Option<String>,
    _db: &DatabaseResult,
    _core: &CoreServicesResult,
) -> anyhow::Result<()> {
    if std::env::var("NURTURE_IN_PROCESS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
    {
        tracing::warn!(
            "⚠️ [Plugin] NURTURE_IN_PROCESS=true but api-server was built without `--features nurture`. Skipping in-process registration."
        );
    }
    Ok(())
}
