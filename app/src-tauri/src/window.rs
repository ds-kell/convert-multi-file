//! Creating and focusing the app's windows.
//!
//! Windows are created in code (not in `tauri.conf.json`) because which one
//! opens depends on the command line: the main window normally, the small
//! quick-convert window when launched from the Explorer context menu.
//! The frontend picks its view from the window label.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Serialises "find or create" so concurrent launches forwarded by the
/// single-instance plugin cannot each see "no window yet" and build duplicate
/// windows with the same label.
///
/// No deadlock with the main thread: it only takes this lock in `setup`,
/// before the event loop runs, and forwarded launches are delivered by that
/// event loop, so they always come after `setup` has released it.
static CREATE_LOCK: Mutex<()> = Mutex::new(());

/// Label of the main window.
pub const MAIN_LABEL: &str = "main";
/// Label of the quick-convert window.
pub const QUICK_LABEL: &str = "quick";

/// Shows the main window, creating it if needed.
pub fn show_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    show_or_create(app, MAIN_LABEL, |builder| {
        builder
            .title("ConvertApp")
            .inner_size(900.0, 640.0)
            .min_inner_size(640.0, 480.0)
    })
}

/// Shows the quick-convert window, creating it if needed.
pub fn show_quick(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    show_or_create(app, QUICK_LABEL, |builder| {
        builder
            .title("ConvertApp – Chuyển đổi nhanh")
            .inner_size(480.0, 380.0)
            .min_inner_size(380.0, 260.0)
            // Files come from the context menu, not from drag & drop.
            .disable_drag_drop_handler()
    })
}

/// Focuses the window `label` if it exists, otherwise builds it with
/// `configure` applied to a centred, focused builder.
fn show_or_create<'a, F>(
    app: &'a AppHandle,
    label: &str,
    configure: F,
) -> tauri::Result<WebviewWindow>
where
    F: FnOnce(
        WebviewWindowBuilder<'a, tauri::Wry, AppHandle>,
    ) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle>,
{
    let _guard = CREATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(window) = app.get_webview_window(label) {
        // Restore a minimised window before focusing it.
        let _ = window.unminimize();
        window.show()?;
        window.set_focus()?;
        return Ok(window);
    }

    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .center()
        .focused(true);
    configure(builder).build()
}
