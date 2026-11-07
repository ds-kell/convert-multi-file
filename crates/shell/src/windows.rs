//! Windows registry backend for [`crate::ContextMenu`].

use std::io;

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use crate::{KeySpec, MENU_PREFIX, ShellError, VERB};

/// Creates every key in `plan` and sets its values.
pub(crate) fn write(plan: &[KeySpec]) -> Result<(), ShellError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for spec in plan {
        let (key, _) = hkcu
            .create_subkey(&spec.path)
            .map_err(|e| registry_error(&spec.path, e))?;
        for (name, data) in &spec.values {
            key.set_value(name, data)
                .map_err(|e| registry_error(&spec.path, e))?;
        }
    }
    Ok(())
}

/// Deletes the verb from every extension and all sub-menu keys under
/// `classes_root`. Keys that do not exist are ignored.
pub(crate) fn remove(classes_root: &str) -> Result<(), ShellError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Some(root) = open_optional(&hkcu, classes_root)? else {
        return Ok(());
    };

    // 1. `SystemFileAssociations\.<ext>\shell\ConvertApp`
    let associations_path = format!(r"{classes_root}\SystemFileAssociations");
    if let Some(associations) = open_optional(&hkcu, &associations_path)? {
        // Collect first: deleting while enumerating would skip entries.
        let extensions: Vec<String> = associations.enum_keys().filter_map(Result::ok).collect();
        for ext in extensions {
            delete_tree(&associations, &format!(r"{ext}\shell\{VERB}"))?;
        }
    }

    // 2. `ConvertApp.Menu.<format>`
    for name in menu_keys(&root) {
        delete_tree(&root, &name)?;
    }
    Ok(())
}

/// `true` if at least one `ConvertApp.Menu.*` key exists under `classes_root`.
pub(crate) fn any_menu_key(classes_root: &str) -> Result<bool, ShellError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    Ok(open_optional(&hkcu, classes_root)?.is_some_and(|root| !menu_keys(&root).is_empty()))
}

/// Tells Explorer that file associations changed, so the new menu shows up
/// without restarting Explorer or signing out.
pub(crate) fn notify_association_changed() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

    // SAFETY: documented usage for SHCNE_ASSOCCHANGED: both item pointers
    // must be null and are never dereferenced. The event id fits in an i32.
    #[allow(unsafe_code)]
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

/// Names of the sub-menu keys directly under `root`.
fn menu_keys(root: &RegKey) -> Vec<String> {
    root.enum_keys()
        .filter_map(Result::ok)
        .filter(|name| name.starts_with(MENU_PREFIX))
        .collect()
}

/// Opens `path`, returning `None` if it does not exist.
fn open_optional(parent: &RegKey, path: &str) -> Result<Option<RegKey>, ShellError> {
    match parent.open_subkey(path) {
        Ok(key) => Ok(Some(key)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(registry_error(path, e)),
    }
}

/// Deletes `path` and everything below it; a missing key is not an error.
fn delete_tree(parent: &RegKey, path: &str) -> Result<(), ShellError> {
    match parent.delete_subkey_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(registry_error(path, e)),
    }
}

fn registry_error(path: &str, source: io::Error) -> ShellError {
    ShellError::Registry {
        path: path.to_owned(),
        source,
    }
}
