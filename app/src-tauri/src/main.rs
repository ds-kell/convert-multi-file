//! ConvertApp desktop entry point: wires the conversion engine to the web UI.
//!
//! Start-up flow:
//! 1. Parse the command line ([`args::parse`]).
//! 2. `--register` / `--unregister`: update the Explorer menu and exit,
//!    without starting the UI.
//! 3. Otherwise start Tauri. The single-instance plugin makes later launches
//!    forward their arguments to this process instead of starting a new one;
//!    `--to` launches go to the quick-convert queue, others focus the main
//!    window.

// Release builds are GUI apps: do not open a console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;
mod commands;
mod dto;
mod menu;
mod quick;
mod window;

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use convertapp_core::Registry;
use tauri::AppHandle;

use crate::args::Launch;
use crate::commands::AppState;
use crate::quick::QuickQueue;

fn main() -> ExitCode {
    let registry = Arc::new(Registry::with_defaults());
    let cwd = std::env::current_dir().unwrap_or_default();

    match parse_or_default(std::env::args_os(), &cwd) {
        Launch::RegisterMenu => exit_with(menu::set_enabled(&registry, true), "registered"),
        Launch::UnregisterMenu => exit_with(menu::set_enabled(&registry, false), "unregistered"),
        launch => {
            run_app(registry, launch);
            ExitCode::SUCCESS
        }
    }
}

/// Starts the Tauri application for a `Window` or `Quick` launch.
fn run_app(registry: Arc<Registry>, launch: Launch) {
    tauri::Builder::default()
        // Must be the first plugin so a second instance exits as early as possible.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let launch = parse_or_default(argv, Path::new(&cwd));
            // This callback runs inside the main thread's window procedure.
            // Creating a window synchronously from there deadlocks on Windows,
            // so finish the work on the async runtime instead.
            let app = app.clone();
            tauri::async_runtime::spawn(async move { handle_launch(&app, launch) });
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { registry })
        .manage(QuickQueue::default())
        .setup(move |app| {
            handle_launch(app.handle(), launch);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::inspect_files,
            commands::convert_files,
            quick::take_quick_requests,
            menu::context_menu_enabled,
            menu::set_context_menu,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start ConvertApp");
}

/// Reacts to a launch of this process or of a forwarded second instance.
fn handle_launch(app: &AppHandle, launch: Launch) {
    match launch {
        Launch::Quick(request) => quick::enqueue(app, request),
        // Register flags are handled before Tauri starts; a forwarded one
        // (from a second instance) just brings the app to the front.
        Launch::Window | Launch::RegisterMenu | Launch::UnregisterMenu => {
            if let Err(err) = window::show_main(app) {
                eprintln!("cannot open the main window: {err}");
            }
        }
    }
}

/// Parses arguments; on invalid input (or `--help`), prints clap's message and
/// falls back to opening the main window, which suits a GUI app better than
/// exiting silently.
fn parse_or_default<I, T>(argv: I, cwd: &Path) -> Launch
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    args::parse(argv, cwd).unwrap_or_else(|err| {
        let _ = err.print();
        Launch::Window
    })
}

/// Prints the outcome of a `--register` / `--unregister` run.
fn exit_with(result: Result<(), String>, done: &str) -> ExitCode {
    match result {
        Ok(()) => {
            println!("Context menu {done}.");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
