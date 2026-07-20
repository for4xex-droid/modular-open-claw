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

/// api-server 子プロセスへ渡す InProcess 用 env（自己 HTTP: Q1=A'）。
fn in_process_api_env(secret: &str, drm_key: &str) -> [(&'static str, String); 4] {
    [
        ("NURTURE_IN_PROCESS", "true".to_string()),
        ("NURTURE_INTERNAL_SECRET", secret.to_string()),
        ("NURTURE_DRM_MASTER_KEY", drm_key.to_string()),
        // 沈黙 skip 禁止（P1-2）。/internal は同一プロセスの JWT 外 nest。
        (
            "NURTURE_API_URL",
            format!("http://127.0.0.1:{}", API_SERVER_PORT),
        ),
    ]
}

fn resolve_drm_master_key(data_dir: &str) -> Result<String, String> {
    if let Ok(key) = std::env::var("NURTURE_DRM_MASTER_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    let key_path = format!("{data_dir}/.nurture_drm_master_key");
    if let Ok(existing) = std::fs::read_to_string(&key_path) {
        let trimmed = existing.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    #[cfg(not(debug_assertions))]
    {
        return Err(
            "NURTURE_DRM_MASTER_KEY is not set and no persisted desktop key was found".to_string(),
        );
    }

    #[cfg(debug_assertions)]
    {
        let generated = generate_session_secret();
        std::fs::write(&key_path, &generated)
            .map_err(|e| format!("Failed to persist DRM master key at {key_path}: {e}"))?;
        Ok(generated)
    }
}

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

    // ── nurture-api（Local のみ spawn。InProcess は ADR-012 により非起動）──
    let nurture_url = match &nurture_mode {
        NurtureMode::Local => {
            // 公式パッケージは nurture-api 非同梱（OP-088 P3）。dev は --with-nurture-sidecar
            let nurture_db = format!("sqlite:{}/nurture.db", &data_dir);

            let drm_master_key = resolve_drm_master_key(&data_dir)?;

            let nurture_sidecar = app
                .shell()
                .sidecar("nurture-api")
                .map_err(|e| {
                    format!(
                        "nurture-api sidecar unavailable (official package excludes it). \
                         Dev escape: --with-nurture-sidecar + NURTURE_MODE=local + temporarily add \
                         binaries/nurture-api to tauri.conf externalBin and capabilities. \
                         Detail: {e}"
                    )
                })?
                .env("DATABASE_URL", &nurture_db)
                .env("NURTURE_INTERNAL_SECRET", &nurture_secret)
                .env("NURTURE_BIND_ADDR", "127.0.0.1")
                .env("CELL_ID", &cell_id)
                .env("NURTURE_DRM_MASTER_KEY", &drm_master_key);

            let (_, nurture_child) = nurture_sidecar.spawn().map_err(|e| e.to_string())?;
            state.nurture_child = Some(nurture_child);
            state.nurture_status = "running".to_string();

            format!("http://localhost:{}", NURTURE_API_PORT)
        }
        NurtureMode::Cloud(url) => {
            state.nurture_status = "cloud".to_string();
            url.clone()
        }
        NurtureMode::InProcess => {
            // nurture-api 非 spawn。api-server 側 plugins.rs が Hook/MCP を担当
            state.nurture_child = None;
            state.nurture_status = "in_process".to_string();
            warn_if_nurture_sidecar_port_alive();
            String::new()
        }
        NurtureMode::Disabled => {
            state.nurture_status = "disabled".to_string();
            String::new()
        }
    };

    // InProcess: Local と同じ DRM persist を api-server に渡す（OP-088 P0）
    let in_process_drm = if matches!(nurture_mode, NurtureMode::InProcess) {
        Some(resolve_drm_master_key(&data_dir)?)
    } else {
        None
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

    if let Some(drm_key) = in_process_drm.as_deref() {
        // self-URL + secret/DRM（G1/G12）。sidecar Local URL とは排他（InProcess は非 spawn）
        for (key, value) in in_process_api_env(&nurture_secret, drm_key) {
            api_sidecar = api_sidecar.env(key, value);
        }
    } else {
        // 親シェルに NURTURE_IN_PROCESS=true が残っていても Local/Cloud を汚染しない
        api_sidecar = api_sidecar.env("NURTURE_IN_PROCESS", "false");
        if !nurture_url.is_empty() {
            api_sidecar = api_sidecar
                .env("NURTURE_API_URL", &nurture_url)
                .env("NURTURE_INTERNAL_SECRET", &nurture_secret);
        }
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
            get_nurture_status,
            set_nurture_mode
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
            "running" => "✓ Local (dev)",
            "cloud" => "☁ Cloud",
            "in_process" => "⚡ InProcess (default)",
            "disabled" => "— Off",
            _ => "✗",
        }
    );
    let status = tauri::menu::MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
    let hint = tauri::menu::MenuItem::with_id(
        app,
        "economy_hint",
        "Economy: no config needed (NURTURE_MODE=local for sidecar)",
        false,
        None::<&str>,
    )?;

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
            &hint,
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

    fn clear_nurture_mode_env() {
        std::env::remove_var("NURTURE_MODE");
        std::env::remove_var("NURTURE_CLOUD_URL");
        std::env::remove_var("NURTURE_DISABLED");
        std::env::remove_var("NURTURE_IN_PROCESS");
        // Avoid picking up a developer machine's persisted .nurture_mode
        let dir = std::env::temp_dir().join("aiome-nurture-mode-test-empty");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join(".nurture_mode"));
        std::env::set_var("AIOME_DATA_DIR", &dir);
    }

    #[test]
    #[serial]
    fn test_nurture_mode_file_when_env_unset() {
        clear_nurture_mode_env();
        let dir = std::env::temp_dir().join(format!(
            "aiome-nurture-mode-file-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".nurture_mode"), "local\n").unwrap();
        assert!(matches!(
            resolve_nurture_mode_from(dir.to_str().unwrap()),
            NurtureMode::Local
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn test_nurture_mode_env_beats_file() {
        clear_nurture_mode_env();
        let dir = std::env::temp_dir().join(format!(
            "aiome-nurture-mode-envbeat-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".nurture_mode"), "local\n").unwrap();
        std::env::set_var("NURTURE_MODE", "disabled");
        assert!(matches!(
            resolve_nurture_mode_from(dir.to_str().unwrap()),
            NurtureMode::Disabled
        ));
        clear_nurture_mode_env();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn test_nurture_mode_default_is_in_process() {
        clear_nurture_mode_env();
        assert!(matches!(resolve_nurture_mode(), NurtureMode::InProcess));
    }

    #[test]
    #[serial]
    fn test_nurture_mode_local_via_mode() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_MODE", "local");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Local));
        std::env::remove_var("NURTURE_MODE");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_local_beats_cloud_url() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_MODE", "local");
        std::env::set_var("NURTURE_CLOUD_URL", "https://nurture.example.com");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Local));
        clear_nurture_mode_env();
    }

    #[test]
    #[serial]
    fn test_nurture_mode_cloud() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_CLOUD_URL", "https://nurture.example.com");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Cloud(_)));
        std::env::remove_var("NURTURE_CLOUD_URL");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_cloud_via_mode() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_MODE", "cloud");
        std::env::set_var("NURTURE_CLOUD_URL", "https://nurture.example.com");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Cloud(_)));
        clear_nurture_mode_env();
    }

    #[test]
    #[serial]
    fn test_nurture_mode_disabled() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_DISABLED", "true");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Disabled));
        std::env::remove_var("NURTURE_DISABLED");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_disabled_via_mode() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_MODE", "disabled");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Disabled));
        std::env::remove_var("NURTURE_MODE");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_in_process_explicit() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_IN_PROCESS", "true");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::InProcess));
        std::env::remove_var("NURTURE_IN_PROCESS");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_in_process_via_mode() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_MODE", "in_process");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::InProcess));
        std::env::remove_var("NURTURE_MODE");
    }

    #[test]
    #[serial]
    fn test_nurture_mode_disabled_beats_cloud() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_CLOUD_URL", "https://nurture.example.com");
        std::env::set_var("NURTURE_DISABLED", "1");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Disabled));
        clear_nurture_mode_env();
    }

    #[test]
    #[serial]
    fn test_nurture_mode_cloud_beats_in_process_legacy() {
        clear_nurture_mode_env();
        std::env::set_var("NURTURE_CLOUD_URL", "https://nurture.example.com");
        std::env::set_var("NURTURE_IN_PROCESS", "true");
        assert!(matches!(resolve_nurture_mode(), NurtureMode::Cloud(_)));
        clear_nurture_mode_env();
    }

    #[test]
    fn test_env_flag_truthy() {
        assert!(env_flag_truthy("true"));
        assert!(env_flag_truthy("1"));
        assert!(!env_flag_truthy("false"));
        assert!(!env_flag_truthy("0"));
        assert!(!env_flag_truthy(""));
    }

    #[test]
    fn test_in_process_api_env_injects_secret_drm_and_self_url() {
        let env = in_process_api_env("sess-secret", "drm-key-hex");
        assert_eq!(env[0], ("NURTURE_IN_PROCESS", "true".to_string()));
        assert_eq!(
            env[1],
            ("NURTURE_INTERNAL_SECRET", "sess-secret".to_string())
        );
        assert_eq!(
            env[2],
            ("NURTURE_DRM_MASTER_KEY", "drm-key-hex".to_string())
        );
        assert_eq!(
            env[3],
            (
                "NURTURE_API_URL",
                format!("http://127.0.0.1:{}", API_SERVER_PORT)
            )
        );
    }

    #[test]
    #[serial]
    fn test_resolve_drm_master_key_reads_env() {
        std::env::set_var("NURTURE_DRM_MASTER_KEY", "from-env-drm");
        let key = resolve_drm_master_key("/tmp/aiome-drm-unused").unwrap();
        assert_eq!(key, "from-env-drm");
        std::env::remove_var("NURTURE_DRM_MASTER_KEY");
    }

    #[test]
    #[serial]
    fn test_resolve_drm_master_key_persists_when_missing() {
        std::env::remove_var("NURTURE_DRM_MASTER_KEY");
        let dir = std::env::temp_dir().join(format!(
            "aiome-drm-p0-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let key1 = resolve_drm_master_key(dir.to_str().unwrap()).unwrap();
        assert!(!key1.is_empty());
        let key2 = resolve_drm_master_key(dir.to_str().unwrap()).unwrap();
        assert_eq!(key1, key2, "persisted DRM key must be stable across calls");
        let _ = std::fs::remove_dir_all(&dir);
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
        assert_eq!(status.status, "stopped");
        assert_eq!(status.mode, "stopped"); // 起動前を local と誤表示しない
    }
}

// ── Nurture Implementation ────────────────────────
#[derive(Debug, PartialEq)]
enum NurtureMode {
    Local,
    Cloud(String),
    InProcess,
    Disabled,
}

fn env_flag_truthy(raw: &str) -> bool {
    matches!(raw.trim().to_ascii_lowercase().as_str(), "true" | "1")
}

/// OP-088 P2/P5-b: `NURTURE_MODE` 正本。優先は env ≫ 旧変数 ≫ `{data_dir}/.nurture_mode` ≫ InProcess。
///
/// ```text
/// MODE=disabled / NURTURE_DISABLED     → Disabled
/// MODE=cloud / NURTURE_CLOUD_URL       → Cloud
/// MODE=local                           → Local（dev escape）
/// MODE=in_process / NURTURE_IN_PROCESS → InProcess
/// file .nurture_mode                   → 同上（env 未設定時）
/// else                                 → InProcess（製品既定）
/// ```
fn resolve_nurture_mode() -> NurtureMode {
    resolve_nurture_mode_from(&get_data_dir())
}

fn parse_nurture_mode_token(token: &str) -> Option<NurtureMode> {
    match token.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "disabled" => Some(NurtureMode::Disabled),
        "cloud" => match std::env::var("NURTURE_CLOUD_URL") {
            Ok(url) if !url.is_empty() => Some(NurtureMode::Cloud(url)),
            _ => {
                eprintln!("⚠️ [Nurture] cloud mode requires NURTURE_CLOUD_URL; using InProcess");
                Some(NurtureMode::InProcess)
            }
        },
        "local" => Some(NurtureMode::Local),
        "in_process" | "in-process" | "inprocess" => Some(NurtureMode::InProcess),
        other => {
            eprintln!("⚠️ [Nurture] Unknown nurture mode={other}; using InProcess default");
            Some(NurtureMode::InProcess)
        }
    }
}

