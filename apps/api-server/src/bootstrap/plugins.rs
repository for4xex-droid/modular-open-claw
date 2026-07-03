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

    let drm_master_key = std::env::var("NURTURE_DRM_MASTER_KEY").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "dev_drm_master_key_1234567890".to_string()
        } else {
            tracing::error!(
                "🚨 [Plugin] NURTURE_DRM_MASTER_KEY must be set for in-process mode in release builds"
            );
            std::process::exit(1);
        }
    });

    let plugin = nurture_api::plugin::create_plugin(
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

    plugin_registry.register(plugin);
    info!("🔌 [Plugin] Nurture registered in-process (NURTURE_IN_PROCESS=true)");
    Ok(())
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
