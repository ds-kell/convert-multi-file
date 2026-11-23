//! Finding and running external command-line programs (Pandoc, Typst, …).
//!
//! Converters that rely on an external program describe it with a [`Tool`].
//! The program is looked up, in order:
//! 1. an environment variable override (e.g. `CONVERTAPP_PANDOC=D:\pandoc.exe`),
//! 2. the `PATH`,
//! 3. well-known install folders of the current operating system (per-user
//!    installers often skip `PATH`; on macOS an app started from Finder or the
//!    Dock gets a minimal `PATH` without Homebrew's `/opt/homebrew/bin`).
//!
//! Lookup only touches the file system (no process is started), so it is cheap
//! enough to repeat whenever the UI asks which formats are available. That
//! also means a tool installed while the app is running is picked up at once.
//!
//! Every platform-dependent value (folders, install commands) is chosen with
//! `cfg!`, so each build only carries what applies to its operating system.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use crate::{Availability, ConvertError, Requirement, Result};

/// `true` when compiled for macOS.
const MACOS: bool = cfg!(target_os = "macos");

/// Where Homebrew links programs: Apple Silicon, then Intel Macs (also used
/// by most `.pkg` installers, e.g. Pandoc's).
const MACOS_BIN_DIRS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin"];

/// Program folders on Linux, for desktop launchers started with a reduced
/// `PATH` (Snap links its commands into `/snap/bin`).
const LINUX_BIN_DIRS: &[&str] = &["/usr/local/bin", "/usr/bin", "/snap/bin"];

/// An external program a converter depends on.
#[derive(Debug, Clone, Copy)]
pub struct Tool {
    /// Display name, used in messages ("Pandoc").
    pub name: &'static str,
    /// Executable file name without extension ("pandoc").
    pub program: &'static str,
    /// Environment variable that may point at the executable.
    pub env_override: &'static str,
    /// Folders to search after `PATH`, as `(environment variable, sub-path)`
    /// pairs, e.g. `("LOCALAPPDATA", "Pandoc")` or `("HOME", ".cargo/bin")`.
    pub known_dirs: &'static [(&'static str, &'static str)],
    /// Absolute folders to search after [`known_dirs`](Self::known_dirs),
    /// e.g. `/opt/homebrew/bin`.
    pub system_dirs: &'static [&'static str],
    /// Windows `App Paths` registry entry written by the installer (e.g.
    /// `soffice.exe`), checked last. Finds programs installed to custom folders.
    pub app_path: Option<&'static str>,
    /// Official download page, shown when the tool is missing.
    pub homepage: &'static str,
    /// How to install the tool on each operating system.
    pub install: InstallCommands,
}

/// One-line install commands for a program, per operating system.
///
/// Linux has one entry per package manager because distributions differ;
/// the one present on the machine is picked at run time. `None` means there
/// is no reliable command, and the UI offers the download page instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallCommands {
    /// `winget` (built into Windows 10/11).
    pub windows: Option<&'static str>,
    /// [Homebrew](https://brew.sh).
    pub macos: Option<&'static str>,
    /// Debian, Ubuntu, Linux Mint…
    pub apt: Option<&'static str>,
    /// Fedora, RHEL…
    pub dnf: Option<&'static str>,
    /// Arch, Manjaro…
    pub pacman: Option<&'static str>,
    /// openSUSE.
    pub zypper: Option<&'static str>,
}

impl InstallCommands {
    /// No command on any system.
    pub const NONE: Self = Self {
        windows: None,
        macos: None,
        apt: None,
        dnf: None,
        pacman: None,
        zypper: None,
    };

    /// The command for the operating system (and, on Linux, the package
    /// manager) this program runs on.
    pub fn for_this_os(&self) -> Option<&'static str> {
        if cfg!(windows) {
            self.windows
        } else if MACOS {
            self.macos
        } else {
            match linux_package_manager()? {
                PackageManager::Apt => self.apt,
                PackageManager::Dnf => self.dnf,
                PackageManager::Pacman => self.pacman,
                PackageManager::Zypper => self.zypper,
            }
        }
    }
}

/// Linux package managers [`InstallCommands`] knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

