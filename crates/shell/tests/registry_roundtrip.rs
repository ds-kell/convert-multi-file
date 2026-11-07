//! Writes the real registry, but under a private test root that Explorer never
//! reads, so running the tests does not change the user's context menu.

#![cfg(windows)]

use convertapp_core::Registry;
use convertapp_shell::ContextMenu;
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

/// Deletes the test root when dropped, even if the test panics.
struct TestRoot {
    base: String,
}

impl TestRoot {
    fn new(name: &str) -> Self {
        Self {
            base: format!(r"Software\ConvertAppTest-{}-{name}", std::process::id()),
        }
    }

    fn classes(&self) -> String {
        format!(r"{}\Classes", self.base)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.base);
    }
}

fn read(path: &str, name: &str) -> std::io::Result<String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(path)?
        .get_value(name)
}

#[test]
fn register_writes_menu_and_unregister_removes_it() {
    let root = TestRoot::new("roundtrip");
    let menu = ContextMenu::new(r"C:\Apps\ConvertApp.exe").with_root(root.classes());
    let registry = Registry::with_defaults();

    assert!(!menu.is_registered().unwrap());

    let written = menu.register(&registry).unwrap();
    assert_eq!(written, menu.plan(&registry).len());
    assert!(menu.is_registered().unwrap());

    let classes = root.classes();
    assert_eq!(
        read(
            &format!(r"{classes}\SystemFileAssociations\.png\shell\ConvertApp"),
            "MUIVerb"
        )
        .unwrap(),
        "Convert to"
    );
    assert_eq!(
        read(
            &format!(r"{classes}\ConvertApp.Menu.png\shell\01_jpg\command"),
            ""
        )
        .unwrap(),
        r#""C:\Apps\ConvertApp.exe" --to jpg "%1""#
    );

    menu.unregister().unwrap();
    assert!(!menu.is_registered().unwrap());
    assert!(
        read(
            &format!(r"{classes}\SystemFileAssociations\.png\shell\ConvertApp"),
            "MUIVerb"
        )
        .is_err()
    );
}

#[test]
fn register_twice_replaces_previous_entries() {
    let root = TestRoot::new("twice");
    let registry = Registry::with_defaults();

    ContextMenu::new(r"C:\Old\ConvertApp.exe")
        .with_root(root.classes())
        .register(&registry)
        .unwrap();
    ContextMenu::new(r"C:\New\ConvertApp.exe")
        .with_root(root.classes())
        .register(&registry)
        .unwrap();

    let command = read(
        &format!(
            r"{}\ConvertApp.Menu.md\shell\01_html\command",
            root.classes()
        ),
        "",
    )
    .unwrap();
    assert!(
        command.starts_with(r#""C:\New\ConvertApp.exe""#),
        "{command}"
    );
}

#[test]
fn unregister_without_registration_is_ok() {
    let root = TestRoot::new("empty");
    ContextMenu::new("x.exe")
        .with_root(root.classes())
        .unregister()
        .unwrap();
}