fn resolve_nurture_mode_from(data_dir: &str) -> NurtureMode {
    let mode = std::env::var("NURTURE_MODE")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if !mode.is_empty() {
        return parse_nurture_mode_token(&mode).unwrap_or(NurtureMode::InProcess);
    }

    // Legacy when NURTURE_MODE unset
    if std::env::var("NURTURE_DISABLED")
        .map(|v| env_flag_truthy(&v))
        .unwrap_or(false)
    {
        return NurtureMode::Disabled;
    }
    if let Ok(url) = std::env::var("NURTURE_CLOUD_URL") {
        if !url.is_empty() {
            return NurtureMode::Cloud(url);
        }
    }
    if std::env::var("NURTURE_IN_PROCESS")
        .map(|v| env_flag_truthy(&v))
        .unwrap_or(false)
    {
        return NurtureMode::InProcess;
    }

    // OP-088 P5-b: persisted desktop preference
    if !data_dir.is_empty() {
        let path = format!("{data_dir}/.nurture_mode");
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Some(parsed) = parse_nurture_mode_token(raw.trim()) {
                return parsed;
            }
        }
    }

    // Product default (ADR-012 Amendment)
    NurtureMode::InProcess
}

/// P2-4: InProcess 中に :3020 が生きていれば二重 Hook の恐れを警告する。
fn warn_if_nurture_sidecar_port_alive() {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr = SocketAddr::from(([127, 0, 0, 1], NURTURE_API_PORT));
    if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
        eprintln!(
            "⚠️ [Nurture] Port {NURTURE_API_PORT} responds while InProcess — \
             nurture-api sidecar may cause double Hook (ADR-012). Stop the sidecar \
             or set NURTURE_MODE=local only when intentionally using it."
        );
    }
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
    /// Nurture operation mode: "local" | "cloud" | "in_process" | "disabled" | "stopped" | "unknown"
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
        "in_process" => "in_process",
        "running" => "local",
        "stopped" => "stopped",
        _ => "unknown",
    };
    Ok(NurtureStatus {
        mode: mode.to_string(),
        status: state.nurture_status.clone(),
        url: match state.nurture_status.as_str() {
            "running" => format!("http://localhost:{}", NURTURE_API_PORT),
            "in_process" => format!("http://127.0.0.1:{}", API_SERVER_PORT),
            _ => String::new(),
        },
    })
}

/// OP-088 P5-b: `{data_dir}/.nurture_mode` に永続化し、既存 `restart_sidecar` で再適用する。
#[tauri::command]
fn set_nurture_mode(app: tauri::AppHandle, mode: String) -> Result<NurtureStatus, String> {
    let normalized = mode.trim().to_ascii_lowercase();
    let allowed = matches!(
        normalized.as_str(),
        "disabled" | "cloud" | "local" | "in_process" | "in-process" | "inprocess"
    );
    if !allowed {
        return Err(format!(
            "invalid nurture mode '{mode}'; use disabled|cloud|local|in_process"
        ));
    }
    let canonical = match normalized.as_str() {
        "in-process" | "inprocess" => "in_process",
        other => other,
    };
    let data_dir = get_data_dir();
    if data_dir.is_empty() {
        return Err("data directory is unavailable".to_string());
    }
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data dir {data_dir}: {e}"))?;
    let path = format!("{data_dir}/.nurture_mode");
    std::fs::write(&path, format!("{canonical}\n"))
        .map_err(|e| format!("Failed to write {path}: {e}"))?;
    restart_sidecar(app)?;
    get_nurture_status()
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
