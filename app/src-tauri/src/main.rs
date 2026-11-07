//! ConvertApp desktop entry point: wires the conversion engine to the web UI.

// Release builds are GUI apps: do not open a console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;

use std::sync::Arc;

use convertapp_core::Registry;

use crate::commands::AppState;

fn main() {
    let state = AppState {
        registry: Arc::new(Registry::with_defaults()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::inspect_files,
            commands::convert_files,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start ConvertApp");
}
