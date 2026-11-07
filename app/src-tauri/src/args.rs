//! Command-line arguments of the desktop app.
//!
//! ```text
//! ConvertApp.exe                         open the main window
//! ConvertApp.exe --to jpg a.png b.png    quick-convert (used by the Explorer menu)
//! ConvertApp.exe --register              add the Explorer context menu, then exit
//! ConvertApp.exe --unregister            remove it, then exit
//! ```
//!
//! The same parser handles arguments forwarded from a second instance by the
//! single-instance plugin, which is how one Explorer click per selected file
//! ends up as a single batch.

use std::path::{Path, PathBuf};

use clap::Parser;
use convertapp_core::Format;

/// Raw arguments as declared to clap.
#[derive(Debug, Parser)]
#[command(name = "ConvertApp", version, about = "Convert files between formats")]
struct LaunchArgs {
    /// Convert FILES to this format in a small progress window.
    #[arg(long, value_name = "FORMAT", requires = "files")]
    to: Option<Format>,

    /// Add "Convert to" to the Explorer context menu and exit.
    #[arg(long, conflicts_with_all = ["unregister", "to"])]
    register: bool,

    /// Remove the Explorer context menu entry and exit.
    #[arg(long, conflicts_with = "to")]
    unregister: bool,

    /// Files to convert (with --to).
    files: Vec<PathBuf>,
}

/// What the app should do for a given command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// Show (or focus) the main window.
    Window,
    /// Convert files without the main window.
    Quick(QuickRequest),
    /// Register the context menu and exit.
    RegisterMenu,
    /// Unregister the context menu and exit.
    UnregisterMenu,
}

/// Files to convert to one target, as received from one process launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickRequest {
    /// Target format.
    pub target: Format,
    /// Absolute input paths.
    pub files: Vec<PathBuf>,
}

/// Parses `argv` (including the program name). Relative file paths are
/// resolved against `cwd`, the working directory of the launching process.
///
/// # Errors
/// Invalid arguments; also used for `--help` / `--version`, whose text is in
/// the error.
pub fn parse<I, T>(argv: I, cwd: &Path) -> Result<Launch, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = LaunchArgs::try_parse_from(argv)?;

    Ok(if args.register {
        Launch::RegisterMenu
    } else if args.unregister {
        Launch::UnregisterMenu
    } else if let Some(target) = args.to {
        Launch::Quick(QuickRequest {
            target,
            files: args.files.into_iter().map(|f| cwd.join(f)).collect(),
        })
    } else {
        Launch::Window
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(args: &[&str]) -> Launch {
        let argv = std::iter::once("ConvertApp.exe").chain(args.iter().copied());
        parse(argv, Path::new(r"C:\work")).unwrap()
    }

    #[test]
    fn no_arguments_opens_the_window() {
        assert_eq!(parse_ok(&[]), Launch::Window);
    }

    #[test]
    fn to_with_files_is_a_quick_request_with_absolute_paths() {
        let launch = parse_ok(&["--to", "jpg", "a.png", r"D:\pics\b.png"]);
        assert_eq!(
            launch,
            Launch::Quick(QuickRequest {
                target: Format::Jpeg,
                files: vec![
                    PathBuf::from(r"C:\work\a.png"),
                    PathBuf::from(r"D:\pics\b.png")
                ],
            })
        );
    }

    #[test]
    fn register_flags() {
        assert_eq!(parse_ok(&["--register"]), Launch::RegisterMenu);
        assert_eq!(parse_ok(&["--unregister"]), Launch::UnregisterMenu);
    }

    #[test]
    fn invalid_combinations_are_rejected() {
        let cwd = Path::new(".");
        assert!(
            parse(["x", "--to", "jpg"], cwd).is_err(),
            "--to needs files"
        );
        assert!(parse(["x", "--to", "nope", "a.png"], cwd).is_err());
        assert!(parse(["x", "--register", "--unregister"], cwd).is_err());
    }
}