/// The package manager installed on this machine, detected once by looking
/// for its executable.
fn linux_package_manager() -> Option<PackageManager> {
    static DETECTED: OnceLock<Option<PackageManager>> = OnceLock::new();
    *DETECTED.get_or_init(|| {
        [
            ("apt-get", PackageManager::Apt),
            ("dnf", PackageManager::Dnf),
            ("pacman", PackageManager::Pacman),
            ("zypper", PackageManager::Zypper),
        ]
        .into_iter()
        .find(|(program, _)| {
            ["/usr/bin", "/bin", "/usr/sbin"]
                .iter()
                .any(|dir| Path::new(dir).join(program).is_file())
        })
        .map(|(_, manager)| manager)
    })
}

/// [Pandoc](https://pandoc.org): universal document converter.
pub const PANDOC: Tool = Tool {
    name: "Pandoc",
    program: "pandoc",
    env_override: "CONVERTAPP_PANDOC",
    known_dirs: if cfg!(windows) {
        &[("LOCALAPPDATA", "Pandoc"), ("ProgramFiles", "Pandoc")]
    } else {
        &[("HOME", ".local/bin")]
    },
    system_dirs: if cfg!(windows) {
        &[]
    } else if MACOS {
        MACOS_BIN_DIRS
    } else {
        LINUX_BIN_DIRS
    },
    app_path: None,
    homepage: "https://pandoc.org/installing.html",
    install: InstallCommands {
        windows: Some("winget install JohnMacFarlane.Pandoc"),
        macos: Some("brew install pandoc"),
        apt: Some("sudo apt install pandoc"),
        dnf: Some("sudo dnf install pandoc"),
        pacman: Some("sudo pacman -S pandoc"),
        zypper: Some("sudo zypper install pandoc"),
    },
};

/// [LibreOffice](https://www.libreoffice.org): office suite used headless
/// to convert Word/Excel/PowerPoint and OpenDocument files.
pub const LIBREOFFICE: Tool = Tool {
    name: "LibreOffice",
    program: "soffice",
    env_override: "CONVERTAPP_SOFFICE",
    known_dirs: if cfg!(windows) {
        &[
            ("ProgramFiles", r"LibreOffice\program"),
            ("ProgramFiles(x86)", r"LibreOffice\program"),
        ]
    } else if MACOS {
        // Dragged into ~/Applications instead of /Applications.
        &[("HOME", "Applications/LibreOffice.app/Contents/MacOS")]
    } else {
        &[]
    },
    system_dirs: if cfg!(windows) {
        &[]
    } else if MACOS {
        // The .dmg and `brew install --cask` both install the app bundle,
        // which never puts `soffice` on the PATH.
        &[
            "/Applications/LibreOffice.app/Contents/MacOS",
            "/opt/homebrew/bin",
            "/usr/local/bin",
        ]
    } else {
        &[
            "/usr/bin",
            "/usr/local/bin",
            "/usr/lib/libreoffice/program",
            "/opt/libreoffice/program",
        ]
    },
    app_path: Some("soffice.exe"),
    homepage: "https://www.libreoffice.org/download/download-libreoffice/",
    install: InstallCommands {
        windows: Some("winget install TheDocumentFoundation.LibreOffice"),
        macos: Some("brew install --cask libreoffice"),
        apt: Some("sudo apt install libreoffice"),
        dnf: Some("sudo dnf install libreoffice"),
        pacman: Some("sudo pacman -S libreoffice-fresh"),
        zypper: Some("sudo zypper install libreoffice"),
    },
};

/// [Typst](https://typst.app): typesetting engine Pandoc uses to make PDFs.
pub const TYPST: Tool = Tool {
    name: "Typst",
    program: "typst",
    env_override: "CONVERTAPP_TYPST",
    known_dirs: if cfg!(windows) {
        &[("LOCALAPPDATA", r"Microsoft\WinGet\Links")]
    } else {
        // `cargo install typst-cli`, or the release archive unpacked by hand.
        &[("HOME", ".cargo/bin"), ("HOME", ".local/bin")]
    },
    system_dirs: if cfg!(windows) {
        &[]
    } else if MACOS {
        MACOS_BIN_DIRS
    } else {
        LINUX_BIN_DIRS
    },
    app_path: None,
    homepage: "https://github.com/typst/typst/releases/latest",
    install: InstallCommands {
        windows: Some("winget install Typst.Typst"),
        macos: Some("brew install typst"),
        // Not packaged by Debian/Ubuntu stable: the download page is shown.
        apt: None,
        dnf: Some("sudo dnf install typst"),
        pacman: Some("sudo pacman -S typst"),
        zypper: Some("sudo zypper install typst"),
    },
};

