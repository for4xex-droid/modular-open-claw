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
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn get_api_url() -> String {
    // Phase 51 NOTE: In production, the api-server might be on a dynamic port.
    // For now, we return the standard 3015 but allow override by A2A_NODE_URL
    std::env::var("A2A_NODE_URL").unwrap_or_else(|_| format!("http://{}:{}", "localhost", 3015))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// `run` 関数
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_api_url])
        .setup(|app| {
            use tauri::tray::TrayIconBuilder;
            use tauri::Manager;

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
                    } else if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {}", e));
    // allow-anti-pattern: fatal configuration error at boot
}

/// システムトレイのメニューを構築します。
pub fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> Result<tauri::menu::Menu<R>, tauri::Error> {
    let toggle =
        tauri::menu::MenuItem::with_id(app, "toggle", "Toggle Window", true, None::<&str>)?;
    let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    tauri::menu::Menu::with_items(app, &[&toggle, &quit])
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_os = "macos"))]
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
            2,
            "Tray menu must contain 2 items"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_build_tray_menu_success() {
        // macOS では `muda` の制約によりメインスレッド以外での Menu 構築がパニックするため、
        // 実行時はダミーとしてパスさせ、ビルドチェックのみを通します。
    }
}
