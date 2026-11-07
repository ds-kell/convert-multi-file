//! # convertapp-shell
//!
//! Integrates ConvertApp with the operating system's file manager.
//!
//! On Windows it adds a cascading **"Convert to ▸"** entry to the Explorer
//! context menu of every supported file type. Each sub-item launches
//! `ConvertApp.exe --to <format> "<file>"`.
//!
//! ## How it is stored (Windows)
//!
//! Everything lives under `HKEY_CURRENT_USER\Software\Classes`, so no
//! administrator rights are needed and other users are unaffected:
//!
//! ```text
//! SystemFileAssociations\.png\shell\ConvertApp      MUIVerb="Convert to", Icon, ExtendedSubCommandsKey
//! ConvertApp.Menu.png\shell\01_jpg                  MUIVerb="JPEG (.jpg)"
//! ConvertApp.Menu.png\shell\01_jpg\command          (default)="…\ConvertApp.exe" --to jpg "%1"
//! ```
//!
//! `SystemFileAssociations` attaches the verb to the extension regardless of
//! which program currently opens that file type, and
//! `ExtendedSubCommandsKey` points at the key holding the sub-menu.
//!
//! The layout is computed by the platform-independent [`ContextMenu::plan`],
//! which is unit-tested; [`ContextMenu::register`] then writes it.

use std::io;
use std::path::{Path, PathBuf};

use convertapp_core::{Format, Registry};

#[cfg(windows)]
mod windows;

/// Name of the verb key under `shell\`, also used as the menu-key prefix.
const VERB: &str = "ConvertApp";

/// Prefix of the keys holding each format's sub-menu.
const MENU_PREFIX: &str = "ConvertApp.Menu.";

/// Default text of the cascading menu entry.
const DEFAULT_MENU_TITLE: &str = "Convert to";

/// Errors from reading or writing the shell integration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShellError {
    /// The current operating system has no implementation yet.
    #[error("context-menu integration is not supported on this platform yet")]
    Unsupported,
    /// A registry operation failed.
    #[error("registry error on '{path}'")]
    Registry {
        /// Key involved, relative to `HKEY_CURRENT_USER`.
        path: String,
        /// Underlying OS error.
        #[source]
        source: io::Error,
    },
}

/// One registry key to create, with the string values to set on it.
///
/// An empty value name means the key's `(Default)` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySpec {
    /// Path relative to `HKEY_CURRENT_USER`.
    pub path: String,
    /// `(name, data)` pairs, all `REG_SZ`.
    pub values: Vec<(String, String)>,
}

/// Explorer context-menu integration for one executable.
#[derive(Debug, Clone)]
pub struct ContextMenu {
    exe: PathBuf,
    classes_root: String,
    title: String,
}

impl ContextMenu {
    /// Per-user class registrations, merged by Windows into `HKEY_CLASSES_ROOT`.
    pub const DEFAULT_ROOT: &'static str = r"Software\Classes";

    /// Integration that launches `exe` (normally `std::env::current_exe()`).
    pub fn new(exe: impl Into<PathBuf>) -> Self {
        Self {
            exe: exe.into(),
            classes_root: Self::DEFAULT_ROOT.to_owned(),
            title: DEFAULT_MENU_TITLE.to_owned(),
        }
    }

    /// Text of the cascading entry, in the user's language (default
    /// "Convert to"). Sub-items are format names and need no translation.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Uses another root key instead of `Software\Classes`.
    ///
    /// Only useful in tests: Explorer reads the default root, so keys written
    /// elsewhere are inert.
    #[must_use]
    pub fn with_root(mut self, classes_root: impl Into<String>) -> Self {
        self.classes_root = classes_root.into();
        self
    }

    /// The executable the menu entries launch.
    pub fn exe(&self) -> &Path {
        &self.exe
    }

    /// Computes every key and value [`register`](Self::register) will write,
    /// for all formats that `registry` can convert.
    ///
    /// Each extension alias (`.jpg`, `.jpeg`, …) gets its own verb, but all
    /// aliases of a format share one sub-menu key.
    pub fn plan(&self, registry: &Registry) -> Vec<KeySpec> {
        let exe = self.exe.display().to_string();
        let mut keys = Vec::new();

        for &format in Format::ALL {
            let targets = registry.targets_for(format);
            if targets.is_empty() {
                continue;
            }
            let menu_key = format!("{MENU_PREFIX}{}", format.extension());

            // The cascading entry, attached to every extension of the format.
            for ext in format.extensions() {
                keys.push(KeySpec {
                    path: self.path(&format!(r"SystemFileAssociations\.{ext}\shell\{VERB}")),
                    values: vec![
                        value("MUIVerb", &self.title),
                        value("Icon", &format!("\"{exe}\",0")),
                        value("ExtendedSubCommandsKey", &menu_key),
                        // Keep the entry visible when more than 15 files are selected.
                        value("MultiSelectModel", "Player"),
                    ],
                });
            }

            // One sub-item per target. The numeric prefix fixes the order,
            // because Explorer sorts sub-verbs by key name.
            for (index, target) in targets.iter().enumerate() {
                let verb = format!(r"{menu_key}\shell\{:02}_{}", index + 1, target.extension());
                keys.push(KeySpec {
                    path: self.path(&verb),
                    values: vec![
                        value(
                            "MUIVerb",
                            &format!("{} (.{})", target.display_name(), target.extension()),
                        ),
                        value("MultiSelectModel", "Player"),
                    ],
                });
                keys.push(KeySpec {
                    path: self.path(&format!(r"{verb}\command")),
                    values: vec![value(
                        "",
                        &format!("\"{exe}\" --to {} \"%1\"", target.extension()),
                    )],
                });
            }
        }
        keys
    }

