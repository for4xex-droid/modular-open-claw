/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
//! # クレート固有のインデックス
//!
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::{Mutex, OnceLock};
use ts_rs::TS;

static SIDECAR_STATE: OnceLock<Mutex<SidecarState>> = OnceLock::new();

/// サイドカーが使用するデフォルトポート
const NURTURE_API_PORT: u16 = 3020;
const KEY_PROXY_PORT: u16 = 3017;
const API_SERVER_PORT: u16 = 3015;

struct SidecarState {
    api_server_child: Option<tauri_plugin_shell::process::CommandChild>,
    key_proxy_child: Option<tauri_plugin_shell::process::CommandChild>,
    nurture_child: Option<tauri_plugin_shell::process::CommandChild>,
    api_server_status: String,
    key_proxy_status: String,
    nurture_status: String,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self {
            api_server_child: None,
            key_proxy_child: None,
            nurture_child: None,
            api_server_status: "stopped".to_string(),
            key_proxy_status: "stopped".to_string(),
            nurture_status: "stopped".to_string(),
        }
    }
}

fn get_sidecar_state() -> &'static Mutex<SidecarState> {
    SIDECAR_STATE.get_or_init(|| Mutex::new(SidecarState::default()))
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn get_api_url() -> String {
    // Phase 51 NOTE: In production, the api-server might be on a dynamic port.
    // For now, we return the standard 3015 but allow override by A2A_NODE_URL
    std::env::var("A2A_NODE_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", API_SERVER_PORT))
}

/// サイドカーエンジンの稼働状態
#[derive(serde::Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SidecarStatus {
    /// api-server の状態
    pub api_server: String,
    /// key-proxy の状態
    pub key_proxy: String,
    /// nurture の状態
    pub nurture: String,
}

/// システムおよび環境のステータス情報
#[derive(serde::Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SystemInfo {
    /// 稼働OS
    pub os: String,
    /// Docker (または互換環境) の使用可否
    pub docker_available: bool,
    /// 合計メモリ量（MB）
    pub total_memory_mb: u64,
}

#[tauri::command]
fn get_data_dir() -> String {
    if let Ok(val) = std::env::var("AIOME_DATA_DIR") {
        if !val.is_empty() {
            return val;
        }
    }
    shared::app_data::AppDataResolver::new()
        .map(|r| r.root().to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command]
fn get_sidecar_status() -> Result<SidecarStatus, String> {
    let state = get_sidecar_state()
        .lock()
        .map_err(|e| format!("Failed to acquire sidecar state lock: {}", e))?;
    Ok(SidecarStatus {
        api_server: state.api_server_status.clone(),
        key_proxy: state.key_proxy_status.clone(),
        nurture: state.nurture_status.clone(),
    })
}

#[tauri::command]
fn restart_sidecar(app: tauri::AppHandle) -> Result<(), String> {
    stop_sidecars();
    start_sidecars(&app)
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let os = std::env::consts::OS.to_string();
    let docker_available = check_docker_available();

    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    let total_memory_mb = sys.total_memory() / 1024 / 1024;

    SystemInfo {
        os,
        docker_available,
        total_memory_mb,
    }
}

fn check_docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .is_ok()
        || std::process::Command::new("podman")
            .arg("--version")
            .output()
            .is_ok()
}

use tauri_plugin_shell::ShellExt;

