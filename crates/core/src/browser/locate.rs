//! Finding a Chromium-based browser on this machine.

use std::path::PathBuf;

use crate::{Availability, Requirement};

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

/// [`Availability`] of web rendering, with what to install when missing.
pub fn availability() -> Availability {
    match locate() {
        Some(_) => Availability::Available,
        None => Availability::missing(vec![requirement()]),
    }
}

/// The browser suggested when none is found: Edge on Windows (where it is
/// normally part of the system), Chrome elsewhere. Either one works, and
/// [`ENV_OVERRIDE`] can point at any Chromium-based browser.
fn requirement() -> Requirement {
    if cfg!(windows) {
        Requirement {
            name: "Microsoft Edge",
            url: "https://www.microsoft.com/edge/download",
            install_command: Some("winget install Microsoft.Edge"),
            free: true,
        }
    } else {
        Requirement {
            name: "Google Chrome",
            url: "https://www.google.com/chrome/",
            // Linux distributions do not package Chrome; the download page
            // offers the right .deb/.rpm.
            install_command: cfg!(target_os = "macos")
                .then_some("brew install --cask google-chrome"),
            free: true,
        }
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
    const BUNDLES: [&str; 3] = [
        "Google Chrome.app/Contents/MacOS/Google Chrome",
        "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "Chromium.app/Contents/MacOS/Chromium",
    ];
    // System-wide /Applications first, then the user's own ~/Applications.
    let roots: Vec<PathBuf> = std::iter::once(PathBuf::from("/Applications"))
        .chain(std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Applications")))
        .collect();
    roots
        .iter()
        .flat_map(|root| BUNDLES.into_iter().map(move |bundle| root.join(bundle)))
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
    // PATH first, then the usual folders, for desktop launchers started
    // with a reduced PATH.
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend(
        [
            "/usr/bin",
            "/usr/local/bin",
            "/snap/bin",
            "/opt/google/chrome",
        ]
        .into_iter()
        .map(PathBuf::from),
    );
    dirs.iter()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .collect()
}
