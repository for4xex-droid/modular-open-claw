/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_api_url])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {}", e));
}