fn start_sidecars(app: &tauri::AppHandle) -> Result<(), String> {
    let mut state = get_sidecar_state()
        .lock()
        .map_err(|e| format!("Failed to acquire sidecar state lock: {}", e))?;
    if state.api_server_status == "running" {
        return Ok(());
    }

    let data_dir = get_data_dir();
    let cell_id = std::env::var("CELL_ID").unwrap_or_else(|_| "desktop-0".to_string());
    let gemini_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let nurture_mode = resolve_nurture_mode();

    // ── セッションシークレット生成 ─────
    let nurture_secret =
        std::env::var("NURTURE_INTERNAL_SECRET").unwrap_or_else(|_| generate_session_secret());

    // ── nurture-api ────────────────────────
    let nurture_url = match &nurture_mode {
        NurtureMode::Local => {
            let nurture_db = format!("sqlite:{}/nurture.db", &data_dir);

            let nurture_sidecar = app
                .shell()
                .sidecar("nurture-api")
                .map_err(|e| e.to_string())?
                .env("DATABASE_URL", &nurture_db)
                .env("NURTURE_INTERNAL_SECRET", &nurture_secret)
                .env("NURTURE_BIND_ADDR", "127.0.0.1")
                .env("CELL_ID", &cell_id)
                .env(
                    "NURTURE_DRM_MASTER_KEY",
                    std::env::var("NURTURE_DRM_MASTER_KEY")
                        .unwrap_or_else(|_| "dev_drm_master_key_1234567890".to_string()),
                );

            let (_, nurture_child) = nurture_sidecar.spawn().map_err(|e| e.to_string())?;
            state.nurture_child = Some(nurture_child);
            state.nurture_status = "running".to_string();

            format!("http://localhost:{}", NURTURE_API_PORT)
        }
        NurtureMode::Cloud(url) => {
            state.nurture_status = "cloud".to_string();
            url.clone()
        }
        NurtureMode::Disabled => {
            state.nurture_status = "disabled".to_string();
            String::new()
        }
    };

    // api-server
    let mut api_sidecar = app
        .shell()
        .sidecar("api-server")
        .map_err(|e| e.to_string())?
        .env("AIOME_DATA_DIR", &data_dir)
        .env("CELL_ID", &cell_id)
        .env(
            "KEY_PROXY_URL",
            format!("http://localhost:{}", KEY_PROXY_PORT),
        )
        .env("PORT", API_SERVER_PORT.to_string());

    if !nurture_url.is_empty() {
        api_sidecar = api_sidecar
            .env("NURTURE_API_URL", &nurture_url)
            .env("NURTURE_INTERNAL_SECRET", &nurture_secret);
    }

    let (_, api_child) = api_sidecar.spawn().map_err(|e| e.to_string())?;
    state.api_server_child = Some(api_child);
    state.api_server_status = "running".to_string();

    // key-proxy
    let mut key_sidecar = app
        .shell()
        .sidecar("key-proxy")
        .map_err(|e| e.to_string())?
        .env("AIOME_DATA_DIR", &data_dir)
        .env("CELL_ID", &cell_id)
        .env("PORT", KEY_PROXY_PORT.to_string());

    if !gemini_key.is_empty() {
        key_sidecar = key_sidecar.env("GEMINI_API_KEY", &gemini_key);
    }

    let (_, key_child) = key_sidecar.spawn().map_err(|e| e.to_string())?;
    state.key_proxy_child = Some(key_child);
    state.key_proxy_status = "running".to_string();

    Ok(())
}

fn stop_sidecars() {
    let mut state = match get_sidecar_state().lock() {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Failed to acquire sidecar state lock during shutdown: {}",
                e
            );
            return;
        }
    };
    if let Some(child) = state.api_server_child.take() {
        let _ = child.kill();
    }
    state.api_server_status = "stopped".to_string();

    if let Some(child) = state.key_proxy_child.take() {
        let _ = child.kill();
    }
    state.key_proxy_status = "stopped".to_string();

    if let Some(child) = state.nurture_child.take() {
        let _ = child.kill();
    }
    state.nurture_status = "stopped".to_string();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// `run` 関数
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_api_url,
            get_data_dir,
            get_sidecar_status,
            restart_sidecar,
            get_system_info,
            get_nurture_status
        ])
        .setup(|app| {
            use tauri::tray::TrayIconBuilder;
            use tauri::Manager;

            #[cfg(feature = "sidecar-auto")]
            {
                let is_test = std::env::current_exe()
                    .map(|p| p.to_string_lossy().contains("/deps/"))
                    .unwrap_or(false);
                if !is_test {
                    let handle = app.handle().clone();
                    if let Err(e) = start_sidecars(&handle) {
                        eprintln!("Failed to start sidecars on boot: {}", e);
                    }
                }
            }

            let menu = build_tray_menu(app)?;

            // デフォルトのウィンドウアイコンを取得し、存在しない場合は空のアイコンではなくエラーを返す
            let icon = app.default_window_icon().cloned().ok_or_else(|| {
                tauri::Error::AssetNotFound("Default window icon not found".to_string())
            })?;

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app: &tauri::AppHandle<tauri::Wry>, event| {
                    if event.id.as_ref() == "toggle" {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible: bool = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    } else if event.id.as_ref() == "restart" {
                        let handle = app.clone();
                        let _ = restart_sidecar(handle);
                    } else if event.id.as_ref() == "open_data" {
                        let data_dir = get_data_dir();
                        use tauri_plugin_opener::OpenerExt;
                        let _ = app.opener().open_path(data_dir, None::<&str>);
                    } else if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("error while building tauri application: {}", e);
            std::process::exit(1);
        });

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            stop_sidecars();
        }
    });
}