impl Tool {
    /// Full path of the executable, or `None` if it cannot be found.
    pub fn locate(&self) -> Option<PathBuf> {
        if let Some(path) = std::env::var_os(self.env_override).map(PathBuf::from) {
            // An explicit override is authoritative, even if it is wrong, so
            // the user sees why their setting does not work.
            return path.is_file().then_some(path);
        }

        let file_name = executable_name(self.program);
        let path_dirs = std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
            .unwrap_or_default();
        let known_dirs = self.known_dirs.iter().filter_map(|(var, sub)| {
            std::env::var_os(var).map(|base| PathBuf::from(base).join(sub))
        });
        let system_dirs = self.system_dirs.iter().map(PathBuf::from);

        path_dirs
            .into_iter()
            .chain(known_dirs)
            .chain(system_dirs)
            .map(|dir| dir.join(&file_name))
            .find(|candidate| candidate.is_file())
            .or_else(|| self.app_path.and_then(registered_app_path))
    }

    /// What the user must install when this tool is missing, with the
    /// install command for this operating system.
    pub fn requirement(&self) -> Requirement {
        Requirement {
            name: self.name,
            url: self.homepage,
            install_command: self.install.for_this_os(),
            free: true,
        }
    }

    /// [`Availability::Available`] if the tool can be found, otherwise
    /// unavailable with this tool as the missing requirement.
    pub fn availability(&self) -> Availability {
        match self.locate() {
            Some(_) => Availability::Available,
            None => Availability::missing(vec![self.requirement()]),
        }
    }

    /// Like [`locate`](Self::locate), but as a conversion error.
    pub(crate) fn require(&self) -> Result<PathBuf> {
        self.locate().ok_or_else(|| ConvertError::External {
            tool: self.name,
            message: self.missing_reason(),
        })
    }

    fn missing_reason(&self) -> String {
        match self.install.for_this_os() {
            Some(command) => format!("{} is not installed. Install it with: {command}", self.name),
            None => format!(
                "{} is not installed. Download it from {}",
                self.name, self.homepage
            ),
        }
    }
}

/// Looks up `exe` (e.g. `soffice.exe`) under the Windows "App Paths"
/// registry key, where installers record custom install locations.
#[cfg(windows)]
fn registered_app_path(exe: &str) -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER]
        .into_iter()
        .filter_map(|hive| {
            RegKey::predef(hive)
                .open_subkey(format!(
                    r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}"
                ))
                .and_then(|key| key.get_value::<String, _>(""))
                .ok()
        })
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

#[cfg(not(windows))]
fn registered_app_path(_exe: &str) -> Option<PathBuf> {
    None
}

/// `pandoc` → `pandoc.exe` on Windows.
fn executable_name(program: &str) -> String {
    if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_owned()
    }
}

/// Runs `exe` with `args` in `cwd` and waits for it, up to `timeout`.
///
/// - No console window flashes up on Windows.
/// - Standard output is discarded; standard error is collected and returned
///   in the error message when the program fails.
/// - A program still running after `timeout` is killed.
///
/// # Errors
/// [`ConvertError::External`] if the program cannot start, exits with an
/// error, or times out.
pub(crate) fn run<I, S>(
    tool: &Tool,
    exe: &Path,
    args: I,
    cwd: &Path,
    timeout: Duration,
) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let failure = |message: String| ConvertError::External {
        tool: tool.name,
        message,
    };

    let mut command = Command::new(exe);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console_window(&mut command);

    tracing::debug!(?command, "running external tool");
    let mut child = command
        .spawn()
        .map_err(|e| failure(format!("cannot start {}: {e}", exe.display())))?;

    // Drain stderr on a thread: a chatty program would otherwise block once
    // the pipe buffer is full, and we would wrongly report a timeout.
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let reader = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                kill_tree(&mut child);
                return Err(failure(format!(
                    "did not finish within {} seconds",
                    timeout.as_secs()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(failure(e.to_string())),
        }
    };

    let stderr = reader.join().unwrap_or_default();
    if status.success() {
        if !stderr.trim().is_empty() {
            tracing::debug!(tool = tool.name, warnings = %stderr.trim(), "tool warnings");
        }
        Ok(())
    } else {
        let detail = stderr.trim();
        Err(failure(if detail.is_empty() {
            format!("exited with {status}")
        } else {
            detail.to_owned()
        }))
    }
}