    /// Writes the context menu for every convertible format, replacing any
    /// previous registration. Returns the number of keys written.
    ///
    /// # Errors
    /// [`ShellError::Registry`] if a key cannot be written, or
    /// [`ShellError::Unsupported`] on non-Windows platforms.
    pub fn register(&self, registry: &Registry) -> Result<usize, ShellError> {
        let plan = self.plan(registry);
        #[cfg(windows)]
        {
            windows::remove(&self.classes_root)?;
            windows::write(&plan)?;
            self.notify_explorer();
            Ok(plan.len())
        }
        #[cfg(not(windows))]
        {
            let _ = plan;
            Err(ShellError::Unsupported)
        }
    }

    /// Removes everything [`register`](Self::register) wrote. Succeeds if
    /// nothing was registered.
    ///
    /// # Errors
    /// See [`register`](Self::register).
    pub fn unregister(&self) -> Result<(), ShellError> {
        #[cfg(windows)]
        {
            windows::remove(&self.classes_root)?;
            self.notify_explorer();
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(ShellError::Unsupported)
        }
    }

    /// `true` if a context menu is currently registered (by any executable).
    ///
    /// # Errors
    /// See [`register`](Self::register).
    pub fn is_registered(&self) -> Result<bool, ShellError> {
        #[cfg(windows)]
        {
            windows::any_menu_key(&self.classes_root)
        }
        #[cfg(not(windows))]
        {
            Err(ShellError::Unsupported)
        }
    }

    /// Asks Explorer to reload file associations so the change shows up
    /// without signing out. Skipped for test roots, which Explorer ignores.
    #[cfg(windows)]
    fn notify_explorer(&self) {
        if self.classes_root == Self::DEFAULT_ROOT {
            windows::notify_association_changed();
        }
    }

    /// Joins `relative` onto the classes root.
    fn path(&self, relative: &str) -> String {
        format!(r"{}\{relative}", self.classes_root)
    }
}

/// Builds a `(name, data)` pair.
fn value(name: &str, data: &str) -> (String, String) {
    (name.to_owned(), data.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> Vec<KeySpec> {
        ContextMenu::new(r"C:\Apps\ConvertApp.exe").plan(&Registry::with_defaults())
    }

    fn find<'a>(plan: &'a [KeySpec], path: &str) -> &'a KeySpec {
        plan.iter()
            .find(|k| k.path == path)
            .unwrap_or_else(|| panic!("missing key {path}"))
    }

    fn get<'a>(key: &'a KeySpec, name: &str) -> &'a str {
        key.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .unwrap_or_else(|| panic!("missing value {name} on {}", key.path))
    }

    #[test]
    fn every_extension_alias_gets_the_cascading_entry() {
        let plan = plan();
        for ext in ["png", "jpg", "jpeg", "md", "markdown", "html", "htm"] {
            let key = find(
                &plan,
                &format!(r"Software\Classes\SystemFileAssociations\.{ext}\shell\ConvertApp"),
            );
            assert_eq!(get(key, "MUIVerb"), "Convert to");
            assert_eq!(get(key, "Icon"), r#""C:\Apps\ConvertApp.exe",0"#);
        }
    }

    #[test]
    fn aliases_share_one_submenu() {
        let plan = plan();
        let jpg = find(
            &plan,
            r"Software\Classes\SystemFileAssociations\.jpg\shell\ConvertApp",
        );
        let jpeg = find(
            &plan,
            r"Software\Classes\SystemFileAssociations\.jpeg\shell\ConvertApp",
        );
        assert_eq!(get(jpg, "ExtendedSubCommandsKey"), "ConvertApp.Menu.jpg");
        assert_eq!(get(jpeg, "ExtendedSubCommandsKey"), "ConvertApp.Menu.jpg");
    }

    #[test]
    fn submenu_items_launch_the_exe_with_the_target() {
        // PNG's targets are all built in, so the order does not depend on
        // which external tools are installed (unlike Markdown's).
        let plan = plan();
        let item = find(&plan, r"Software\Classes\ConvertApp.Menu.png\shell\01_jpg");
        assert_eq!(get(item, "MUIVerb"), "JPEG (.jpg)");

        let command = find(
            &plan,
            r"Software\Classes\ConvertApp.Menu.png\shell\01_jpg\command",
        );
        assert_eq!(
            get(command, ""),
            r#""C:\Apps\ConvertApp.exe" --to jpg "%1""#
        );
    }

    #[test]
    fn submenu_never_offers_the_source_format() {
        let plan = plan();
        assert!(
            !plan
                .iter()
                .any(|k| k.path.contains(r"ConvertApp.Menu.png\shell\") && k.path.ends_with("_png"))
        );
    }

    #[test]
    fn title_can_be_localised() {
        let plan = ContextMenu::new("app.exe")
            .with_title("Chuyển sang")
            .plan(&Registry::with_defaults());
        let entry = find(
            &plan,
            r"Software\Classes\SystemFileAssociations\.png\shell\ConvertApp",
        );
        assert_eq!(get(entry, "MUIVerb"), "Chuyển sang");
    }

    #[test]
    fn custom_root_is_used_for_every_key() {
        let plan = ContextMenu::new("app.exe")
            .with_root(r"Software\Test\Classes")
            .plan(&Registry::with_defaults());
        assert!(
            plan.iter()
                .all(|k| k.path.starts_with(r"Software\Test\Classes\"))
        );
    }
}