/// システムトレイのメニューを構築します。
pub fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> Result<tauri::menu::Menu<R>, tauri::Error> {
    let toggle =
        tauri::menu::MenuItem::with_id(app, "toggle", "Toggle Window", true, None::<&str>)?;
    let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;

    let state = get_sidecar_state().lock().map_err(|e| {
        tauri::Error::Io(std::io::Error::other(format!(
            "Failed to acquire sidecar state lock: {}",
            e
        )))
    })?;
    let status_text = format!(
        "Engine: {} | Economy: {}",
        if state.api_server_status == "running" {
            "✓"
        } else {
            "✗"
        },
        match state.nurture_status.as_str() {
            "running" => "✓ Local",
            "cloud" => "☁ Cloud",
            "disabled" => "— Off",
            _ => "✗",
        }
    );
    let status = tauri::menu::MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;

    let restart =
        tauri::menu::MenuItem::with_id(app, "restart", "Restart Engine", true, None::<&str>)?;
    let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let open_data =
        tauri::menu::MenuItem::with_id(app, "open_data", "Open Data Dir", true, None::<&str>)?;
    let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    tauri::menu::Menu::with_items(
        app,
        &[
            &toggle,
            &separator1,
            &status,
            &restart,
            &separator2,
            &open_data,
            &quit,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_build_tray_menu_success() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let menu_res = build_tray_menu(&app);
        assert!(
            menu_res.is_ok(),
            "Tray menu should be constructed successfully"
        );
        let menu = menu_res.unwrap();
        assert_eq!(
            menu.items().unwrap().len(),
            7,
            "Tray menu must contain 7 items"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_build_tray_menu_success() {
        // macOS では `muda` の制約によりメインスレッド以外での Menu 構築がパニックするため、
        // 実行時はダミーとしてパスさせ、ビルドチェックのみを通します。
    }

    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_data_dir() {
        std::env::set_var("AIOME_DATA_DIR", "/tmp/custom-tauri-data");
        assert_eq!(get_data_dir(), "/tmp/custom-tauri-data");
        std::env::remove_var("AIOME_DATA_DIR");
    }

    #[test]
    fn test_get_sidecar_status_default() {
        let status = get_sidecar_status().unwrap();
        assert_eq!(status.api_server, "stopped");
        assert_eq!(status.key_proxy, "stopped");
    }

    #[test]
    fn test_get_system_info() {
        let info = get_system_info();
        assert!(!info.os.is_empty());
    }

    #[test]
    #[serial]
    fn test_nurture_mode_default_is_local() {
        std::env::remove_var("NURTURE_CLOUD_URL");
        std::env::remove_var("NURTURE_DISABLED");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Local));
    }

    #[test]
    #[serial]
    fn test_nurture_mode_cloud() {
        std::env::set_var("NURTURE_CLOUD_URL", "https://nurture.example.com");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Cloud(_)));
        std::env::remove_var("NURTURE_CLOUD_URL");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_disabled() {
        std::env::set_var("NURTURE_DISABLED", "true");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Disabled));
        std::env::remove_var("NURTURE_DISABLED");
    }

    #[test]
    fn test_generate_session_secret_length() {
        let secret = generate_session_secret();
        assert_eq!(secret.len(), 64);
    }

    #[test]
    fn test_generate_session_secret_uniqueness() {
        let secret1 = generate_session_secret();
        let secret2 = generate_session_secret();
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn test_sidecar_status_includes_nurture() {
        let status = get_sidecar_status().unwrap();
        assert_eq!(status.nurture, "stopped"); // デフォルトは stopped を期待
    }

    #[test]
    fn test_get_nurture_status_default() {
        let status = get_nurture_status().unwrap();
        assert_eq!(status.status, "stopped"); // デフォルトは stopped を期待
    }
}

// ── Nurture Implementation ────────────────────────
#[derive(Debug, PartialEq)]
enum NurtureMode {
    Local,
    Cloud(String),
    Disabled,
}

fn resolve_nurture_mode() -> NurtureMode {
    if let Ok(url) = std::env::var("NURTURE_CLOUD_URL") {
        if !url.is_empty() {
            return NurtureMode::Cloud(url);
        }
    }
    if std::env::var("NURTURE_DISABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
    {
        return NurtureMode::Disabled;
    }
    NurtureMode::Local
}

fn generate_session_secret() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .expect("Fatal: Failed to secure random bytes for session secret");
    hex::encode(bytes)
}

/// Nurture サイドカーのステータス情報
#[derive(serde::Serialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct NurtureStatus {
    /// Nurture operation mode: "local" | "cloud" | "disabled"
    pub mode: String,
    /// Current sidecar status
    pub status: String,
    /// Connection URL
    pub url: String,
}

#[tauri::command]
fn get_nurture_status() -> Result<NurtureStatus, String> {
    let state = get_sidecar_state()
        .lock()
        .map_err(|e| format!("Failed to acquire sidecar state lock: {}", e))?;
    let mode = match state.nurture_status.as_str() {
        "cloud" => "cloud",
        "disabled" => "disabled",
        _ => "local",
    };
    Ok(NurtureStatus {
        mode: mode.to_string(),
        status: state.nurture_status.clone(),
        url: if state.nurture_status == "running" {
            format!("http://localhost:{}", NURTURE_API_PORT)
        } else {
            String::new()
        },
    })
}

#[cfg(test)]
mod ts_export_tests {
    use super::*;

    #[test]
    fn export_bindings() {
        // ts-rs のエクスポートテスト
        // これにより cargo test 実行時に自動的に bindings ディレクトリへ型定義が出力されます
        <SidecarStatus as ts_rs::TS>::export().unwrap();
        <SystemInfo as ts_rs::TS>::export().unwrap();
        <NurtureStatus as ts_rs::TS>::export().unwrap();
    }
}