/// Kills `child` and every process it started. Launchers such as
/// `soffice.com` hand the work to a child process (`soffice.bin`) that would
/// otherwise keep running and keep our stderr pipe open.
fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let mut taskkill = Command::new("taskkill");
        taskkill.args(["/T", "/F", "/PID", &child.id().to_string()]);
        hide_console_window(&mut taskkill);
        let _ = taskkill.output();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Prevents a console window from flashing up for `command` (Windows only).
#[cfg(windows)]
pub(crate) fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW` process creation flag.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_console_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: Tool = Tool {
        name: "Nope",
        program: "definitely-not-installed-convertapp",
        env_override: "CONVERTAPP_TEST_NOPE",
        known_dirs: &[],
        system_dirs: &[],
        app_path: None,
        homepage: "https://example.com/nope",
        install: InstallCommands {
            windows: Some("install nope"),
            macos: Some("install nope"),
            apt: Some("install nope"),
            dnf: Some("install nope"),
            pacman: Some("install nope"),
            zypper: Some("install nope"),
        },
    };

    #[test]
    fn missing_tool_reports_install_hint() {
        assert!(MISSING.locate().is_none());
        match MISSING.availability() {
            Availability::Unavailable { reason, missing } => {
                assert_eq!(missing, [MISSING.requirement()]);
                assert!(reason.contains("Nope is not installed"));
                // A Linux machine without a known package manager gets the
                // download page instead of a command.
                let hint = MISSING.install.for_this_os().unwrap_or(MISSING.homepage);
                assert!(reason.contains(hint));
            }
            Availability::Available => panic!("should be missing"),
        }
        assert!(matches!(
            MISSING.require(),
            Err(ConvertError::External { tool: "Nope", .. })
        ));
    }

    #[test]
    fn install_commands_match_this_os() {
        for tool in [PANDOC, LIBREOFFICE, TYPST] {
            let command = tool.install.for_this_os();
            if cfg!(windows) {
                assert!(
                    command.is_some_and(|c| c.starts_with("winget ")),
                    "{command:?}"
                );
            } else if MACOS {
                assert!(
                    command.is_some_and(|c| c.starts_with("brew ")),
                    "{command:?}"
                );
            } else if let Some(command) = command {
                assert!(command.starts_with("sudo "), "{command}");
                assert!(!command.contains("winget") && !command.contains("brew"));
            }
        }
    }

    #[test]
    fn search_folders_belong_to_this_os() {
        for tool in [PANDOC, LIBREOFFICE, TYPST] {
            for (var, sub) in tool.known_dirs {
                assert_eq!(*var == "HOME", !cfg!(windows), "{var}");
                if !cfg!(windows) {
                    assert!(!sub.contains('\\'), "{sub}");
                }
            }
            assert!(tool.system_dirs.iter().all(|dir| dir.starts_with('/')));
            assert_eq!(tool.system_dirs.is_empty(), cfg!(windows));
        }
    }

    #[cfg(windows)]
    #[test]
    fn run_reports_failures_and_success() {
        let cmd = PathBuf::from(std::env::var_os("ComSpec").expect("ComSpec is set on Windows"));
        let dir = std::env::temp_dir();
        let timeout = Duration::from_secs(10);

        run(&MISSING, &cmd, ["/C", "exit 0"], &dir, timeout).unwrap();

        let err = run(
            &MISSING,
            &cmd,
            ["/C", "echo boom 1>&2 & exit 3"],
            &dir,
            timeout,
        )
        .unwrap_err();
        assert!(err.to_string().contains("boom"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn run_kills_programs_that_hang() {
        let powershell =
            PathBuf::from(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
        if !powershell.is_file() {
            return;
        }
        let started = Instant::now();
        let err = run(
            &MISSING,
            &powershell,
            ["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            &std::env::temp_dir(),
            Duration::from_millis(500),
        )
        .unwrap_err();
        assert!(err.to_string().contains("did not finish"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
