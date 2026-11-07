//! Turning the Explorer context menu on and off, from the UI or the command
//! line (`--register` / `--unregister`).

use convertapp_core::{Registry, error_chain};
use convertapp_shell::ContextMenu;
use serde::Deserialize;
use tauri::State;

use crate::commands::AppState;

/// UI language, as sent by the frontend (`"en"` / `"vi"`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// English.
    #[default]
    En,
    /// Vietnamese.
    Vi,
}

impl Language {
    /// The operating system's display language, used when no UI is involved
    /// (`--register` from the installer). Anything but Vietnamese is English.
    pub fn from_system() -> Self {
        match sys_locale::get_locale() {
            Some(locale) if locale.to_ascii_lowercase().starts_with("vi") => Language::Vi,
            _ => Language::En,
        }
    }

    /// Text of the cascading Explorer menu entry.
    fn menu_title(self) -> &'static str {
        match self {
            Language::En => "Convert to",
            Language::Vi => "Chuyển sang",
        }
    }
}

/// Context-menu integration pointing at the running executable.
fn context_menu(language: Language) -> Result<ContextMenu, String> {
    let exe = std::env::current_exe().map_err(|e| error_chain(&e))?;
    Ok(ContextMenu::new(exe).with_title(language.menu_title()))
}

/// Registers or removes the menu. Shared by the command and the CLI flags.
pub fn set_enabled(registry: &Registry, enabled: bool, language: Language) -> Result<(), String> {
    let menu = context_menu(language)?;
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
    context_menu(Language::default())?
        .is_registered()
        .map_err(|e| error_chain(&e))
}

/// Adds or removes the "Convert to" entry, titled in `language`. Calling it
/// again with another language re-registers with the new title. Returns the
/// new state.
#[tauri::command]
pub async fn set_context_menu(
    state: State<'_, AppState>,
    enabled: bool,
    language: Language,
) -> Result<bool, String> {
    set_enabled(&state.registry, enabled, language)?;
    Ok(enabled)
}
