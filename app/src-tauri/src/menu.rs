//! Turning the Explorer context menu on and off, from the UI or the command
//! line (`--register` / `--unregister`).

use convertapp_core::{Registry, error_chain};
use convertapp_shell::ContextMenu;
use tauri::State;

use crate::commands::AppState;

/// Context-menu integration pointing at the running executable.
fn context_menu() -> Result<ContextMenu, String> {
    let exe = std::env::current_exe().map_err(|e| error_chain(&e))?;
    Ok(ContextMenu::new(exe))
}

/// Registers or removes the menu. Shared by the command and the CLI flags.
pub fn set_enabled(registry: &Registry, enabled: bool) -> Result<(), String> {
    let menu = context_menu()?;
    if enabled {
        menu.register(registry).map(|_| ())
    } else {
        menu.unregister()
    }
    .map_err(|e| error_chain(&e))
}

/// Whether the "Convert to" entry is currently in the Explorer menu.
#[tauri::command]
pub async fn context_menu_enabled() -> Result<bool, String> {
    context_menu()?.is_registered().map_err(|e| error_chain(&e))
}

/// Adds or removes the "Convert to" entry. Returns the new state.
#[tauri::command]
pub async fn set_context_menu(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    set_enabled(&state.registry, enabled)?;
    Ok(enabled)
}
