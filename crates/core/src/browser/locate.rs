//! Finding a Chromium-based browser on this machine.

use std::path::PathBuf;

use crate::Availability;

/// Environment variable that may point at the browser executable.
pub const ENV_OVERRIDE: &str = "CONVERTAPP_BROWSER";

/// Full path of Edge or Chrome, or `None` if neither is installed.
///
/// Order: [`ENV_OVERRIDE`], then the platform's usual locations. On Windows
/// Edge comes first because it is part of the OS and always present.
pub fn locate() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(ENV_OVERRIDE).map(PathBuf::from) {
        // An explicit setting is authoritative, even if wrong, so the user
        // sees why it does not work.
        return path.is_file().then_some(path);
    }
    candidates().into_iter().find(|path| path.is_file())
}

/// [`Availability`] of web rendering, with an explanation when missing.
pub fn availability() -> Availability {
    match locate() {
        Some(_) => Availability::Available,
        None => Availability::Unavailable {
            reason: format!(
                "Microsoft Edge or Google Chrome is required to render web pages. \
                 Install one, or set {ENV_OVERRIDE} to the browser's executable."
            ),
        },
    }
}

#[cfg(windows)]
fn candidates() -> Vec<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    const APP_PATHS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
    let mut found = Vec::new();

    // 1. "App Paths" registrations, written by the installers.
    for exe in ["msedge.exe", "chrome.exe"] {
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            let path: Option<String> = RegKey::predef(hive)
                .open_subkey(format!(r"{APP_PATHS}\{exe}"))
                .and_then(|key| key.get_value(""))
                .ok();
            found.extend(path.map(PathBuf::from));
        }
    }

    // 2. Default install folders, in case the registration is missing.
    let in_env =
        |var: &str, sub: &str| std::env::var_os(var).map(|base| PathBuf::from(base).join(sub));
    found.extend(
        [
            in_env(
                "ProgramFiles(x86)",
                r"Microsoft\Edge\Application\msedge.exe",
            ),
            in_env("ProgramFiles", r"Microsoft\Edge\Application\msedge.exe"),
            in_env("ProgramFiles", r"Google\Chrome\Application\chrome.exe"),
            in_env("LOCALAPPDATA", r"Google\Chrome\Application\chrome.exe"),
        ]
        .into_iter()
        .flatten(),
    );
    found
}

#[cfg(target_os = "macos")]
fn candidates() -> Vec<PathBuf> {
    [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn candidates() -> Vec<PathBuf> {
    let names = [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "microsoft-edge",
    ];
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.iter()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .collect()
}
